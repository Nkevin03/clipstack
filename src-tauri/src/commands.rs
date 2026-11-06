//! Tauri commands (frontend → Rust). See BRIEF §8.

use crate::actions::{self, Action, TextStats};
use crate::clipboard::{hash, kinds};
use crate::db::queries as q;
use crate::models::*;
use crate::paste::{self, PasteOutcome};
use crate::settings::Settings;
use crate::state::{AppState, UndoBuffer};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::sync::atomic::Ordering;
use tauri::{AppHandle, Emitter, Manager, State};
use ts_rs::TS;

pub type Cmd<T> = Result<T, String>;
pub type St<'a> = State<'a, Arc<AppState>>;

pub fn err<E: std::fmt::Display>(e: E) -> String {
    e.to_string()
}

fn changed(app: &AppHandle) {
    let _ = app.emit("items:changed", ());
    crate::tray::refresh(app);
}

// ---- items ------------------------------------------------------------------------------------

#[tauri::command]
pub fn get_items(state: St, query: ItemQuery) -> Cmd<Vec<Item>> {
    q::get_items(&state.db(), &query).map_err(err)
}

#[tauri::command]
pub fn get_item(state: St, id: i64) -> Cmd<Option<ItemDetail>> {
    q::get_item_detail(&state.db(), id).map_err(err)
}

#[tauri::command]
pub fn get_counts(state: St) -> Cmd<Counts> {
    q::counts(&state.db()).map_err(err)
}

#[tauri::command]
pub fn paste_item(app: AppHandle, id: i64, plain: bool) -> Cmd<PasteOutcome> {
    paste::paste_ids(&app, &[id], plain).map_err(err)
}

#[tauri::command]
pub fn paste_many(
    app: AppHandle,
    state: St,
    ids: Vec<i64>,
    separator: Option<String>,
    plain: bool,
) -> Cmd<PasteOutcome> {
    if let Some(sep) = separator {
        let s = state.settings();
        if s.multi_separator != sep {
            let new = s
                .with("multiSeparator", serde_json::json!(sep))
                .map_err(err)?;
            *state.settings.write().unwrap_or_else(|e| e.into_inner()) = new;
        }
    }
    paste::paste_ids(&app, &ids, plain).map_err(err)
}

#[tauri::command]
pub fn copy_item(app: AppHandle, state: St, ids: Vec<i64>, plain: bool) -> Cmd<()> {
    paste::copy_ids(&state, &ids, plain).map_err(err)?;
    changed(&app);
    Ok(())
}

#[tauri::command]
pub fn pin_item(app: AppHandle, state: St, ids: Vec<i64>, value: bool) -> Cmd<()> {
    q::set_pinned(&state.db(), &ids, value).map_err(err)?;
    changed(&app);
    Ok(())
}

#[tauri::command]
pub fn delete_items(app: AppHandle, state: St, ids: Vec<i64>) -> Cmd<usize> {
    let n = {
        let db = state.db();
        let mut rows = Vec::new();
        for r in q::get_full_many(&db, &ids).map_err(err)? {
            let tags = q::tag_ids_for(&db, r.id).map_err(err)?;
            rows.push((r, tags));
        }
        let n = q::delete_items(&db, &ids).map_err(err)?;
        *state.undo.lock().unwrap_or_else(|e| e.into_inner()) = Some(UndoBuffer {
            rows,
            at: std::time::Instant::now(),
        });
        n
    };
    let _ = app.emit("item:deleted", &ids);
    changed(&app);
    Ok(n)
}

#[tauri::command]
pub fn undo_delete(app: AppHandle, state: St) -> Cmd<Vec<i64>> {
    let Some(buf) = state.undo.lock().unwrap_or_else(|e| e.into_inner()).take() else {
        return Ok(vec![]);
    };
    let mut restored = Vec::new();
    {
        let db = state.db();
        for (row, tags) in &buf.rows {
            // Skip if the same content was captured again meanwhile.
            if q::insert_full(&db, row, true).is_ok() {
                let _ = q::set_item_tags(&db, row.id, tags);
                restored.push(row.id);
            }
        }
    }
    changed(&app);
    Ok(restored)
}

