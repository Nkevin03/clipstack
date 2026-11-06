//! Copy sequence and paste peek.
//!
//! **Copy sequence**: press the sequence shortcut (default ⌥⇧⌘C), copy A, B, C, then press ⌘V
//! repeatedly in any app. Order is configurable (last-copied first → C, B, A; or first-copied
//! first → A, B, C) and it can loop forever ("cycle": A, B, C, A, B, C…). Without cycle, each
//! pasted item leaves the sequence and the session ends when it is empty. Items can be removed
//! from the HUD. Clipstack keeps the pasteboard holding the item that the next ⌘V must paste.
//!
//! **Paste peek**: optionally, after every ⌘V a small HUD lists the last N copies for a few
//! seconds; clicking one pastes it instead.
//!
//! ⌘V is detected with a global key monitor (needs the Accessibility permission). Nothing is
//! swapped if the pasteboard changed in a way Clipstack did not record.

use crate::clipboard::pasteboard;
use crate::db::queries as q;
use crate::models::Item;
use crate::state::AppState;
use serde::Serialize;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};
use ts_rs::TS;

#[derive(Debug, PartialEq, Eq)]
pub enum Step {
    /// Put this item on the pasteboard for the next ⌘V.
    Next(i64),
    /// The pasteboard already holds the next item (cycle with a single item).
    Same,
    /// The last item was pasted: the session is over.
    Finished,
    /// Not in a session, or the pasteboard is not what the sequence expects.
    Ignored,
}

#[derive(Debug, Default)]
pub struct PasteStack {
    active: bool,
    /// Item ids in copy order (oldest first).
    ids: Vec<i64>,
    /// true: last copied is pasted first. false: first copied is pasted first.
    last_first: bool,
    /// Loop over the items instead of consuming them.
    cycle: bool,
    /// In cycle mode: the item the next ⌘V pastes.
    cursor: Option<i64>,
    pasted: usize,
    /// Pasteboard change count that corresponds to `current()` (-1 = unknown).
    expected_cc: isize,
}

impl PasteStack {
    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn start(&mut self, last_first: bool, cycle: bool) {
        *self = PasteStack {
            active: true,
            last_first,
            cycle,
            expected_cc: -1,
            ..Default::default()
        };
    }

    pub fn stop(&mut self) {
        *self = PasteStack {
            expected_cc: -1,
            ..Default::default()
        };
    }

    /// Ids in paste order.
    pub fn order(&self) -> Vec<i64> {
        if self.last_first {
            self.ids.iter().rev().copied().collect()
        } else {
            self.ids.clone()
        }
    }

    /// The item the next ⌘V must paste.
    pub fn current(&self) -> Option<i64> {
        if self.cycle
            && let Some(c) = self.cursor.filter(|c| self.ids.contains(c))
        {
            return Some(c);
        }
        self.order().first().copied()
    }

    /// Upcoming pastes, starting with `current()` (rotated in cycle mode).
    pub fn upcoming(&self) -> Vec<i64> {
        let order = self.order();
        match self
            .current()
            .and_then(|c| order.iter().position(|x| *x == c))
        {
            Some(pos) if self.cycle => order[pos..]
                .iter()
                .chain(order[..pos].iter())
                .copied()
                .collect(),
            _ => order,
        }
    }

    /// Returns the item that must be put on the pasteboard if it is not what the
    /// pasteboard holds now (`holding`, with change count `cc`).
    fn reconcile(&mut self, holding: Option<i64>, cc: isize) -> Option<i64> {
        let cur = self.current();
        if self.cycle {
            self.cursor = cur;
        }
        if cur.is_some() && cur == holding {
            self.expected_cc = cc;
            None
        } else {
            cur
        }
    }

    /// A copy was recorded (`id` now on the pasteboard with change count `cc`).
    /// Returns an item to write back if the next paste must be something else.
    pub fn on_capture(&mut self, id: i64, cc: isize) -> Option<i64> {
        if !self.active {
            return None;
        }
        self.ids.retain(|x| *x != id);
        self.ids.push(id);
        // Last-copied-first: a new copy is always the next paste. First-copied-first: the
        // cursor stays where it is (the first copy starts the cycle).
        if self.cycle && (self.last_first || self.cursor.is_none()) {
            self.cursor = Some(id);
        }
        self.reconcile(Some(id), cc)
    }

