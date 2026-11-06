//! Global shortcuts: open/close the panel (default ⌘⇧V) and optional pause toggle.

use crate::state::AppState;
use std::str::FromStr;
use std::sync::Arc;
use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutEvent, ShortcutState};

pub fn plugin() -> tauri::plugin::TauriPlugin<tauri::Wry> {
    tauri_plugin_global_shortcut::Builder::new()
        .with_handler(
            |app: &AppHandle, shortcut: &Shortcut, event: ShortcutEvent| {
                if event.state != ShortcutState::Pressed {
                    return;
                }
                let Some(state) = app.try_state::<Arc<AppState>>() else {
                    return;
                };
                let s = state.settings();
                let is = |acc: &str| {
                    Shortcut::from_str(acc)
                        .map(|x| x.id() == shortcut.id())
                        .unwrap_or(false)
                };
                if is(&s.shortcut) {
                    crate::panel::toggle(app);
                } else if s.sequence_shortcut.as_deref().map(is).unwrap_or(false) {
                    crate::paste_stack::toggle_session(app);
                } else if s.pause_shortcut.as_deref().map(is).unwrap_or(false) {
                    crate::tray::toggle_pause(app);
                }
            },
        )
        .build()
}

/// Validate an accelerator string such as `CommandOrControl+Shift+V`.
pub fn parse(acc: &str) -> Result<Shortcut, String> {
    Shortcut::from_str(acc).map_err(|e| format!("invalid shortcut \"{acc}\": {e}"))
}

/// (Re-)register every configured shortcut. Returns the first human-readable error.
pub fn register_all(app: &AppHandle) -> Result<(), String> {
    let gs = app.global_shortcut();
    let _ = gs.unregister_all();
    let s = app.state::<Arc<AppState>>().settings();
    let main = parse(&s.shortcut)?;
    gs.register(main)
        .map_err(|e| format!("cannot register {}: {e}", s.shortcut))?;
    let mut first_err = None;
    for (what, acc) in [
        ("pause", &s.pause_shortcut),
        ("sequence", &s.sequence_shortcut),
    ] {
        let Some(p) = acc.as_deref().filter(|p| !p.is_empty()) else {
            continue;
        };
        if let Err(e) = parse(p).and_then(|sc| gs.register(sc).map_err(|e| e.to_string())) {
            log::warn!("{what} shortcut not registered: {e}");
            first_err.get_or_insert(format!("{p}: {e}"));
        }
    }
    if let Some(e) = first_err {
        return Err(e);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn default_shortcut_parses() {
        assert!(parse("CommandOrControl+Shift+V").is_ok());
        assert!(parse("Alt+Space").is_ok());
        assert!(parse("Banana+Q").is_err());
        assert!(parse("CommandOrControl+Alt+Shift+C").is_ok());
        assert_eq!(
            crate::i18n::pretty_shortcut("CommandOrControl+Alt+Shift+C"),
            "⌥⇧⌘C"
        );
    }
}
