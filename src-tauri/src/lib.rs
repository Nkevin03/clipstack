mod actions;
mod clipboard;
mod commands;
mod data;
mod db;
mod hooks;
mod i18n;
mod linkmeta;
mod models;
mod ocr;
mod panel;
mod paste;
mod permissions;
mod platform;
mod privacy;
mod settings;
mod shortcuts;
mod state;
mod tray;
mod windows;

use state::{AppState, Paths};
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicIsize};
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;
use tauri::Manager;

fn start_maintenance(state: Arc<AppState>, app: tauri::AppHandle) {
    std::thread::Builder::new()
        .name("maintenance".into())
        .spawn(move || {
            let _ = data::cleanup(&state);
            let mut minutes = 0u32;
            loop {
                std::thread::sleep(Duration::from_secs(60));
                minutes += 1;
                // Sensitive items with an expiry go away within a minute of expiring.
                let expired =
                    db::queries::cleanup(&state.db(), db::queries::now_ms(), 0, 0).unwrap_or(0);
                if minutes.is_multiple_of(60) {
                    let _ = data::cleanup(&state);
                }
                // Drop the undo buffer after 30 s so its files can be collected.
                {
                    let mut u = state.undo.lock().unwrap_or_else(|e| e.into_inner());
                    if u.as_ref()
                        .map(|b| b.at.elapsed() > Duration::from_secs(30))
                        .unwrap_or(false)
                    {
                        *u = None;
                    }
                }
                if expired > 0 {
                    use tauri::Emitter;
                    let _ = app.emit("items:changed", ());
                    tray::refresh(&app);
                }
            }
        })
        .expect("spawn maintenance thread");
}

/// stderr in development; `~/Library/Logs/Clipstack/clipstack.log` (rotated at 2 MB) in release.
fn init_logging() {
    let mut b = env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("clipstack_lib=info"),
    );
    if !cfg!(debug_assertions)
        && let Some(home) = std::env::var_os("HOME")
    {
        let dir = std::path::Path::new(&home).join("Library/Logs/Clipstack");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("clipstack.log");
        if std::fs::metadata(&path)
            .map(|m| m.len() > 2 * 1024 * 1024)
            .unwrap_or(false)
        {
            let _ = std::fs::rename(&path, dir.join("clipstack.old.log"));
        }
        if let Ok(f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
        {
            b.target(env_logger::Target::Pipe(Box::new(f)));
        }
    }
    b.init();
    std::panic::set_hook(Box::new(|info| log::error!("panic: {info}")));
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    init_logging();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_nspanel::init())
        .plugin(shortcuts::plugin())
        .setup(|app| {
            // Menu-bar only app: no Dock icon, no app menu (also enforced by LSUIElement in Info.plist).
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let paths = Paths::new(app.path().app_data_dir()?)?;
            let conn = data::open_db(&paths.db)?;
            let settings = settings::Settings::load(&conn)?;
            log::info!("data dir: {}", paths.root.display());
            let onboarding_done = settings.onboarding_done;
            let theme = settings.theme.clone();
            let state = Arc::new(AppState {
                db: Mutex::new(conn),
                paths,
                settings: RwLock::new(settings),
                paused: AtomicBool::new(false),
                own_change_count: AtomicIsize::new(-1),
                undo: Mutex::new(None),
                prev_app_pid: AtomicI32::new(0),
                panel_visible: AtomicBool::new(false),
            });
            app.manage(state.clone());

            let handle = app.handle().clone();
            panel::init(&handle)?;
            windows::apply_theme(&handle, &theme);
            tray::init(&handle)?;
            if let Err(e) = shortcuts::register_all(&handle) {
                log::error!("{e}");
            }
            clipboard::monitor::start(handle.clone());
            start_maintenance(state, handle.clone());
            if !onboarding_done {
                windows::open_onboarding(&handle);
            }
            // Development aid: show the panel at startup (debug builds only).
            #[cfg(debug_assertions)]
            if std::env::var("CLIPSTACK_SHOW_PANEL").is_ok() {
                let h = handle.clone();
                std::thread::spawn(move || {
                    std::thread::sleep(Duration::from_millis(1500));
                    panel::show(&h);
                });
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_items,
            commands::get_item,
            commands::get_counts,
            commands::paste_item,
            commands::paste_many,
            commands::copy_item,
            commands::pin_item,
            commands::delete_items,
            commands::undo_delete,
            commands::update_item_text,
            commands::move_to_space,
            commands::set_item_tags,
            commands::run_action,
            commands::list_spaces,
            commands::create_space,
            commands::rename_space,
            commands::delete_space,
            commands::reorder_spaces,
            commands::list_tags,
            commands::create_tag,
            commands::update_tag,
            commands::delete_tag,
            commands::merge_tags,
            commands::list_source_apps,
            commands::get_settings,
            commands::set_setting,
            commands::set_shortcut,
            commands::check_accessibility,
            commands::request_accessibility,
            commands::pause_monitoring,
            commands::get_monitoring_state,
            commands::list_excluded_apps,
            commands::add_excluded_app,
            commands::remove_excluded_app,
            commands::export_data,
            commands::import_data,
            commands::clear_history,
            commands::hide_panel,
            commands::show_panel,
            commands::open_settings,
            commands::finish_onboarding,
            commands::open_url,
            commands::reveal_in_finder,
            commands::file_icon,
            commands::system_accent,
            commands::app_info,
        ])
        .build(tauri::generate_context!())
        .expect("error while building Clipstack")
        .run(|_app, event| {
            // Keep running in the menu bar when the Settings/Onboarding windows close.
            if let tauri::RunEvent::ExitRequested {
                api, code: None, ..
            } = event
            {
                api.prevent_exit();
            }
        });
}
