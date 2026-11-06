//! Backup (export/import), retention cleanup, orphan files and optional DB encryption.

use crate::db::queries::{self as q, FullRow};
use crate::models::{Space, Tag};
use crate::state::{AppState, Paths};
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::io::{Read, Write};
use std::path::Path;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Backup {
    pub format: String,
    pub version: u32,
    pub exported_at: i64,
    pub items: Vec<FullRow>,
    pub spaces: Vec<Space>,
    pub tags: Vec<Tag>,
    /// (item id, tag id) using the ids of this backup.
    pub item_tags: Vec<(i64, i64)>,
}

const FORMAT: &str = "clipstack-backup";

fn file_name(p: &str) -> Option<String> {
    Path::new(p)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
}

/// Write a zip with `backup.json` and the original images under `images/`.
pub fn export(conn: &Connection, dest: &Path) -> anyhow::Result<usize> {
    let items = q::all_full(conn)?;
    let mut st = conn.prepare("SELECT item_id, tag_id FROM item_tags")?;
    let item_tags = st
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let backup = Backup {
        format: FORMAT.into(),
        version: 1,
        exported_at: q::now_ms(),
        items,
        spaces: q::list_spaces(conn)?,
        tags: q::list_tags(conn)?,
        item_tags,
    };
    let file = std::fs::File::create(dest)?;
    let mut zip = zip::ZipWriter::new(file);
    let opts = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    zip.start_file("backup.json", opts)?;
    zip.write_all(serde_json::to_string(&backup)?.as_bytes())?;
    let stored =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    let mut added = HashSet::new();
    for it in &backup.items {
        if let Some(path) = &it.image_path
            && let (Some(name), Ok(bytes)) = (file_name(path), std::fs::read(path))
            && added.insert(name.clone())
        {
            zip.start_file(format!("images/{name}"), stored)?;
            zip.write_all(&bytes)?;
        }
    }
    zip.finish()?;
    Ok(backup.items.len())
}

/// Import a backup made by [`export`]. Existing content (same hash) is kept; returns imported count.
pub fn import(conn: &mut Connection, paths: &Paths, src: &Path) -> anyhow::Result<usize> {
    let file = std::fs::File::open(src)?;
    let mut zip = zip::ZipArchive::new(file)?;
    let backup: Backup = {
        let mut f = zip.by_name("backup.json")?;
        let mut s = String::new();
        f.read_to_string(&mut s)?;
        serde_json::from_str(&s)?
    };
    if backup.format != FORMAT {
        anyhow::bail!("not a Clipstack backup");
    }
    // Restore images first.
    for i in 0..zip.len() {
        let mut f = zip.by_index(i)?;
        let Some(name) = f.name()?.strip_prefix("images/").map(|s| s.to_string()) else {
            continue;
        };
        if name.is_empty() || name.contains('/') || name.contains("..") {
            continue;
        }
        let dest = paths.images.join(&name);
        if !dest.exists() {
            let mut buf = Vec::new();
            f.read_to_end(&mut buf)?;
            std::fs::write(&dest, buf)?;
        }
    }
    let tx = conn.transaction()?;
    let mut space_map: HashMap<i64, i64> = HashMap::new();
    let existing_spaces = q::list_spaces(&tx)?;
    for s in &backup.spaces {
        let id = match existing_spaces
            .iter()
            .find(|e| e.name.eq_ignore_ascii_case(&s.name))
        {
            Some(e) => e.id,
            None => q::create_space(&tx, &s.name, s.color.as_deref(), s.icon.as_deref())?,
        };
        space_map.insert(s.id, id);
    }
    let mut tag_map: HashMap<i64, i64> = HashMap::new();
    for t in &backup.tags {
        tag_map.insert(t.id, q::create_tag(&tx, &t.name, t.color.as_deref())?);
    }
    let mut item_map: HashMap<i64, i64> = HashMap::new();
    let mut imported = 0;
    for row in &backup.items {
        let exists: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM items WHERE content_hash = ?1)",
            [&row.content_hash],
            |r| r.get(0),
        )?;
        if exists {
            continue;
        }
        let mut r = row.clone();
        r.space_id = r.space_id.and_then(|s| space_map.get(&s).copied());
        r.image_path = r
            .image_path
            .as_deref()
            .and_then(file_name)
            .map(|n| paths.images.join(n).to_string_lossy().into_owned());
        if let Some(img) = &r.image_path {
            r.thumb_path = regenerate_thumb(img, &r.content_hash, paths).or(None);
        } else {
            r.thumb_path = None;
        }
        r.link_favicon_path = None;
        let new_id = q::insert_full(&tx, &r, false)?;
        item_map.insert(row.id, new_id);
        imported += 1;
    }
    for (item, tag) in &backup.item_tags {
        if let (Some(i), Some(t)) = (item_map.get(item), tag_map.get(tag)) {
            tx.execute(
                "INSERT OR IGNORE INTO item_tags (item_id, tag_id) VALUES (?1, ?2)",
                params![i, t],
            )?;
        }
    }
    tx.commit()?;
    Ok(imported)
}