#[tauri::command]
pub fn update_item_text(app: AppHandle, state: St, id: i64, text: String) -> Cmd<Option<Item>> {
    let (kind, lang) = kinds::detect(&text, false);
    let db = state.db();
    q::update_text(&db, id, &text, kind, lang, &hash::text_hash(&text)).map_err(|e| {
        if e.to_string().contains("UNIQUE") {
            "duplicate".to_string()
        } else {
            err(e)
        }
    })?;
    let item = q::get_item(&db, id).map_err(err)?;
    drop(db);
    changed(&app);
    Ok(item)
}

#[tauri::command]
pub fn move_to_space(app: AppHandle, state: St, ids: Vec<i64>, space_id: Option<i64>) -> Cmd<()> {
    q::move_to_space(&state.db(), &ids, space_id).map_err(err)?;
    changed(&app);
    Ok(())
}

#[tauri::command]
pub fn set_item_tags(app: AppHandle, state: St, id: i64, tag_ids: Vec<i64>) -> Cmd<()> {
    q::set_item_tags(&state.db(), id, &tag_ids).map_err(err)?;
    changed(&app);
    Ok(())
}

// ---- actions ----------------------------------------------------------------------------------

#[derive(Serialize, Deserialize, TS, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum ActionMode {
    /// Paste the result into the previous app (not stored).
    Paste,
    /// Store the result as a new history item.
    Save,
    /// Only compute (preview / stats).
    Preview,
}

#[derive(Serialize, Deserialize, TS, Clone, Debug)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ActionResult {
    pub text: String,
    pub stats: TextStats,
    pub new_item_id: Option<i64>,
}

#[tauri::command]
pub fn run_action(
    app: AppHandle,
    state: St,
    id: i64,
    action: Action,
    mode: ActionMode,
) -> Cmd<ActionResult> {
    let row = q::get_full(&state.db(), id)
        .map_err(err)?
        .ok_or("item not found")?;
    let input = row.text_content.clone().unwrap_or_default();
    let text = actions::apply(action, &input)?;
    let stats = actions::stats(&text);
    let mut new_item_id = None;
    match mode {
        ActionMode::Preview => {}
        ActionMode::Paste if action != Action::Count => {
            paste::paste_text(&app, &text).map_err(err)?;
        }
        ActionMode::Save if action != Action::Count => {
            let (kind, language) = kinds::detect(&text, false);
            let n = q::NewItem {
                kind: Some(kind),
                hash: hash::text_hash(&text),
                byte_size: text.len() as i64,
                text: Some(text.clone()),
                language,
                source: row
                    .source_bundle_id
                    .as_ref()
                    .and_then(|b| q::get_source_app(&state.db(), b).ok().flatten()),
                ..Default::default()
            };
            let (nid, _) = q::upsert_capture(&state.db(), &n, q::now_ms()).map_err(err)?;
            new_item_id = Some(nid);
            changed(&app);
        }
        _ => {}
    }
    Ok(ActionResult {
        text,
        stats,
        new_item_id,
    })
}

// ---- spaces & tags ----------------------------------------------------------------------------

#[tauri::command]
pub fn list_spaces(state: St) -> Cmd<Vec<Space>> {
    q::list_spaces(&state.db()).map_err(err)
}

#[tauri::command]
pub fn create_space(
    app: AppHandle,
    state: St,
    name: String,
    color: Option<String>,
    icon: Option<String>,
) -> Cmd<i64> {
    let id = q::create_space(&state.db(), name.trim(), color.as_deref(), icon.as_deref())
        .map_err(err)?;
    changed(&app);
    Ok(id)
}

#[tauri::command]
pub fn rename_space(
    app: AppHandle,
    state: St,
    id: i64,
    name: String,
    color: Option<String>,
    icon: Option<String>,
) -> Cmd<()> {
    q::update_space(
        &state.db(),
        id,
        name.trim(),
        color.as_deref(),
        icon.as_deref(),
    )
    .map_err(err)?;
    changed(&app);
    Ok(())
}