    /// Change order/cycle during a session. Returns an item to write if needed.
    pub fn configure(&mut self, last_first: bool, cycle: bool) -> Option<i64> {
        if !self.active || (self.last_first == last_first && self.cycle == cycle) {
            return None;
        }
        let holding = self.current().filter(|_| self.expected_cc >= 0);
        self.last_first = last_first;
        self.cycle = cycle;
        self.cursor = None;
        self.reconcile(holding, self.expected_cc)
    }

    /// Remove an item from the sequence (not from history). Returns an item to write if the
    /// removed one was the next to paste.
    pub fn remove(&mut self, id: i64) -> Option<i64> {
        let before = self.current();
        let upcoming = self.upcoming();
        if self.cycle && before == Some(id) {
            // Move the cursor to the item that would have come next.
            self.cursor = upcoming.iter().copied().find(|x| *x != id);
        }
        self.ids.retain(|x| *x != id);
        let after = self.current();
        if after != before { after } else { None }
    }

    /// The pasteboard changed to something Clipstack did not store: don't swap until the next
    /// recorded copy.
    pub fn on_unrecorded(&mut self) {
        self.expected_cc = -1;
    }

    /// The user pasted while the pasteboard had change count `current_cc`.
    pub fn on_paste(&mut self, current_cc: isize) -> Step {
        if !self.active || self.ids.is_empty() || current_cc != self.expected_cc {
            return Step::Ignored;
        }
        self.pasted += 1;
        let cur = self.current();
        if self.cycle {
            let up = self.upcoming();
            let next = up.get(1).or(up.first()).copied();
            self.cursor = next;
            return if next == cur {
                Step::Same
            } else {
                next.map(Step::Next).unwrap_or(Step::Ignored)
            };
        }
        if let Some(c) = cur {
            self.ids.retain(|x| *x != c);
        }
        match self.current() {
            Some(id) => Step::Next(id),
            None => {
                self.stop();
                Step::Finished
            }
        }
    }

    pub fn on_swapped(&mut self, cc: isize) {
        self.expected_cc = cc;
    }
}

#[derive(Serialize, TS, Clone, Debug)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SequenceState {
    pub active: bool,
    /// Upcoming pastes: `items[0]` is pasted by the next ⌘V.
    pub items: Vec<Item>,
    pub pasted: i64,
    /// The session just ended because everything was pasted.
    pub finished: bool,
    pub accessibility: bool,
    pub cycle: bool,
    pub last_first: bool,
}

pub fn lock(state: &AppState) -> std::sync::MutexGuard<'_, PasteStack> {
    state.paste_stack.lock().unwrap_or_else(|e| e.into_inner())
}

pub fn snapshot(state: &AppState, finished: bool) -> SequenceState {
    let (active, ids, pasted, cycle, last_first) = {
        let st = lock(state);
        (st.active, st.upcoming(), st.pasted, st.cycle, st.last_first)
    };
    let db = state.db();
    let items = ids
        .iter()
        .filter_map(|id| q::get_item(&db, *id).ok().flatten())
        .collect();
    SequenceState {
        active,
        items,
        pasted: pasted as i64,
        finished,
        accessibility: crate::permissions::is_trusted(),
        cycle,
        last_first,
    }
}

pub fn emit(app: &AppHandle, finished: bool) {
    let state = app.state::<Arc<AppState>>();
    let snap = snapshot(&state, finished);
    let _ = app.emit("sequence:state", &snap);
}

pub fn is_active(app: &AppHandle) -> bool {
    lock(&app.state::<Arc<AppState>>()).is_active()
}

/// Put `id` on the pasteboard as the item for the next ⌘V.
fn write_item(state: &AppState, id: i64) {
    let row = q::get_full(&state.db(), id).ok().flatten();
    match row.map(|r| crate::paste::build(&[r], false, "\n")) {
        Some(Ok(data)) => {
            let cc = pasteboard::write(&data);
            state.own_change_count.store(cc, Ordering::SeqCst);
            lock(state).on_swapped(cc);
        }
        _ => {
            log::warn!("copy sequence: item #{id} unavailable");
            lock(state).on_unrecorded();
        }
    }
}

/// Called by the clipboard monitor after a capture.
pub fn after_capture(app: &AppHandle, id: i64, cc: isize) {
    let state = app.state::<Arc<AppState>>().inner().clone();
    let (active, write) = {
        let mut st = lock(&state);
        (st.is_active(), st.on_capture(id, cc))
    };
    if let Some(w) = write {
        write_item(&state, w);
    }
    if active {
        emit(app, false);
    }
}