fn regenerate_thumb(image: &str, hash: &str, paths: &Paths) -> Option<String> {
    let bytes = std::fs::read(image).ok()?;
    let fmt = Path::new(image)
        .extension()
        .and_then(|e| e.to_str())
        .and_then(crate::clipboard::pasteboard::ImageFormat::from_ext)?;
    crate::clipboard::images::save(&bytes, fmt, hash, &paths.images, &paths.thumbs)?.thumb
}

/// Retention + removal of files no longer referenced by any item (or the undo buffer).
pub fn cleanup(state: &AppState) -> anyhow::Result<usize> {
    let s = state.settings();
    let (deleted, referenced) = {
        let db = state.db();
        let n = q::cleanup(&db, q::now_ms(), s.max_items, s.retention_days)?;
        (n, q::referenced_files(&db)?)
    };
    let mut keep = referenced;
    if let Some(u) = state
        .undo
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .as_ref()
    {
        for (row, _) in &u.rows {
            keep.extend(row.image_path.clone());
            keep.extend(row.thumb_path.clone());
            keep.extend(row.link_favicon_path.clone());
        }
    }
    let mut removed_files = 0;
    for dir in [
        &state.paths.images,
        &state.paths.thumbs,
        &state.paths.favicons,
    ] {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for e in entries.flatten() {
                let p = e.path().to_string_lossy().into_owned();
                if !keep.contains(&p) && std::fs::remove_file(e.path()).is_ok() {
                    removed_files += 1;
                }
            }
        }
    }
    if deleted > 0 || removed_files > 0 {
        log::info!("cleanup: {deleted} items, {removed_files} files removed");
    }
    Ok(deleted)
}

// ---------------------------------------------------------------------------------------------
// Encryption (SQLCipher, key in the login Keychain)
// ---------------------------------------------------------------------------------------------

const KEYCHAIN_SERVICE: &str = "com.clipstack.app";
const KEYCHAIN_ACCOUNT: &str = "database-key";

pub fn is_plaintext_sqlite(path: &Path) -> bool {
    let mut header = [0u8; 16];
    match std::fs::File::open(path).and_then(|mut f| f.read_exact(&mut header)) {
        Ok(()) => &header == b"SQLite format 3\0",
        Err(_) => true, // missing/empty file → will be created as plaintext
    }
}

fn keychain_get() -> Option<String> {
    security_framework::passwords::get_generic_password(KEYCHAIN_SERVICE, KEYCHAIN_ACCOUNT)
        .ok()
        .and_then(|b| String::from_utf8(b).ok())
}

fn keychain_set(key: &str) -> anyhow::Result<()> {
    security_framework::passwords::set_generic_password(
        KEYCHAIN_SERVICE,
        KEYCHAIN_ACCOUNT,
        key.as_bytes(),
    )?;
    Ok(())
}

fn keychain_delete() {
    let _ =
        security_framework::passwords::delete_generic_password(KEYCHAIN_SERVICE, KEYCHAIN_ACCOUNT);
}

