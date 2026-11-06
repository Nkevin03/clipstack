mod actions;
mod clipboard;
mod commands;
mod db;
mod hooks;
mod models;
mod panel;
mod paste;
mod permissions;
mod privacy;
mod settings;
mod shortcuts;
mod state;
mod tray;

use state::{AppState, Paths};
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicIsize};
use std::sync::{Arc, Mutex, RwLock};
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("clipstack_lib=info"),
    )
    .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // Menu-bar only app: no Dock icon, no app menu (also enforced by LSUIElement in Info.plist).
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let paths = Paths::new(app.path().app_data_dir()?)?;
            let conn = db::open(&paths.db)?;
            let settings = settings::Settings::load(&conn)?;
            log::info!("data dir: {}", paths.root.display());
            app.manage(Arc::new(AppState {
                db: Mutex::new(conn),
                paths,
                settings: RwLock::new(settings),
                paused: AtomicBool::new(false),
                own_change_count: AtomicIsize::new(-1),
                undo: Mutex::new(None),
                prev_app_pid: AtomicI32::new(0),
                panel_visible: AtomicBool::new(false),
            }));
            clipboard::monitor::start(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_items,
            commands::get_item,
            commands::get_counts,
            commands::list_spaces,
            commands::list_tags,
            commands::list_source_apps,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Clipstack");
}
