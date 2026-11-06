//! Global application state managed by Tauri.

use crate::db::queries::FullRow;
use crate::settings::Settings;
use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicIsize};
use std::sync::{Mutex, RwLock};

#[derive(Clone, Debug)]
pub struct Paths {
    pub root: PathBuf,
    pub db: PathBuf,
    pub images: PathBuf,
    pub thumbs: PathBuf,
    pub icons: PathBuf,
    pub favicons: PathBuf,
}

impl Paths {
    pub fn new(root: PathBuf) -> std::io::Result<Paths> {
        let p = Paths {
            db: root.join("clipstack.sqlite"),
            images: root.join("images"),
            thumbs: root.join("thumbs"),
            icons: root.join("icons"),
            favicons: root.join("favicons"),
            root,
        };
        for d in [&p.root, &p.images, &p.thumbs, &p.icons, &p.favicons] {
            std::fs::create_dir_all(d)?;
        }
        Ok(p)
    }
}

/// Rows removed by the last delete, kept in memory so `⌘Z` can restore them.
pub struct UndoBuffer {
    pub rows: Vec<(FullRow, Vec<i64>)>,
    pub at: std::time::Instant,
}

pub struct AppState {
    pub db: Mutex<Connection>,
    pub paths: Paths,
    pub settings: RwLock<Settings>,
    pub paused: AtomicBool,
    /// Change count produced by our own pasteboard writes; the monitor skips it.
    pub own_change_count: AtomicIsize,
    pub undo: Mutex<Option<UndoBuffer>>,
    /// pid of the app that was frontmost when the panel opened (paste target).
    pub prev_app_pid: AtomicI32,
    pub panel_visible: AtomicBool,
}

impl AppState {
    pub fn db(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.db.lock().unwrap_or_else(|e| e.into_inner())
    }
    pub fn settings(&self) -> Settings {
        self.settings
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }
}
