//! Tauri commands (frontend → Rust).

use crate::db::queries as q;
use crate::models::*;
use crate::state::AppState;
use std::sync::Arc;
use tauri::State;

pub type Cmd<T> = Result<T, String>;
pub type St<'a> = State<'a, Arc<AppState>>;

pub fn err<E: std::fmt::Display>(e: E) -> String {
    e.to_string()
}

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
pub fn list_spaces(state: St) -> Cmd<Vec<Space>> {
    q::list_spaces(&state.db()).map_err(err)
}

#[tauri::command]
pub fn list_tags(state: St) -> Cmd<Vec<Tag>> {
    q::list_tags(&state.db()).map_err(err)
}

#[tauri::command]
pub fn list_source_apps(state: St) -> Cmd<Vec<SourceAppInfo>> {
    q::list_source_apps(&state.db(), 12).map_err(err)
}