#[tauri::command]
pub fn delete_space(app: AppHandle, state: St, id: i64) -> Cmd<()> {
    q::delete_space(&state.db(), id).map_err(err)?;
    changed(&app);
    Ok(())
}

#[tauri::command]
pub fn reorder_spaces(app: AppHandle, state: St, ids: Vec<i64>) -> Cmd<()> {
    q::reorder_spaces(&state.db(), &ids).map_err(err)?;
    changed(&app);
    Ok(())
}

#[tauri::command]
pub fn list_tags(state: St) -> Cmd<Vec<Tag>> {
    q::list_tags(&state.db()).map_err(err)
}

#[tauri::command]
pub fn create_tag(app: AppHandle, state: St, name: String, color: Option<String>) -> Cmd<i64> {
    let name = name.trim();
    if name.is_empty() {
        return Err("empty name".into());
    }
    let id = q::create_tag(&state.db(), name, color.as_deref()).map_err(err)?;
    changed(&app);
    Ok(id)
}

#[tauri::command]
pub fn update_tag(
    app: AppHandle,
    state: St,
    id: i64,
    name: String,
    color: Option<String>,
) -> Cmd<()> {
    q::update_tag(&state.db(), id, name.trim(), color.as_deref()).map_err(err)?;
    changed(&app);
    Ok(())
}

#[tauri::command]
pub fn delete_tag(app: AppHandle, state: St, id: i64) -> Cmd<()> {
    q::delete_tag(&state.db(), id).map_err(err)?;
    changed(&app);
    Ok(())
}

#[tauri::command]
pub fn merge_tags(app: AppHandle, state: St, from: Vec<i64>, into: i64) -> Cmd<()> {
    q::merge_tags(&state.db(), &from, into).map_err(err)?;
    changed(&app);
    Ok(())
}

#[tauri::command]
pub fn list_source_apps(state: St) -> Cmd<Vec<SourceAppInfo>> {
    q::list_source_apps(&state.db(), 12).map_err(err)
}

// ---- settings ---------------------------------------------------------------------------------

#[tauri::command]
pub fn get_settings(state: St) -> Cmd<Settings> {
    Ok(state.settings())
}

#[tauri::command]
pub fn set_setting(
    app: AppHandle,
    state: St,
    key: String,
    value: serde_json::Value,
) -> Cmd<Settings> {
    let old = state.settings();
    let new = old.with(&key, value.clone()).map_err(err)?;
    match key.as_str() {
        "shortcut" | "pauseShortcut" | "sequenceShortcut" => {
            if let Some(acc) = value.as_str().filter(|s| !s.is_empty()) {
                crate::shortcuts::parse(acc)?;
            }
        }
        "launchAtLogin" => {
            use tauri_plugin_autostart::ManagerExt;
            let al = app.autolaunch();
            let r = if new.launch_at_login {
                al.enable()
            } else {
                al.disable()
            };
            r.map_err(err)?;
        }
        "encryptDb" => crate::data::set_encryption(&state, new.encrypt_db).map_err(err)?,
        _ => {}
    }
    crate::settings::persist(&state.db(), &key, &value).map_err(err)?;
    *state.settings.write().unwrap_or_else(|e| e.into_inner()) = new.clone();
    if matches!(
        key.as_str(),
        "shortcut" | "pauseShortcut" | "sequenceShortcut"
    ) && let Err(e) = crate::shortcuts::register_all(&app)
    {
        // Roll back to the previous working shortcut.
        let _ = crate::settings::persist(
            &state.db(),
            &key,
            &serde_json::to_value(match key.as_str() {
                "shortcut" => serde_json::json!(old.shortcut),
                "pauseShortcut" => serde_json::json!(old.pause_shortcut),
                _ => serde_json::json!(old.sequence_shortcut),
            })
            .unwrap(),
        );
        *state.settings.write().unwrap_or_else(|e| e.into_inner()) = old;
        let _ = crate::shortcuts::register_all(&app);
        return Err(e);
    }
    if matches!(key.as_str(), "layout" | "cardSize" | "cursorPopup") {
        crate::panel::relayout(&app);
    }
    if key == "sequenceShortcut" {
        crate::tray::refresh(&app);
    }
    if matches!(
        key.as_str(),
        "sequenceOrder" | "sequenceCycle" | "pastePeek"
    ) {
        crate::paste_stack::reconfigure(&app);
    }
    if key == "language" {
        crate::tray::refresh(&app);
    }
    if key == "theme" {
        crate::windows::apply_theme(&app, &new.theme);
    }
    if matches!(key.as_str(), "maxItems" | "retentionDays") {
        let _ = crate::data::cleanup(&state);
        changed(&app);
    }
    let _ = app.emit("settings:changed", &new);
    Ok(new)
}

