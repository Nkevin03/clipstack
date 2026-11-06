//! The history panel: a non-activating `NSPanel` that can become key (so it receives
//! keystrokes) without activating Clipstack, so the frontmost app keeps focus.

use crate::settings::{Layout, Settings};
use crate::state::AppState;
use objc2::MainThreadMarker;
use objc2_app_kit::{NSEvent, NSScreen, NSWindowStyleMask};
use objc2_foundation::{NSPoint, NSRect, NSSize};
use serde::Serialize;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use tauri::{AppHandle, Emitter, Manager};
use tauri_nspanel::{CollectionBehavior, ManagerExt, WebviewWindowExt};

pub const LABEL: &str = "panel";
/// NSStatusWindowLevel: above normal and floating windows, also over full-screen apps.
const PANEL_LEVEL: i64 = 25;
const MARGIN: f64 = 8.0;

#[allow(clippy::unused_unit)] // the panel_event! macro requires an explicit `-> ()`
mod native {
    use tauri_nspanel::tauri_panel;
    tauri_panel! {
        panel!(ClipPanel {
            config: {
                can_become_key_window: true,
                can_become_main_window: false,
                is_floating_panel: true
            }
        })

        panel_event!(ClipPanelEvents {
            window_did_resign_key(notification: &NSNotification) -> ()
        })
    }

    /// Hide the panel when it stops being key (click outside / another window became key).
    pub fn install_events(panel: &tauri_nspanel::PanelHandle<tauri::Wry>, app: &tauri::AppHandle) {
        let events = ClipPanelEvents::new();
        let handle = app.clone();
        events.window_did_resign_key(move |_| super::hide(&handle));
        panel.set_event_handler(Some(events.as_ref()));
    }
}
use native::ClipPanel;

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ShownPayload {
    pub layout: Layout,
}

/// Convert the configured `panel` window into an NSPanel. Must run on the main thread.
pub fn init(app: &AppHandle) -> tauri::Result<()> {
    let window = app
        .get_webview_window(LABEL)
        .expect("panel window in tauri.conf.json");

    #[cfg(target_os = "macos")]
    {
        use window_vibrancy::{NSVisualEffectMaterial, NSVisualEffectState, apply_vibrancy};
        if let Err(e) = apply_vibrancy(
            &window,
            NSVisualEffectMaterial::HudWindow,
            Some(NSVisualEffectState::Active),
            Some(16.0),
        ) {
            log::warn!("vibrancy unavailable: {e}");
        }
    }

    let panel = window.to_panel::<ClipPanel>()?;
    if let Err(e) = panel.add_style_mask(NSWindowStyleMask::NonactivatingPanel) {
        log::error!("cannot make panel non-activating: {e:?}");
    }
    panel.set_level(PANEL_LEVEL);
    panel.set_collection_behavior(
        CollectionBehavior::new()
            .can_join_all_spaces()
            .full_screen_auxiliary()
            .ignores_cycle()
            .value(),
    );
    panel.set_hides_on_deactivate(false);
    panel.set_has_shadow(true);
    panel.set_transparent(true);

    native::install_events(&panel, app);
    Ok(())
}

/// Height of the strip layouts for a given card size.
fn strip_height(s: &Settings) -> f64 {
    let card = match s.card_size.as_str() {
        "small" => 200.0,
        "large" => 300.0,
        _ => 250.0,
    };
    card + 104.0
}

/// Frame (Cocoa coordinates, bottom-left origin) for the given visible screen area.
pub fn frame_for(layout: Layout, visible: NSRect, s: &Settings) -> NSRect {
    let vw = visible.size.width;
    let vh = visible.size.height;
    match layout {
        Layout::Top | Layout::Bottom => {
            let h = strip_height(s).min(vh - 2.0 * MARGIN);
            let w = vw - 2.0 * MARGIN;
            let y = if layout == Layout::Top {
                visible.origin.y + vh - h - MARGIN
            } else {
                visible.origin.y + MARGIN
            };
            NSRect::new(
                NSPoint::new(visible.origin.x + MARGIN, y),
                NSSize::new(w, h),
            )
        }
        Layout::Center => {
            let w = (vw - 120.0).clamp(480.0, 900.0).min(vw);
            let h = (vh - 160.0).clamp(360.0, 580.0).min(vh);
            let x = visible.origin.x + (vw - w) / 2.0;
            // Slightly above centre, like Spotlight.
            let y = visible.origin.y + (vh - h) / 2.0 + vh * 0.08;
            NSRect::new(
                NSPoint::new(x, y.min(visible.origin.y + vh - h)),
                NSSize::new(w, h),
            )
        }
    }
}