fn random_key_hex() -> anyhow::Result<String> {
    let mut b = [0u8; 32];
    std::fs::File::open("/dev/urandom")?.read_exact(&mut b)?;
    Ok(b.iter().map(|x| format!("{x:02x}")).collect())
}

fn apply_key(conn: &Connection, key_hex: &str) -> rusqlite::Result<()> {
    conn.execute_batch(&format!("PRAGMA key = \"x'{key_hex}'\";"))
}

/// Open the database, transparently using the Keychain key if the file is encrypted.
pub fn open_db(path: &Path) -> anyhow::Result<Connection> {
    if is_plaintext_sqlite(path) {
        return crate::db::open(path);
    }
    let key = keychain_get()
        .ok_or_else(|| anyhow::anyhow!("database is encrypted but the Keychain key is missing"))?;
    let mut conn = Connection::open(path)?;
    apply_key(&conn, &key)?;
    crate::db::configure(&conn)?;
    crate::db::migrations::run(&mut conn)?;
    Ok(conn)
}

/// Re-write the database encrypted (`enable`) or in clear text, swapping it in place.
pub fn set_encryption(state: &AppState, enable: bool) -> anyhow::Result<()> {
    let path = &state.paths.db;
    let currently_plain = is_plaintext_sqlite(path);
    if enable == !currently_plain {
        return Ok(());
    }
    let tmp = path.with_extension("sqlite.tmp");
    let _ = std::fs::remove_file(&tmp);
    let mut guard = state.db();
    guard.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")?;
    let new_key = if enable {
        Some(random_key_hex()?)
    } else {
        None
    };
    let attach_key = new_key
        .as_ref()
        .map(|k| format!("x'{k}'"))
        .unwrap_or_default();
    guard.execute(
        "ATTACH DATABASE ?1 AS target KEY ?2",
        params![tmp.to_string_lossy(), attach_key],
    )?;
    guard.query_row("SELECT sqlcipher_export('target')", [], |_| Ok(()))?;
    let version: i64 = guard.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    guard.execute_batch(&format!(
        "PRAGMA target.user_version = {version}; DETACH DATABASE target;"
    ))?;
    // Swap: replace connection with an in-memory placeholder while files move.
    let old = std::mem::replace(&mut *guard, Connection::open_in_memory()?);
    drop(old);
    for ext in ["sqlite-wal", "sqlite-shm"] {
        let _ = std::fs::remove_file(path.with_extension(ext));
    }
    if let Some(k) = &new_key {
        keychain_set(k)?;
    }
    std::fs::rename(&tmp, path)?;
    if !enable {
        keychain_delete();
    }
    *guard = open_db(path)?;
    log::info!(
        "database encryption {}",
        if enable { "enabled" } else { "disabled" }
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn export_import_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let paths = Paths::new(dir.path().join("data")).unwrap();
        let src = crate::db::open_memory();
        let n = q::NewItem {
            kind: Some(crate::models::Kind::Text),
            hash: "h1".into(),
            text: Some("hello".into()),
            ..Default::default()
        };
        let (id, _) = q::upsert_capture(&src, &n, 1).unwrap();
        let t = q::create_tag(&src, "work", Some("#f00")).unwrap();
        q::set_item_tags(&src, id, &[t]).unwrap();
        q::move_to_space(&src, &[id], Some(1)).unwrap();
        let zip = dir.path().join("b.zip");
        assert_eq!(export(&src, &zip).unwrap(), 1);

        let mut dst = crate::db::open_memory();
        assert_eq!(import(&mut dst, &paths, &zip).unwrap(), 1);
        assert_eq!(import(&mut dst, &paths, &zip).unwrap(), 0, "dedup by hash");
        let items = q::get_items(
            &dst,
            &crate::models::ItemQuery {
                tags: vec!["work".into()],
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(items.len(), 1);
        assert!(items[0].space_id.is_some());
    }

    #[test]
    fn sqlcipher_is_available() {
        let c = Connection::open_in_memory().unwrap();
        let v: String = c
            .query_row("PRAGMA cipher_version", [], |r| r.get(0))
            .unwrap();
        assert!(!v.is_empty());
    }
}