#[tauri::command]
pub fn set_shortcut(app: AppHandle, state: St, accelerator: String) -> Cmd<Settings> {
    set_setting(
        app,
        state,
        "shortcut".into(),
        serde_json::json!(accelerator),
    )
}

// ---- permissions, monitoring, privacy ---------------------------------------------------------

#[tauri::command]
pub fn check_accessibility() -> bool {
    crate::permissions::is_trusted()
}

#[tauri::command]
pub fn request_accessibility(app: AppHandle) -> bool {
    let trusted = crate::permissions::request();
    if !trusted {
        use tauri_plugin_opener::OpenerExt;
        let _ = app
            .opener()
            .open_url(crate::permissions::SETTINGS_URL, None::<&str>);
    }
    trusted
}

#[tauri::command]
pub fn pause_monitoring(app: AppHandle, value: bool) {
    crate::tray::set_paused(&app, value);
}

#[tauri::command]
pub fn get_monitoring_state(state: St) -> bool {
    state.paused.load(Ordering::SeqCst)
}

#[tauri::command]
pub fn list_excluded_apps(state: St) -> Cmd<Vec<ExcludedApp>> {
    q::list_excluded(&state.db()).map_err(err)
}

/// Add an app by bundle id, or by path to a `.app` bundle (from the file picker).
#[tauri::command]
pub fn add_excluded_app(
    state: St,
    bundle_id: Option<String>,
    path: Option<String>,
    name: Option<String>,
) -> Cmd<ExcludedApp> {
    let (id, nm) = match (bundle_id, path) {
        (Some(b), _) if !b.trim().is_empty() => (b.trim().to_string(), name),
        (_, Some(p)) => crate::platform::bundle_info(&p).ok_or("not an application bundle")?,
        _ => return Err("missing app".into()),
    };
    q::add_excluded(&state.db(), &id, nm.as_deref()).map_err(err)?;
    Ok(ExcludedApp {
        bundle_id: id,
        name: nm,
    })
}

#[tauri::command]
pub fn remove_excluded_app(state: St, bundle_id: String) -> Cmd<()> {
    q::remove_excluded(&state.db(), &bundle_id).map_err(err)
}

// ---- data -------------------------------------------------------------------------------------

#[tauri::command]
pub async fn export_data(app: AppHandle, path: String) -> Cmd<usize> {
    let state = app.state::<Arc<AppState>>().inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        crate::data::export(&state.db(), std::path::Path::new(&path))
    })
    .await
    .map_err(err)?
    .map_err(err)
}

#[tauri::command]
pub async fn import_data(app: AppHandle, path: String) -> Cmd<usize> {
    let state = app.state::<Arc<AppState>>().inner().clone();
    let n = tauri::async_runtime::spawn_blocking(move || {
        let mut db = state.db();
        crate::data::import(&mut db, &state.paths, std::path::Path::new(&path))
    })
    .await
    .map_err(err)?
    .map_err(err)?;
    changed(&app);
    Ok(n)
}

#[tauri::command]
pub fn clear_history(app: AppHandle, state: St, keep_pinned: bool) -> Cmd<usize> {
    let n = q::clear_history(&state.db(), keep_pinned).map_err(err)?;
    *state.undo.lock().unwrap_or_else(|e| e.into_inner()) = None;
    let _ = crate::data::cleanup(&state);
    changed(&app);
    Ok(n)
}

// ---- windows & misc ---------------------------------------------------------------------------

