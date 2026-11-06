//! Secondary windows: Settings and first-run Onboarding (created on demand).

use objc2::MainThreadMarker;
use objc2_app_kit::NSApplication;
use tauri::{AppHandle, Emitter, Manager, Theme, TitleBarStyle, WebviewUrl, WebviewWindowBuilder};

pub const SETTINGS: &str = "settings";
pub const ONBOARDING: &str = "onboarding";

fn activate_app() {
    if let Some(mtm) = MainThreadMarker::new() {
        #[allow(deprecated)]
        NSApplication::sharedApplication(mtm).activateIgnoringOtherApps(true);
    }
}

fn open(app: &AppHandle, label: &'static str, route: &str, title: String, size: (f64, f64)) {
    let handle = app.clone();
    let route = route.to_string();
    let _ = app.run_on_main_thread(move || {
        crate::panel::hide(&handle);
        if let Some(w) = handle.get_webview_window(label) {
            let _ = w.emit("route", &route);
            let _ = w.show();
            let _ = w.unminimize();
            activate_app();
            let _ = w.set_focus();
            return;
        }
        let built = WebviewWindowBuilder::new(
            &handle,
            label,
            WebviewUrl::App(format!("index.html#{route}").into()),
        )
        .title(title)
        .inner_size(size.0, size.1)
        .min_inner_size(size.0 * 0.85, size.1 * 0.85)
        .resizable(true)
        .title_bar_style(TitleBarStyle::Overlay)
        .hidden_title(true)
        .transparent(true)
        .center()
        .build();
        match built {
            Ok(w) => {
                use window_vibrancy::{
                    NSVisualEffectMaterial, NSVisualEffectState, apply_vibrancy,
                };
                let _ = apply_vibrancy(
                    &w,
                    NSVisualEffectMaterial::Sidebar,
                    Some(NSVisualEffectState::FollowsWindowActiveState),
                    None,
                );
                let theme = handle
                    .state::<std::sync::Arc<crate::state::AppState>>()
                    .settings()
                    .theme;
                apply_theme(&handle, &theme);
                activate_app();
                let _ = w.set_focus();
            }
            Err(e) => log::error!("cannot open {label}: {e}"),
        }
    });
}

pub fn open_settings(app: &AppHandle, section: Option<&str>) {
    let route = format!("/settings/{}", section.unwrap_or("general"));
    open(
        app,
        SETTINGS,
        &route,
        crate::i18n::t(app, "settingsTitle"),
        (780.0, 560.0),
    );
}

pub fn open_onboarding(app: &AppHandle) {
    open(
        app,
        ONBOARDING,
        "/onboarding",
        crate::i18n::t(app, "welcomeTitle"),
        (620.0, 520.0),
    );
}

/// Force light/dark on every window (vibrancy follows the window appearance), or follow system.
pub fn apply_theme(app: &AppHandle, theme: &str) {
    let t = match theme {
        "light" => Some(Theme::Light),
        "dark" => Some(Theme::Dark),
        _ => None,
    };
    for w in app.webview_windows().values() {
        let _ = w.set_theme(t);
    }
}
