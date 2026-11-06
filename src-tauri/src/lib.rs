mod actions;
mod clipboard;
mod db;
mod panel;
mod paste;
mod permissions;
mod privacy;
mod settings;
mod shortcuts;
mod tray;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // Menu-bar only app: no Dock icon, no app menu (also enforced by LSUIElement in Info.plist).
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Clipstack");
}