#[tauri::command]
pub fn hide_panel(app: AppHandle) {
    crate::panel::hide(&app);
}

#[tauri::command]
pub fn show_panel(app: AppHandle) {
    crate::panel::show(&app);
}

#[tauri::command]
pub fn open_settings(app: AppHandle, section: Option<String>) {
    crate::windows::open_settings(&app, section.as_deref());
}

#[tauri::command]
pub fn finish_onboarding(app: AppHandle, state: St) -> Cmd<()> {
    crate::settings::persist(&state.db(), "onboardingDone", &serde_json::json!(true))
        .map_err(err)?;
    let s = state
        .settings()
        .with("onboardingDone", serde_json::json!(true))
        .map_err(err)?;
    *state.settings.write().unwrap_or_else(|e| e.into_inner()) = s;
    if let Some(w) = app.get_webview_window(crate::windows::ONBOARDING) {
        let _ = w.close();
    }
    Ok(())
}

#[tauri::command]
pub fn open_url(app: AppHandle, url: String) -> Cmd<()> {
    use tauri_plugin_opener::OpenerExt;
    if !crate::clipboard::kinds::is_link(&url) {
        return Err("not a link".into());
    }
    crate::panel::hide(&app);
    app.opener().open_url(url, None::<&str>).map_err(err)
}

#[tauri::command]
pub fn reveal_in_finder(app: AppHandle, state: St, id: i64) -> Cmd<()> {
    let item = q::get_item(&state.db(), id)
        .map_err(err)?
        .ok_or("item not found")?;
    let paths: Vec<String> = if !item.file_paths.is_empty() {
        item.file_paths
    } else {
        item.image_path.into_iter().collect()
    };
    crate::panel::hide(&app);
    let handle = app.clone();
    app.run_on_main_thread(move || {
        crate::platform::reveal_in_finder(&paths);
        drop(handle);
    })
    .map_err(err)
}

#[tauri::command]
pub fn file_icon(state: St, path: String) -> Option<String> {
    crate::platform::file_icon(&path, &state.paths.icons).map(|p| p.to_string_lossy().into_owned())
}

#[tauri::command]
pub fn system_accent() -> Option<String> {
    crate::platform::system_accent()
}

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AppInfo {
    pub version: String,
    pub data_dir: String,
    pub encrypted: bool,
    pub accessibility: bool,
    pub system_language: String,
}

#[tauri::command]
pub fn app_info(app: AppHandle, state: St) -> AppInfo {
    AppInfo {
        version: app.package_info().version.to_string(),
        data_dir: state.paths.root.to_string_lossy().into_owned(),
        encrypted: !crate::data::is_plaintext_sqlite(&state.paths.db),
        accessibility: crate::permissions::is_trusted(),
        system_language: crate::platform::system_language().into(),
    }
}

// ---- copy sequence ----------------------------------------------------------------------------

#[tauri::command]
pub fn toggle_sequence(app: AppHandle) {
    crate::paste_stack::toggle_session(&app);
}

#[tauri::command]
pub fn stop_sequence(app: AppHandle) {
    crate::paste_stack::stop_session(&app);
}

#[tauri::command]
pub fn sequence_state(state: St) -> crate::paste_stack::SequenceState {
    crate::paste_stack::snapshot(&state, false)
}

#[tauri::command]
pub fn sequence_remove(app: AppHandle, id: i64) {
    crate::paste_stack::remove_item(&app, id);
}

#[tauri::command]
pub fn peek_state(state: St) -> crate::paste_stack::PeekState {
    crate::paste_stack::peek_state(&state)
}

/// Click on an item of the paste peek: paste it in place of what was just pasted.
#[tauri::command]
pub fn peek_paste(app: AppHandle, id: i64) -> Cmd<PasteOutcome> {
    crate::paste_stack::hide_peek(&app);
    paste::paste_ids(&app, &[id], false).map_err(err)
}

#[tauri::command]
pub fn hide_peek(app: AppHandle) {
    crate::paste_stack::hide_peek(&app);
}

#[tauri::command]
pub fn resize_hud(app: AppHandle, height: f64) {
    crate::hud::resize(&app, height);
}
