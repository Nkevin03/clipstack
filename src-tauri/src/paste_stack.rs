//! "Copy sequence": press the sequence shortcut (default ⌥⇧⌘C), copy A, B, C, then ⌘V ⌘V ⌘V
//! in any app pastes C, B, A. A small HUD shows the stack live.
//!
//! While a session is active every capture is pushed on the stack. When the user releases ⌘V in
//! another app (global key monitor, needs Accessibility) the pasted item is popped and the
//! previous one is written to the pasteboard. The session ends when the stack is empty, or
//! when the shortcut / the HUD close button is used.
//! Nothing is swapped if the pasteboard changed in a way Clipstack did not record.

use crate::clipboard::pasteboard;
use crate::db::queries as q;
use crate::models::Item;
use crate::state::AppState;
use serde::Serialize;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use ts_rs::TS;

#[derive(Debug, PartialEq, Eq)]
pub enum Step {
    /// Put this item on the pasteboard for the next ⌘V.
    Next(i64),
    /// The last item was pasted: the session is over.
    Finished,
    /// Not in a session, or the pasteboard is not what the stack expects.
    Ignored,
}

#[derive(Debug, Default)]
pub struct PasteStack {
    active: bool,
    /// Item ids, oldest first; the last one is what the pasteboard currently holds.
    ids: Vec<i64>,
    /// How many items were pasted in this session.
    pasted: usize,
    /// Pasteboard change count the top of the stack corresponds to (-1 = unknown).
    expected_cc: isize,
}

impl PasteStack {
    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn start(&mut self) {
        *self = PasteStack {
            active: true,
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

    pub fn on_capture(&mut self, id: i64, cc: isize) {
        if !self.active {
            return;
        }
        self.ids.retain(|x| *x != id);
        self.ids.push(id);
        self.expected_cc = cc;
    }

    /// The pasteboard changed to something Clipstack did not store (password, excluded app,
    /// paused, a paste from the panel…): don't swap until the next recorded copy.
    pub fn on_unrecorded(&mut self) {
        self.expected_cc = -1;
    }

    /// The user pasted while the pasteboard had change count `current_cc`.
    pub fn on_paste(&mut self, current_cc: isize) -> Step {
        if !self.active || self.ids.is_empty() || current_cc != self.expected_cc {
            return Step::Ignored;
        }
        self.ids.pop();
        self.pasted += 1;
        match self.ids.last() {
            Some(id) => Step::Next(*id),
            None => {
                self.stop();
                Step::Finished
            }
        }
    }

    pub fn on_swapped(&mut self, cc: isize) {
        self.expected_cc = cc;
    }

    /// Ids newest first (the first one is pasted by the next ⌘V).
    pub fn ids_top_first(&self) -> Vec<i64> {
        self.ids.iter().rev().copied().collect()
    }
}

#[derive(Serialize, TS, Clone, Debug)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SequenceState {
    pub active: bool,
    /// Newest first: `items[0]` is pasted by the next ⌘V.
    pub items: Vec<Item>,
    pub pasted: i64,
    /// The session just ended because everything was pasted.
    pub finished: bool,
    pub accessibility: bool,
}

pub fn snapshot(state: &AppState, finished: bool) -> SequenceState {
    let (active, ids, pasted) = {
        let st = state.paste_stack.lock().unwrap_or_else(|e| e.into_inner());
        (st.active, st.ids_top_first(), st.pasted)
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
    }
}

pub fn emit(app: &AppHandle, finished: bool) {
    let state = app.state::<Arc<AppState>>();
    let snap = snapshot(&state, finished);
    let _ = app.emit("sequence:state", &snap);
}

pub fn lock(state: &AppState) -> std::sync::MutexGuard<'_, PasteStack> {
    state.paste_stack.lock().unwrap_or_else(|e| e.into_inner())
}

pub fn is_active(app: &AppHandle) -> bool {
    lock(&app.state::<Arc<AppState>>()).is_active()
}