pub fn start_session(app: &AppHandle) {
    let state = app.state::<Arc<AppState>>().inner().clone();
    let s = state.settings();
    lock(&state).start(s.sequence_order != "firstFirst", s.sequence_cycle);
    if !crate::permissions::is_trusted() {
        // Shows the macOS prompt; the HUD explains why ⌘V can't be followed yet.
        crate::permissions::request();
    }
    ensure_monitor(app.clone());
    hide_peek(app);
    crate::hud::show(app);
    emit(app, false);
    crate::tray::refresh(app);
    log::info!("copy sequence started");
}

pub fn stop_session(app: &AppHandle) {
    let state = app.state::<Arc<AppState>>().inner().clone();
    lock(&state).stop();
    emit(app, false);
    crate::hud::hide(app);
    crate::tray::refresh(app);
    log::info!("copy sequence stopped");
}

pub fn toggle_session(app: &AppHandle) {
    if is_active(app) {
        stop_session(app);
    } else {
        start_session(app);
    }
}

/// Settings changed: apply order/cycle to a running session.
pub fn reconfigure(app: &AppHandle) {
    let state = app.state::<Arc<AppState>>().inner().clone();
    let s = state.settings();
    let write = lock(&state).configure(s.sequence_order != "firstFirst", s.sequence_cycle);
    if let Some(w) = write {
        write_item(&state, w);
    }
    if is_active(app) {
        emit(app, false);
    }
    if s.paste_peek {
        ensure_monitor(app.clone());
    }
}

pub fn remove_item(app: &AppHandle, id: i64) {
    let state = app.state::<Arc<AppState>>().inner().clone();
    let write = lock(&state).remove(id);
    if let Some(w) = write {
        write_item(&state, w);
    }
    emit(app, false);
}

/// Called (off the main thread) shortly after the user released ⌘V in another app.
pub fn handle_user_paste(app: &AppHandle) {
    let state = app.state::<Arc<AppState>>().inner().clone();
    if state.panel_visible.load(Ordering::SeqCst) {
        return;
    }
    let step = lock(&state).on_paste(pasteboard::change_count());
    match step {
        Step::Ignored => {}
        Step::Same => emit(app, false),
        Step::Finished => {
            emit(app, true);
            crate::tray::refresh(app);
            let app = app.clone();
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(1400));
                if !is_active(&app) {
                    crate::hud::hide(&app);
                }
            });
        }
        Step::Next(id) => {
            write_item(&state, id);
            emit(app, false);
        }
    }
}

// ---- paste peek ------------------------------------------------------------------------------

#[derive(Serialize, TS, Clone, Debug)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PeekState {
    pub visible: bool,
    pub items: Vec<Item>,
}

static PEEK_GEN: AtomicU64 = AtomicU64::new(0);
static PEEK_VISIBLE: AtomicBool = AtomicBool::new(false);
/// Our own synthetic ⌘V (paste from panel/peek) must not re-open the peek.
static SUPPRESS_UNTIL: Mutex<Option<Instant>> = Mutex::new(None);
const PEEK_DURATION: Duration = Duration::from_millis(5000);

pub fn suppress_peek() {
    *SUPPRESS_UNTIL.lock().unwrap_or_else(|e| e.into_inner()) =
        Some(Instant::now() + Duration::from_millis(900));
}

fn suppressed() -> bool {
    SUPPRESS_UNTIL
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .map(|t| Instant::now() < t)
        .unwrap_or(false)
}

pub fn peek_state(state: &AppState) -> PeekState {
    let n = state.settings().paste_peek_count.clamp(1, 20);
    PeekState {
        visible: PEEK_VISIBLE.load(Ordering::SeqCst),
        items: q::latest(&state.db(), n).unwrap_or_default(),
    }
}

fn show_peek(app: &AppHandle) {
    let state = app.state::<Arc<AppState>>().inner().clone();
    PEEK_VISIBLE.store(true, Ordering::SeqCst);
    let _ = app.emit("peek:state", peek_state(&state));
    crate::hud::show(app);
    let generation = PEEK_GEN.fetch_add(1, Ordering::SeqCst) + 1;
    let app = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(PEEK_DURATION);
        if PEEK_GEN.load(Ordering::SeqCst) == generation {
            hide_peek(&app);
        }
    });
}

