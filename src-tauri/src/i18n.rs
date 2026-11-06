//! Native-side strings (menu bar, notifications). The UI has its own catalogue in `src/lib/i18n`.

use crate::state::AppState;
use std::sync::Arc;
use tauri::{AppHandle, Manager};

pub fn lang(app: &AppHandle) -> &'static str {
    let setting = app
        .try_state::<Arc<AppState>>()
        .map(|s| s.settings().language)
        .unwrap_or_else(|| "auto".into());
    match setting.as_str() {
        "it" => "it",
        "en" => "en",
        _ => crate::platform::system_language(),
    }
}

pub fn tr(lang: &str, key: &str) -> &'static str {
    let it = lang == "it";
    match key {
        "openHistory" => {
            if it {
                "Apri cronologia"
            } else {
                "Open History"
            }
        }
        "pause" => {
            if it {
                "Pausa registrazione"
            } else {
                "Pause Recording"
            }
        }
        "resume" => {
            if it {
                "Riprendi registrazione"
            } else {
                "Resume Recording"
            }
        }
        "pasteStack" => {
            if it {
                "Sequenza di copia"
            } else {
                "Copy Sequence"
            }
        }
        "recent" => {
            if it {
                "Elementi recenti"
            } else {
                "Recent Items"
            }
        }
        "noRecent" => {
            if it {
                "Nessun elemento"
            } else {
                "No items yet"
            }
        }
        "settings" => {
            if it {
                "Impostazioni…"
            } else {
                "Settings…"
            }
        }
        "quit" => {
            if it {
                "Esci da Clipstack"
            } else {
                "Quit Clipstack"
            }
        }
        "copiedPressCmdV" => {
            if it {
                "Elemento copiato, premi ⌘V. Abilita Clipstack in Accessibilità per incollare automaticamente."
            } else {
                "Item copied, press ⌘V. Enable Clipstack in Accessibility to paste automatically."
            }
        }
        "paused" => {
            if it {
                "Clipstack (in pausa)"
            } else {
                "Clipstack (paused)"
            }
        }
        "image" => {
            if it {
                "Immagine"
            } else {
                "Image"
            }
        }
        "files" => {
            if it {
                "file"
            } else {
                "files"
            }
        }
        "settingsTitle" => {
            if it {
                "Impostazioni Clipstack"
            } else {
                "Clipstack Settings"
            }
        }
        "welcomeTitle" => {
            if it {
                "Benvenuto in Clipstack"
            } else {
                "Welcome to Clipstack"
            }
        }
        _ => "",
    }
}

pub fn t(app: &AppHandle, key: &str) -> String {
    tr(lang(app), key).to_string()
}

/// `CommandOrControl+Alt+Shift+C` → `⌥⇧⌘C` (menu labels).
pub fn pretty_shortcut(acc: &str) -> String {
    let mut mods = String::new();
    let mut key = String::new();
    let parts: Vec<&str> = acc.split('+').collect();
    for (sym, names) in [
        ("⌃", ["Control", "Ctrl"]),
        ("⌥", ["Alt", "Option"]),
        ("⇧", ["Shift", "Shift"]),
        ("⌘", ["CommandOrControl", "Command"]),
    ] {
        if parts.iter().any(|p| {
            names.contains(p) || (sym == "⌘" && (*p == "Cmd" || *p == "Super" || *p == "CmdOrCtrl"))
        }) {
            mods.push_str(sym);
        }
    }
    if let Some(last) = parts.last() {
        key = last
            .trim_start_matches("Key")
            .trim_start_matches("Digit")
            .to_string();
    }
    format!("{mods}{key}")
}
