//! Menu bar icon (template image drawn at runtime) and its menu.

use crate::db::queries as q;
use crate::i18n::tr;
use crate::models::{Item, Kind};
use crate::state::AppState;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use tauri::image::Image;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, Manager};

pub const TRAY_ID: &str = "clipstack-tray";

/// Signed distance from (px,py) to a rounded rectangle.
fn sd_round_rect(px: f32, py: f32, x: f32, y: f32, w: f32, h: f32, r: f32) -> f32 {
    let cx = x + w / 2.0;
    let cy = y + h / 2.0;
    let qx = (px - cx).abs() - (w / 2.0 - r);
    let qy = (py - cy).abs() - (h / 2.0 - r);
    let ox = qx.max(0.0);
    let oy = qy.max(0.0);
    (ox * ox + oy * oy).sqrt() + qx.max(qy).min(0.0) - r
}

/// Original Clipstack glyph: a filled card stacked in front of an outlined card, with two
/// text lines cut out. Paused state shows only outlines plus a pause mark.
pub fn draw_icon(paused: bool) -> Image<'static> {
    const S: u32 = 44; // 22 pt @2x
    let mut rgba = vec![0u8; (S * S * 4) as usize];
    for y in 0..S {
        for x in 0..S {
            let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
            // Back card (outline), offset up-right.
            let back = sd_round_rect(px, py, 13.0, 5.0, 25.0, 27.0, 5.0);
            let back_a = (1.6 - back.abs()).clamp(0.0, 1.0);
            // Front card.
            let front = sd_round_rect(px, py, 6.0, 12.0, 25.0, 27.0, 5.0);
            let mut a = if paused {
                (1.6 - front.abs()).clamp(0.0, 1.0)
            } else {
                (0.5 - front).clamp(0.0, 1.0)
            };
            // Hide the back outline where the front card covers it.
            let covered = front < 1.5;
            if !covered {
                a = a.max(back_a);
            }
            if !paused {
                // Two text lines cut out of the front card.
                let l1 = sd_round_rect(px, py, 11.0, 20.0, 15.0, 3.2, 1.6);
                let l2 = sd_round_rect(px, py, 11.0, 27.0, 10.0, 3.2, 1.6);
                let cut = (0.5 - l1.min(l2)).clamp(0.0, 1.0);
                a *= 1.0 - cut;
            } else {
                let b1 = sd_round_rect(px, py, 14.0, 19.0, 3.5, 13.0, 1.75);
                let b2 = sd_round_rect(px, py, 20.5, 19.0, 3.5, 13.0, 1.75);
                a = a.max((0.5 - b1.min(b2)).clamp(0.0, 1.0));
            }
            let i = ((y * S + x) * 4) as usize;
            rgba[i + 3] = (a * 255.0) as u8;
        }
    }
    Image::new_owned(rgba, S, S)
}

fn label_for(item: &Item, lang: &str) -> String {
    let base = match item.kind {
        Kind::Image => format!(
            "{} {}×{}",
            tr(lang, "image"),
            item.width.unwrap_or(0),
            item.height.unwrap_or(0)
        ),
        Kind::File => {
            let first = item
                .file_paths
                .first()
                .and_then(|p| std::path::Path::new(p).file_name())
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            if item.file_paths.len() > 1 {
                format!(
                    "{first} +{} {}",
                    item.file_paths.len() - 1,
                    tr(lang, "files")
                )
            } else {
                first
            }
        }
        _ => item
            .preview
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" "),
    };
    let mut s: String = base.chars().take(48).collect();
    if base.chars().count() > 48 {
        s.push('…');
    }
    s
}