pub fn start_session(app: &AppHandle) {
    let state = app.state::<Arc<AppState>>().inner().clone();
    lock(&state).start();
    if !crate::permissions::is_trusted() {
        // Shows the macOS prompt; the HUD explains why ⌘V can't be followed yet.
        crate::permissions::request();
    }
    ensure_monitor(app.clone());
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

/// Called (off the main thread) shortly after the user released ⌘V in another app.
pub fn handle_user_paste(app: &AppHandle) {
    let state = app.state::<Arc<AppState>>().inner().clone();
    if state.panel_visible.load(Ordering::SeqCst) {
        return;
    }
    let step = lock(&state).on_paste(pasteboard::change_count());
    match step {
        Step::Ignored => {}
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
            let row = q::get_full(&state.db(), id).ok().flatten();
            match row.map(|r| crate::paste::build(&[r], false, "\n")) {
                Some(Ok(data)) => {
                    let cc = pasteboard::write(&data);
                    state.own_change_count.store(cc, Ordering::SeqCst);
                    lock(&state).on_swapped(cc);
                }
                _ => {
                    log::warn!("copy sequence: item #{id} unavailable");
                    lock(&state).on_unrecorded();
                }
            }
            emit(app, false);
        }
    }
}

// ---- ⌘V detection ----------------------------------------------------------------------------

static INSTALLED: AtomicBool = AtomicBool::new(false);
static WAITING: AtomicBool = AtomicBool::new(false);
static CMD_V_DOWN: AtomicBool = AtomicBool::new(false);
const KEY_V: u16 = 9;

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
        if ev.keyCode() != KEY_V || !is_active(&handle) {
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
                handle_user_paste(&app);
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
            log::info!("copy sequence: ⌘V monitor installed");
            emit(&app, false);
        }
        None => {
            INSTALLED.store(false, Ordering::SeqCst);
            log::warn!("copy sequence: could not install key monitor");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session() -> PasteStack {
        let mut s = PasteStack::default();
        s.start();
        s
    }

    #[test]
    fn pastes_in_reverse_copy_order_then_finishes() {
        let mut s = session();
        s.on_capture(1, 10);
        s.on_capture(2, 11);
        s.on_capture(3, 12);
        assert_eq!(s.ids_top_first(), vec![3, 2, 1]);
        assert_eq!(s.on_paste(12), Step::Next(2));
        s.on_swapped(13);
        assert_eq!(s.on_paste(13), Step::Next(1));
        s.on_swapped(14);
        assert_eq!(s.on_paste(14), Step::Finished);
        assert!(!s.is_active());
        assert_eq!(s.on_paste(14), Step::Ignored);
    }

    #[test]
    fn inactive_does_nothing() {
        let mut s = PasteStack::default();
        s.on_capture(1, 10);
        s.on_capture(2, 11);
        assert_eq!(s.on_paste(11), Step::Ignored);
        assert!(s.ids_top_first().is_empty());
    }

    #[test]
    fn copies_after_a_paste_are_pushed_on_top() {
        let mut s = session();
        s.on_capture(1, 10);
        s.on_capture(2, 11);
        assert_eq!(s.on_paste(11), Step::Next(1));
        s.on_swapped(12);
        s.on_capture(5, 13);
        assert_eq!(s.ids_top_first(), vec![5, 1]);
        assert_eq!(s.on_paste(13), Step::Next(1));
    }

    #[test]
    fn unrecorded_change_pauses_swapping_but_keeps_stack() {
        let mut s = session();
        s.on_capture(1, 10);
        s.on_capture(2, 11);
        s.on_unrecorded(); // e.g. a concealed password was copied
        assert_eq!(s.on_paste(15), Step::Ignored);
        assert_eq!(s.ids_top_first(), vec![2, 1]);
        s.on_capture(3, 16);
        assert_eq!(s.on_paste(16), Step::Next(2));
    }

    #[test]
    fn recopying_moves_item_to_top_without_duplicates() {
        let mut s = session();
        s.on_capture(1, 10);
        s.on_capture(2, 11);
        s.on_capture(1, 12);
        assert_eq!(s.ids_top_first(), vec![1, 2]);
    }
}
