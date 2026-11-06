//! "Paste in sequence": copy A, B, C, then ⌘V ⌘V ⌘V in any app pastes C, B, A.
//!
//! The stack is made of consecutive captures. When the user presses ⌘V in another app (seen by
//! a global key monitor, which needs the Accessibility permission), the item just pasted is
//! popped and the previous one is written to the pasteboard, ready for the next ⌘V.
//! A new copy after a paste starts a new stack. Nothing is swapped if the pasteboard changed
//! in a way Clipstack did not record (skipped/concealed copy, paused, another tool).

use crate::clipboard::pasteboard;
use crate::db::queries as q;
use crate::state::AppState;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tauri::{AppHandle, Manager};

#[derive(Debug, Default)]
pub struct PasteStack {
    /// Item ids, oldest first; the last one is what the pasteboard currently holds.
    ids: Vec<i64>,
    /// Pasteboard change count the stack corresponds to.
    expected_cc: isize,
    /// A paste happened: the next capture starts a fresh stack.
    sealed: bool,
}

impl PasteStack {
    pub fn on_capture(&mut self, id: i64, cc: isize) {
        if self.sealed {
            self.ids.clear();
            self.sealed = false;
        }
        self.ids.retain(|x| *x != id);
        self.ids.push(id);
        self.expected_cc = cc;
    }

    pub fn reset(&mut self) {
        self.ids.clear();
        self.sealed = false;
        self.expected_cc = -1;
    }

    /// The user pasted while the pasteboard had change count `current_cc`.
    /// Returns the id that should go on the pasteboard next, if any.
    pub fn on_paste(&mut self, current_cc: isize) -> Option<i64> {
        if self.ids.is_empty() || current_cc != self.expected_cc {
            self.reset();
            return None;
        }
        self.sealed = true;
        self.ids.pop();
        self.ids.last().copied()
    }

    pub fn on_swapped(&mut self, cc: isize) {
        self.expected_cc = cc;
    }

    pub fn remaining(&self) -> usize {
        self.ids.len()
    }
}

/// Called (off the main thread) shortly after the user released ⌘V in another app.
pub fn handle_user_paste(app: &AppHandle) {
    let state = app.state::<Arc<AppState>>().inner().clone();
    let settings = state.settings();
    if !settings.paste_stack || state.panel_visible.load(Ordering::SeqCst) {
        return;
    }
    let next = {
        let mut st = state.paste_stack.lock().unwrap_or_else(|e| e.into_inner());
        st.on_paste(pasteboard::change_count())
    };
    let Some(id) = next else { return };
    let row = match q::get_full(&state.db(), id) {
        Ok(Some(r)) => r,
        _ => {
            state
                .paste_stack
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .reset();
            return;
        }
    };
    match crate::paste::build(&[row], false, "\n") {
        Ok(data) => {
            let cc = pasteboard::write(&data);
            state.own_change_count.store(cc, Ordering::SeqCst);
            let mut st = state.paste_stack.lock().unwrap_or_else(|e| e.into_inner());
            st.on_swapped(cc);
            log::info!("paste stack: next #{id} ready ({} left)", st.remaining());
        }
        Err(e) => {
            log::warn!("paste stack: cannot prepare #{id}: {e}");
            state
                .paste_stack
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .reset();
        }
    }
}

static INSTALLED: AtomicBool = AtomicBool::new(false);
static CMD_V_DOWN: AtomicBool = AtomicBool::new(false);
const KEY_V: u16 = 9;

/// Install the global ⌘V monitor once the Accessibility permission is granted.
pub fn start(app: AppHandle) {
    std::thread::Builder::new()
        .name("paste-stack-permission".into())
        .spawn(move || {
            loop {
                if crate::permissions::is_trusted() {
                    let handle = app.clone();
                    let _ = app.run_on_main_thread(move || install(handle));
                    return;
                }
                std::thread::sleep(Duration::from_secs(3));
            }
        })
        .expect("spawn paste-stack thread");
}

fn install(app: AppHandle) {
    use block2::RcBlock;
    use objc2_app_kit::{NSEvent, NSEventMask, NSEventModifierFlags, NSEventType};
    use std::ptr::NonNull;

    if INSTALLED.swap(true, Ordering::SeqCst) {
        return;
    }
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
            let app = app.clone();
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
            log::info!("paste stack: ⌘V monitor installed");
        }
        None => {
            INSTALLED.store(false, Ordering::SeqCst);
            log::warn!("paste stack: could not install key monitor");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pastes_in_reverse_copy_order() {
        let mut s = PasteStack::default();
        s.on_capture(1, 10);
        s.on_capture(2, 11);
        s.on_capture(3, 12); // pasteboard holds 3
        assert_eq!(s.on_paste(12), Some(2)); // 3 pasted → put 2
        s.on_swapped(13);
        assert_eq!(s.on_paste(13), Some(1)); // 2 pasted → put 1
        s.on_swapped(14);
        assert_eq!(s.on_paste(14), None); // 1 pasted → stack exhausted, keep 1
        assert_eq!(s.on_paste(14), None); // further pastes repeat 1
    }

    #[test]
    fn new_copy_after_paste_starts_new_stack() {
        let mut s = PasteStack::default();
        s.on_capture(1, 10);
        s.on_capture(2, 11);
        assert_eq!(s.on_paste(11), Some(1));
        s.on_swapped(12);
        s.on_capture(5, 13); // new copy
        assert_eq!(s.remaining(), 1);
        assert_eq!(s.on_paste(13), None);
    }

    #[test]
    fn unknown_pasteboard_change_resets() {
        let mut s = PasteStack::default();
        s.on_capture(1, 10);
        s.on_capture(2, 11);
        // Something not recorded (e.g. a concealed password) is on the pasteboard now.
        assert_eq!(s.on_paste(15), None);
        assert_eq!(s.remaining(), 0);
    }

    #[test]
    fn recopying_moves_item_to_top_without_duplicates() {
        let mut s = PasteStack::default();
        s.on_capture(1, 10);
        s.on_capture(2, 11);
        s.on_capture(1, 12);
        assert_eq!(s.on_paste(12), Some(2));
        s.on_swapped(13);
        assert_eq!(s.on_paste(13), None);
    }
}