fn build_menu(app: &AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
    let state = app.state::<Arc<AppState>>();
    let lang = crate::i18n::lang(app);
    let paused = state.paused.load(Ordering::SeqCst);
    let menu = Menu::new(app)?;
    menu.append(&MenuItem::with_id(
        app,
        "open",
        tr(lang, "openHistory"),
        true,
        None::<&str>,
    )?)?;
    menu.append(&MenuItem::with_id(
        app,
        "pause",
        if paused {
            tr(lang, "resume")
        } else {
            tr(lang, "pause")
        },
        true,
        None::<&str>,
    )?)?;
    menu.append(&PredefinedMenuItem::separator(app)?)?;
    menu.append(&MenuItem::with_id(
        app,
        "recent-header",
        tr(lang, "recent"),
        false,
        None::<&str>,
    )?)?;
    let recent = q::latest(&state.db(), 5).unwrap_or_default();
    if recent.is_empty() {
        menu.append(&MenuItem::with_id(
            app,
            "recent-none",
            tr(lang, "noRecent"),
            false,
            None::<&str>,
        )?)?;
    }
    for item in recent.iter() {
        menu.append(&MenuItem::with_id(
            app,
            format!("recent:{}", item.id),
            label_for(item, lang),
            true,
            None::<&str>,
        )?)?;
    }
    menu.append(&PredefinedMenuItem::separator(app)?)?;
    menu.append(&MenuItem::with_id(
        app,
        "settings",
        tr(lang, "settings"),
        true,
        None::<&str>,
    )?)?;
    menu.append(&MenuItem::with_id(
        app,
        "quit",
        tr(lang, "quit"),
        true,
        None::<&str>,
    )?)?;
    Ok(menu)
}

pub fn init(app: &AppHandle) -> tauri::Result<()> {
    let paused = app.state::<Arc<AppState>>().paused.load(Ordering::SeqCst);
    TrayIconBuilder::with_id(TRAY_ID)
        .icon(draw_icon(paused))
        .icon_as_template(true)
        .tooltip("Clipstack")
        .menu(&build_menu(app)?)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| {
            let id = event.id().as_ref();
            match id {
                "open" => crate::panel::show(app),
                "pause" => toggle_pause(app),
                "settings" => crate::windows::open_settings(app, None),
                "quit" => app.exit(0),
                other => {
                    if let Some(item_id) = other
                        .strip_prefix("recent:")
                        .and_then(|s| s.parse::<i64>().ok())
                    {
                        let app = app.clone();
                        // Menu tracking must end before the target app receives ⌘V.
                        std::thread::spawn(move || {
                            std::thread::sleep(std::time::Duration::from_millis(120));
                            if let Err(e) = crate::paste::paste_ids(&app, &[item_id], false) {
                                log::error!("paste from menu failed: {e}");
                            }
                        });
                    }
                }
            }
        })
        .build(app)?;
    Ok(())
}

/// Rebuild menu (recent items / labels) and icon state.
pub fn refresh(app: &AppHandle) {
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        let Some(tray) = handle.tray_by_id(TRAY_ID) else {
            return;
        };
        if let Ok(menu) = build_menu(&handle) {
            let _ = tray.set_menu(Some(menu));
        }
        let state = handle.state::<Arc<AppState>>();
        let paused = state.paused.load(Ordering::SeqCst);
        let _ = tray.set_icon(Some(draw_icon(paused)));
        let _ = tray.set_icon_as_template(true);
        let lang = crate::i18n::lang(&handle);
        let _ = tray.set_tooltip(Some(if paused {
            tr(lang, "paused")
        } else {
            "Clipstack"
        }));
    });
}

pub fn set_paused(app: &AppHandle, value: bool) {
    let state = app.state::<Arc<AppState>>();
    state.paused.store(value, Ordering::SeqCst);
    let _ = app.emit("monitoring:state", serde_json::json!({ "paused": value }));
    refresh(app);
}

pub fn toggle_pause(app: &AppHandle) {
    let paused = app.state::<Arc<AppState>>().paused.load(Ordering::SeqCst);
    set_paused(app, !paused);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn icon_has_ink_and_transparency() {
        for paused in [false, true] {
            let img = draw_icon(paused);
            let a: Vec<u8> = img.rgba().chunks(4).map(|p| p[3]).collect();
            assert!(a.contains(&255));
            assert!(a.contains(&0));
        }
    }
}
