//! Writing items back to the pasteboard and simulating ⌘V in the previous app.

use crate::clipboard::pasteboard::{self, ImageFormat, WriteData};
use crate::db::queries::{self as q, FullRow};
use crate::models::Kind;
use crate::state::AppState;
use core_graphics::event::{CGEvent, CGEventFlags, CGEventTapLocation};
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
use serde::Serialize;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

const KEY_V: u16 = 9;

pub fn separator(name: &str) -> &'static str {
    match name {
        "space" => " ",
        "comma" => ", ",
        "none" => "",
        _ => "\n",
    }
}

/// Build what to write for a set of rows. Multiple rows: texts joined with `sep`, files merged;
/// images can only be pasted alone (there is no meaningful way to concatenate them).
pub fn build(rows: &[FullRow], plain: bool, sep: &str) -> anyhow::Result<WriteData> {
    if let [row] = rows {
        let kind = Kind::parse(&row.kind).unwrap_or(Kind::Text);
        return Ok(match kind {
            Kind::File => WriteData {
                files: row
                    .file_paths
                    .as_deref()
                    .and_then(|f| serde_json::from_str(f).ok())
                    .unwrap_or_default(),
                text: if plain {
                    row.text_content.clone()
                } else {
                    None
                },
                ..Default::default()
            },
            Kind::Image => {
                let path = row
                    .image_path
                    .clone()
                    .ok_or_else(|| anyhow::anyhow!("image file missing"))?;
                let bytes = std::fs::read(&path)?;
                let fmt = std::path::Path::new(&path)
                    .extension()
                    .and_then(|e| e.to_str())
                    .and_then(ImageFormat::from_ext)
                    .unwrap_or(ImageFormat::Png);
                WriteData {
                    image: Some((bytes, fmt)),
                    ..Default::default()
                }
            }
            _ => WriteData {
                text: row.text_content.clone(),
                rtf: if plain { None } else { row.rich_rtf.clone() },
                html: if plain { None } else { row.rich_html.clone() },
                ..Default::default()
            },
        });
    }
    let all_files = rows.iter().all(|r| r.kind == "file");
    if all_files {
        let mut files: Vec<String> = Vec::new();
        for r in rows {
            let f: Vec<String> = r
                .file_paths
                .as_deref()
                .and_then(|f| serde_json::from_str(f).ok())
                .unwrap_or_default();
            files.extend(f);
        }
        return Ok(WriteData {
            files,
            ..Default::default()
        });
    }
    let text = rows
        .iter()
        .filter(|r| r.kind != "image")
        .filter_map(|r| r.text_content.clone())
        .collect::<Vec<_>>()
        .join(sep);
    if text.is_empty() {
        anyhow::bail!("nothing to paste");
    }
    Ok(WriteData {
        text: Some(text),
        ..Default::default()
    })
}

/// Put the given items on the pasteboard (no keystroke). Records our change count.
pub fn copy_ids(state: &AppState, ids: &[i64], plain: bool) -> anyhow::Result<()> {
    let settings = state.settings();
    let rows = {
        let db = state.db();
        q::get_full_many(&db, ids)?
    };
    if rows.is_empty() {
        anyhow::bail!("item not found");
    }
    let data = build(&rows, plain, separator(&settings.multi_separator))?;
    let cc = pasteboard::write(&data);
    state.own_change_count.store(cc, Ordering::SeqCst);
    // Our own write is not part of the copy sequence.
    state
        .paste_stack
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .on_unrecorded();
    let db = state.db();
    for r in &rows {
        q::mark_used(&db, r.id, settings.move_to_top_after_paste)?;
    }
    Ok(())
}

fn post_cmd_v() -> Result<(), ()> {
    let src = CGEventSource::new(CGEventSourceStateID::HIDSystemState)?;
    let down = CGEvent::new_keyboard_event(src.clone(), KEY_V, true)?;
    down.set_flags(CGEventFlags::CGEventFlagCommand);
    let up = CGEvent::new_keyboard_event(src, KEY_V, false)?;
    up.set_flags(CGEventFlags::CGEventFlagCommand);
    down.post(CGEventTapLocation::HID);
    up.post(CGEventTapLocation::HID);
    Ok(())
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PasteOutcome {
    /// true if ⌘V was sent, false if the item was only copied.
    pub pasted: bool,
    pub needs_permission: bool,
}

/// Copy, hide the panel and (if allowed) send ⌘V to the app that was frontmost.
pub fn paste_ids(app: &AppHandle, ids: &[i64], plain: bool) -> anyhow::Result<PasteOutcome> {
    let state = app.state::<Arc<AppState>>().inner().clone();
    copy_ids(&state, ids, plain)?;
    let _ = app.emit("items:changed", ());
    crate::tray::refresh(app);
    Ok(finish(app, &state))
}

/// Paste an arbitrary text (e.g. the result of a transformation) without storing it.
pub fn paste_text(app: &AppHandle, text: &str) -> anyhow::Result<PasteOutcome> {
    let state = app.state::<Arc<AppState>>().inner().clone();
    let cc = pasteboard::write(&WriteData {
        text: Some(text.to_string()),
        ..Default::default()
    });
    state.own_change_count.store(cc, Ordering::SeqCst);
    state
        .paste_stack
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .on_unrecorded();
    Ok(finish(app, &state))
}

fn finish(app: &AppHandle, state: &AppState) -> PasteOutcome {
    let settings = state.settings();
    crate::panel::hide(app);
    if !settings.auto_paste {
        return PasteOutcome {
            pasted: false,
            needs_permission: false,
        };
    }
    if !crate::permissions::is_trusted() {
        crate::platform::notify(app, "Clipstack", &crate::i18n::t(app, "copiedPressCmdV"));
        let _ = app.emit("paste:needs-permission", ());
        return PasteOutcome {
            pasted: false,
            needs_permission: true,
        };
    }
    crate::paste_stack::suppress_peek();
    std::thread::spawn(|| {
        // Let the panel order out and the target app's window become key again.
        std::thread::sleep(Duration::from_millis(70));
        if post_cmd_v().is_err() {
            log::error!("could not post ⌘V event");
        }
    });
    PasteOutcome {
        pasted: true,
        needs_permission: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(kind: &str, text: &str) -> FullRow {
        FullRow {
            kind: kind.into(),
            text_content: Some(text.into()),
            ..Default::default()
        }
    }

    #[test]
    fn plain_strips_rich() {
        let mut r = row("rich", "hi");
        r.rich_html = Some("<b>hi</b>".into());
        assert!(build(&[r.clone()], false, "\n").unwrap().html.is_some());
        let d = build(&[r], true, "\n").unwrap();
        assert!(d.html.is_none() && d.rtf.is_none());
        assert_eq!(d.text.as_deref(), Some("hi"));
    }

    #[test]
    fn multi_joins_with_separator() {
        let d = build(
            &[row("text", "a"), row("link", "b"), row("image", "")],
            false,
            ", ",
        )
        .unwrap();
        assert_eq!(d.text.as_deref(), Some("a, b"));
        assert_eq!(separator("none"), "");
    }

    #[test]
    fn multi_files_merge() {
        let mut a = row("file", "/a");
        a.file_paths = Some("[\"/a\"]".into());
        let mut b = row("file", "/b");
        b.file_paths = Some("[\"/b\",\"/c\"]".into());
        assert_eq!(
            build(&[a, b], false, "\n").unwrap().files,
            vec!["/a", "/b", "/c"]
        );
    }
}