pub fn hide_peek(app: &AppHandle) {
    if PEEK_VISIBLE.swap(false, Ordering::SeqCst) {
        let _ = app.emit(
            "peek:state",
            PeekState {
                visible: false,
                items: vec![],
            },
        );
        if !is_active(app) {
            crate::hud::hide(app);
        }
    }
}

// ---- ⌘V detection ----------------------------------------------------------------------------

static INSTALLED: AtomicBool = AtomicBool::new(false);
static WAITING: AtomicBool = AtomicBool::new(false);
static CMD_V_DOWN: AtomicBool = AtomicBool::new(false);
const KEY_V: u16 = 9;

fn on_cmd_v_released(app: &AppHandle) {
    let state = app.state::<Arc<AppState>>().inner().clone();
    if is_active(app) {
        handle_user_paste(app);
    } else if state.settings().paste_peek
        && !suppressed()
        && !state.panel_visible.load(Ordering::SeqCst)
    {
        show_peek(app);
    }
}

/// Install the global ⌘V monitor as soon as the Accessibility permission is granted.
pub fn ensure_monitor(app: AppHandle) {
    if INSTALLED.load(Ordering::SeqCst) || WAITING.swap(true, Ordering::SeqCst) {
        return;
    }
    std::thread::Builder::new()
        .name("sequence-permission".into())
        .spawn(move || {
            loop {
                if crate::permissions::is_trusted() {
                    let handle = app.clone();
                    let _ = app.run_on_main_thread(move || install(handle));
                    break;
                }
                std::thread::sleep(Duration::from_secs(2));
            }
            WAITING.store(false, Ordering::SeqCst);
        })
        .expect("spawn sequence thread");
}

