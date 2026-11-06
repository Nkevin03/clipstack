//! Side effects that run after a capture is stored (sound, OCR, link metadata).

use crate::models::Item;
use crate::settings::Settings;
use tauri::AppHandle;

pub fn after_capture(_app: &AppHandle, _settings: &Settings, _item: &Item, _inserted: bool) {}