/// Visible frame of the screen containing the mouse cursor.
fn cursor_screen_visible(mtm: MainThreadMarker) -> Option<NSRect> {
    let mouse = NSEvent::mouseLocation();
    let screens = NSScreen::screens(mtm);
    let mut fallback = None;
    for s in screens.iter() {
        let f = s.frame();
        if fallback.is_none() {
            fallback = Some(s.visibleFrame());
        }
        if mouse.x >= f.origin.x
            && mouse.x <= f.origin.x + f.size.width
            && mouse.y >= f.origin.y
            && mouse.y <= f.origin.y + f.size.height
        {
            return Some(s.visibleFrame());
        }
    }
    fallback
}

pub fn is_visible(app: &AppHandle) -> bool {
    app.state::<Arc<AppState>>()
        .panel_visible
        .load(Ordering::SeqCst)
}

pub fn show(app: &AppHandle) {
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        let state = handle.state::<Arc<AppState>>().inner().clone();
        let Ok(panel) = handle.get_webview_panel(LABEL) else {
            return;
        };
        let Some(mtm) = MainThreadMarker::new() else {
            return;
        };
        if let Some(front) = crate::clipboard::source::frontmost()
            && front.bundle_id != handle.config().identifier
        {
            state.prev_app_pid.store(front.pid, Ordering::SeqCst);
        }
        let settings = state.settings();
        if let Some(visible) = cursor_screen_visible(mtm) {
            let frame = frame_for(settings.layout, visible, &settings);
            panel.as_panel().setFrame_display(frame, true);
        }
        state.panel_visible.store(true, Ordering::SeqCst);
        panel.show_and_make_key();
        let _ = handle.emit(
            "panel:shown",
            ShownPayload {
                layout: settings.layout,
            },
        );
    });
}

pub fn hide(app: &AppHandle) {
    let state = app.state::<Arc<AppState>>().inner().clone();
    if !state.panel_visible.swap(false, Ordering::SeqCst) {
        return;
    }
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        if let Ok(panel) = handle.get_webview_panel(LABEL) {
            panel.hide();
        }
        let _ = handle.emit("panel:hidden", ());
    });
}

pub fn toggle(app: &AppHandle) {
    if is_visible(app) {
        hide(app);
    } else {
        show(app);
    }
}

/// Re-apply the frame (layout/card size changed while visible).
pub fn relayout(app: &AppHandle) {
    if is_visible(app) {
        let state = app.state::<Arc<AppState>>().inner().clone();
        state.panel_visible.store(false, Ordering::SeqCst);
        show(app);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn screen() -> NSRect {
        // 1440×875 visible area starting above a 25 pt Dock, menu bar excluded.
        NSRect::new(NSPoint::new(0.0, 25.0), NSSize::new(1440.0, 850.0))
    }

    #[test]
    fn top_layout_is_full_width_under_menu_bar() {
        let s = Settings::default();
        let f = frame_for(Layout::Top, screen(), &s);
        assert_eq!(f.size.width, 1440.0 - 16.0);
        assert_eq!(f.origin.y + f.size.height, 25.0 + 850.0 - MARGIN);
    }

    #[test]
    fn bottom_and_center() {
        let s = Settings::default();
        let b = frame_for(Layout::Bottom, screen(), &s);
        assert_eq!(b.origin.y, 25.0 + MARGIN);
        let c = frame_for(Layout::Center, screen(), &s);
        assert!(c.size.width <= 900.0 && c.origin.x > 0.0);
        assert!(c.origin.y + c.size.height <= 25.0 + 850.0);
    }

    #[test]
    fn secondary_screen_offsets_are_respected() {
        let s = Settings::default();
        let second = NSRect::new(NSPoint::new(1440.0, -200.0), NSSize::new(1920.0, 1050.0));
        let f = frame_for(Layout::Top, second, &s);
        assert_eq!(f.origin.x, 1440.0 + MARGIN);
    }
}