fn install(app: AppHandle) {
    use block2::RcBlock;
    use objc2_app_kit::{NSEvent, NSEventMask, NSEventModifierFlags, NSEventType};
    use std::ptr::NonNull;

    if INSTALLED.swap(true, Ordering::SeqCst) {
        return;
    }
    let handle = app.clone();
    let block = RcBlock::new(move |ev: NonNull<NSEvent>| {
        let ev = unsafe { ev.as_ref() };
        if ev.keyCode() != KEY_V {
            return;
        }
        let ty = ev.r#type();
        if ty == NSEventType::KeyDown {
            let f = ev.modifierFlags();
            let only_cmd = f.contains(NSEventModifierFlags::Command)
                && !f.intersects(
                    NSEventModifierFlags::Shift
                        | NSEventModifierFlags::Option
                        | NSEventModifierFlags::Control,
                );
            if only_cmd && !ev.isARepeat() {
                CMD_V_DOWN.store(true, Ordering::SeqCst);
            }
        } else if ty == NSEventType::KeyUp && CMD_V_DOWN.swap(false, Ordering::SeqCst) {
            let app = handle.clone();
            // The target app has handled the keyDown by now; give it a moment to read the pasteboard.
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(60));
                on_cmd_v_released(&app);
            });
        }
    });
    match NSEvent::addGlobalMonitorForEventsMatchingMask_handler(
        NSEventMask::KeyDown | NSEventMask::KeyUp,
        &block,
    ) {
        Some(monitor) => {
            // Keep the monitor (and its block) alive for the whole process lifetime.
            std::mem::forget(monitor);
            std::mem::forget(block);
            log::info!("⌘V monitor installed");
            if is_active(&app) {
                emit(&app, false);
            }
        }
        None => {
            INSTALLED.store(false, Ordering::SeqCst);
            log::warn!("could not install the ⌘V key monitor");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session(last_first: bool, cycle: bool) -> PasteStack {
        let mut s = PasteStack::default();
        s.start(last_first, cycle);
        s
    }

    /// Simulate the IO layer: apply writes with fresh change counts.
    fn copy(s: &mut PasteStack, id: i64, cc: &mut isize) {
        *cc += 1;
        if s.on_capture(id, *cc).is_some() {
            *cc += 1;
            s.on_swapped(*cc);
        }
    }
    fn paste(s: &mut PasteStack, cc: &mut isize) -> Step {
        let step = s.on_paste(*cc);
        if let Step::Next(_) = step {
            *cc += 1;
            s.on_swapped(*cc);
        }
        step
    }

    #[test]
    fn last_first_consumes_in_reverse_order() {
        let mut s = session(true, false);
        let mut cc = 0;
        for id in [1, 2, 3] {
            copy(&mut s, id, &mut cc);
        }
        assert_eq!(s.upcoming(), vec![3, 2, 1]);
        assert_eq!(s.current(), Some(3));
        assert_eq!(paste(&mut s, &mut cc), Step::Next(2));
        assert_eq!(paste(&mut s, &mut cc), Step::Next(1));
        assert_eq!(paste(&mut s, &mut cc), Step::Finished);
        assert!(!s.is_active());
    }

    #[test]
    fn first_first_keeps_first_copy_on_the_pasteboard() {
        let mut s = session(false, false);
        assert_eq!(s.on_capture(1, 1), None); // first copy is already the current one
        assert_eq!(s.on_capture(2, 2), Some(1), "second copy: put #1 back");
        s.on_swapped(3);
        assert_eq!(s.on_capture(3, 4), Some(1));
        s.on_swapped(5);
        let mut cc = 5;
        assert_eq!(s.upcoming(), vec![1, 2, 3]);
        assert_eq!(paste(&mut s, &mut cc), Step::Next(2));
        assert_eq!(paste(&mut s, &mut cc), Step::Next(3));
        assert_eq!(paste(&mut s, &mut cc), Step::Finished);
    }

    #[test]
    fn cycle_loops_in_copy_order() {
        let mut s = session(false, true);
        let mut cc = 0;
        for id in [1, 2, 3] {
            copy(&mut s, id, &mut cc);
        }
        let mut pasted = vec![s.current().unwrap()];
        for _ in 0..5 {
            match paste(&mut s, &mut cc) {
                Step::Next(id) => pasted.push(id),
                other => panic!("{other:?}"),
            }
        }
        assert_eq!(pasted, vec![1, 2, 3, 1, 2, 3]);
        assert!(s.is_active(), "cycle never finishes on its own");
        assert_eq!(
            s.upcoming(),
            vec![3, 1, 2],
            "3 was just put on the pasteboard"
        );
    }

    #[test]
    fn cycle_last_first_and_single_item() {
        let mut s = session(true, true);
        let mut cc = 0;
        for id in [1, 2, 3] {
            copy(&mut s, id, &mut cc);
        }
        assert_eq!(s.current(), Some(3));
        assert_eq!(paste(&mut s, &mut cc), Step::Next(2));
        assert_eq!(paste(&mut s, &mut cc), Step::Next(1));
        assert_eq!(paste(&mut s, &mut cc), Step::Next(3));
        let mut one = session(false, true);
        let mut cc = 0;
        copy(&mut one, 9, &mut cc);
        assert_eq!(paste(&mut one, &mut cc), Step::Same);
    }

    #[test]
    fn removing_the_current_item_prepares_the_next() {
        let mut s = session(true, false);
        let mut cc = 0;
        for id in [1, 2, 3] {
            copy(&mut s, id, &mut cc);
        }
        assert_eq!(s.remove(3), Some(2), "3 was next: put 2 on the pasteboard");
        assert_eq!(s.remove(1), None, "1 was not next");
        assert_eq!(s.upcoming(), vec![2]);
        let mut c = session(false, true);
        let mut cc = 0;
        for id in [1, 2, 3] {
            copy(&mut c, id, &mut cc);
        }
        assert_eq!(paste(&mut c, &mut cc), Step::Next(2));
        assert_eq!(c.remove(2), Some(3), "cycle moves on to what came after");
        assert_eq!(c.upcoming(), vec![3, 1]);
    }

    #[test]
    fn reconfigure_mid_session() {
        let mut s = session(true, false);
        let mut cc = 0;
        for id in [1, 2, 3] {
            copy(&mut s, id, &mut cc);
        }
        assert_eq!(s.configure(false, false), Some(1));
        assert_eq!(s.upcoming(), vec![1, 2, 3]);
        assert_eq!(s.configure(false, false), None, "no change");
    }

    #[test]
    fn inactive_and_unrecorded() {
        let mut s = PasteStack::default();
        assert_eq!(s.on_capture(1, 10), None);
        assert_eq!(s.on_paste(10), Step::Ignored);
        let mut s = session(true, false);
        s.on_capture(1, 10);
        s.on_capture(2, 11);
        s.on_unrecorded();
        assert_eq!(s.on_paste(15), Step::Ignored);
        assert_eq!(s.upcoming(), vec![2, 1]);
    }
}
