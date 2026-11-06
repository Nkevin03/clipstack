//! Copy-sequence HUD: a tiny non-activating panel (never key) showing the stack live,
//! anchored to the top-right corner of the screen with the cursor.

use objc2::MainThreadMarker;
use objc2_app_kit::{NSEvent, NSScreen, NSWindowStyleMask};
use objc2_foundation::{NSPoint, NSRect, NSSize};
use std::sync::Mutex;
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};
use tauri_nspanel::{CollectionBehavior, ManagerExt, WebviewWindowExt};

pub const LABEL: &str = "hud";
pub const WIDTH: f64 = 300.0;
const MARGIN: f64 = 12.0;

#[allow(clippy::unused_unit)]
mod native {
    use tauri_nspanel::tauri_panel;
    tauri_panel! {
        panel!(HudPanel {
            config: {
                can_become_key_window: false,
                can_become_main_window: false,
                is_floating_panel: true
            }
        })
    }
}
use native::HudPanel;

/// Top-right anchor (Cocoa coordinates) of the HUD on the current screen.
static ANCHOR: Mutex<Option<(f64, f64)>> = Mutex::new(None);
static HEIGHT: Mutex<f64> = Mutex::new(120.0);

fn cursor_visible_frame(mtm: MainThreadMarker) -> Option<NSRect> {
    let mouse = NSEvent::mouseLocation();
    let screens = NSScreen::screens(mtm);
    let mut first = None;
    for s in screens.iter() {
        let f = s.frame();
        first.get_or_insert(s.visibleFrame());
        if mouse.x >= f.origin.x
            && mouse.x <= f.origin.x + f.size.width
            && mouse.y >= f.origin.y
            && mouse.y <= f.origin.y + f.size.height
        {
            return Some(s.visibleFrame());
        }
    }
    first
}

fn apply_frame(app: &AppHandle) {
    let Ok(panel) = app.get_webview_panel(LABEL) else {
        return;
    };
    let Some((right, top)) = *ANCHOR.lock().unwrap_or_else(|e| e.into_inner()) else {
        return;
    };
    let h = *HEIGHT.lock().unwrap_or_else(|e| e.into_inner());
    let frame = NSRect::new(NSPoint::new(right - WIDTH, top - h), NSSize::new(WIDTH, h));
    panel.as_panel().setFrame_display(frame, true);
}

fn create(app: &AppHandle) -> tauri::Result<()> {
    let window = WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App("index.html#/hud".into()))
        .title("Clipstack")
        .inner_size(WIDTH, 120.0)
        .decorations(false)
        .transparent(true)
        .resizable(false)
        .skip_taskbar(true)
        .shadow(false)
        .focused(false)
        .visible(false)
        .accept_first_mouse(true)
        .build()?;
    {
        use window_vibrancy::{NSVisualEffectMaterial, NSVisualEffectState, apply_vibrancy};
        let _ = apply_vibrancy(
            &window,
            NSVisualEffectMaterial::HudWindow,
            Some(NSVisualEffectState::Active),
            Some(14.0),
        );
    }
    let theme = app
        .state::<std::sync::Arc<crate::state::AppState>>()
        .settings()
        .theme;
    crate::windows::apply_theme(app, &theme);
    let panel = window.to_panel::<HudPanel>()?;
    let _ = panel.add_style_mask(NSWindowStyleMask::NonactivatingPanel);
    panel.set_level(25);
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
    Ok(())
}

pub fn show(app: &AppHandle) {
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        if handle.get_webview_panel(LABEL).is_err()
            && let Err(e) = create(&handle)
        {
            log::error!("cannot create HUD: {e}");
            return;
        }
        if let Some(mtm) = MainThreadMarker::new()
            && let Some(v) = cursor_visible_frame(mtm)
        {
            *ANCHOR.lock().unwrap_or_else(|e| e.into_inner()) = Some((
                v.origin.x + v.size.width - MARGIN,
                v.origin.y + v.size.height - MARGIN,
            ));
        }
        apply_frame(&handle);
        if let Ok(panel) = handle.get_webview_panel(LABEL) {
            panel.order_front_regardless();
        }
    });
}

pub fn hide(app: &AppHandle) {
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        if let Ok(panel) = handle.get_webview_panel(LABEL) {
            panel.hide();
        }
    });
}

/// The HUD content measured itself: resize keeping the top-right corner fixed.
pub fn resize(app: &AppHandle, height: f64) {
    *HEIGHT.lock().unwrap_or_else(|e| e.into_inner()) = height.clamp(60.0, 520.0);
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || apply_frame(&handle));
}
