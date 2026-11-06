//! Side effects that run after a capture is stored (sound, OCR, link metadata, menu refresh).

use crate::models::{Item, Kind};
use crate::settings::Settings;
use tauri::AppHandle;

pub fn after_capture(app: &AppHandle, settings: &Settings, item: &Item, inserted: bool) {
    crate::tray::refresh(app);
    if !inserted {
        return;
    }
    if settings.sound_on_copy {
        crate::platform::play_copy_sound();
    }
    if item.kind == Kind::Image && settings.ocr_enabled {
        crate::ocr::spawn(app.clone(), item.id, item.image_path.clone());
    }
    if item.kind == Kind::Link && settings.fetch_link_meta {
        crate::linkmeta::spawn(app.clone(), item.id, item.preview.clone());
    }
}
