//! All SQL used by the app. Every function takes a `&Connection` so it can be unit-tested
//! against an in-memory database.

use crate::models::*;
use rusqlite::types::Value;
use rusqlite::{Connection, OptionalExtension, Row as SqlRow, params, params_from_iter};
use serde::{Deserialize, Serialize};

pub type Res<T> = rusqlite::Result<T>;

pub fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

/// A complete `items` row. Used for paste, undo, export and import.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct FullRow {
    pub id: i64,
    pub kind: String,
    pub content_hash: String,
    pub text_content: Option<String>,
    #[serde(with = "b64_opt")]
    pub rich_rtf: Option<Vec<u8>>,
    pub rich_html: Option<String>,
    pub file_paths: Option<String>,
    pub image_path: Option<String>,
    pub thumb_path: Option<String>,
    pub width: Option<i64>,
    pub height: Option<i64>,
    pub byte_size: Option<i64>,
    pub source_bundle_id: Option<String>,
    pub source_app_name: Option<String>,
    pub source_color: Option<String>,
    pub link_title: Option<String>,
    pub link_favicon_path: Option<String>,
    pub language: Option<String>,
    pub ocr_text: Option<String>,
    pub pinned: bool,
    pub space_id: Option<i64>,
    pub use_count: i64,
    pub created_at: i64,
    pub last_used_at: i64,
    pub expires_at: Option<i64>,
}

mod b64_opt {
    use base64::Engine;
    use serde::{Deserialize, Deserializer, Serializer};
    pub fn serialize<S: Serializer>(v: &Option<Vec<u8>>, s: S) -> Result<S::Ok, S::Error> {
        match v {
            Some(b) => s.serialize_some(&base64::engine::general_purpose::STANDARD.encode(b)),
            None => s.serialize_none(),
        }
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<Vec<u8>>, D::Error> {
        let s: Option<String> = Option::deserialize(d)?;
        s.map(|s| {
            base64::engine::general_purpose::STANDARD
                .decode(s)
                .map_err(serde::de::Error::custom)
        })
        .transpose()
    }
}

const FULL_COLS: &str = "id, kind, content_hash, text_content, rich_rtf, rich_html, file_paths, \
    image_path, thumb_path, width, height, byte_size, source_bundle_id, source_app_name, \
    source_color, link_title, link_favicon_path, language, ocr_text, pinned, space_id, use_count, \
    created_at, last_used_at, expires_at";

fn full_from_row(r: &SqlRow) -> Res<FullRow> {
    Ok(FullRow {
        id: r.get(0)?,
        kind: r.get(1)?,
        content_hash: r.get(2)?,
        text_content: r.get(3)?,
        rich_rtf: r.get(4)?,
        rich_html: r.get(5)?,
        file_paths: r.get(6)?,
        image_path: r.get(7)?,
        thumb_path: r.get(8)?,
        width: r.get(9)?,
        height: r.get(10)?,
        byte_size: r.get(11)?,
        source_bundle_id: r.get(12)?,
        source_app_name: r.get(13)?,
        source_color: r.get(14)?,
        link_title: r.get(15)?,
        link_favicon_path: r.get(16)?,
        language: r.get(17)?,
        ocr_text: r.get(18)?,
        pinned: r.get::<_, i64>(19)? != 0,
        space_id: r.get(20)?,
        use_count: r.get(21)?,
        created_at: r.get(22)?,
        last_used_at: r.get(23)?,
        expires_at: r.get(24)?,
    })
}

/// Insert a row keeping its id when `keep_id` (undo/import); otherwise a new id is assigned.
pub fn insert_full(conn: &Connection, row: &FullRow, keep_id: bool) -> Res<i64> {
    let id: Option<i64> = if keep_id { Some(row.id) } else { None };
    conn.execute(
        &format!("INSERT INTO items ({FULL_COLS}) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22,?23,?24,?25)"),
        params![
            id, row.kind, row.content_hash, row.text_content, row.rich_rtf, row.rich_html,
            row.file_paths, row.image_path, row.thumb_path, row.width, row.height, row.byte_size,
            row.source_bundle_id, row.source_app_name, row.source_color, row.link_title,
            row.link_favicon_path, row.language, row.ocr_text, row.pinned as i64, row.space_id,
            row.use_count, row.created_at, row.last_used_at, row.expires_at
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn get_full(conn: &Connection, id: i64) -> Res<Option<FullRow>> {
    conn.query_row(
        &format!("SELECT {FULL_COLS} FROM items WHERE id = ?1"),
        [id],
        full_from_row,
    )
    .optional()
}

pub fn get_full_many(conn: &Connection, ids: &[i64]) -> Res<Vec<FullRow>> {
    let mut out = Vec::with_capacity(ids.len());
    for id in ids {
        if let Some(r) = get_full(conn, *id)? {
            out.push(r);
        }
    }
    Ok(out)
}

pub fn all_full(conn: &Connection) -> Res<Vec<FullRow>> {
    let mut st = conn.prepare(&format!("SELECT {FULL_COLS} FROM items ORDER BY id"))?;
    let rows = st.query_map([], full_from_row)?;
    rows.collect()
}

/// New capture coming from the clipboard monitor.
#[derive(Clone, Debug, Default)]
pub struct NewItem {
    pub kind: Option<Kind>,
    pub hash: String,
    pub text: Option<String>,
    pub rtf: Option<Vec<u8>>,
    pub html: Option<String>,
    pub file_paths: Vec<String>,
    pub image_path: Option<String>,
    pub thumb_path: Option<String>,
    pub width: Option<i64>,
    pub height: Option<i64>,
    pub byte_size: i64,
    pub source: Option<SourceApp>,
    pub language: Option<String>,
    pub expires_at: Option<i64>,
}

/// Insert a capture, or — if the same content already exists — move it to the top.
/// Returns `(id, inserted)`.
pub fn upsert_capture(conn: &Connection, n: &NewItem, now: i64) -> Res<(i64, bool)> {
    let existing: Option<i64> = conn
        .query_row(
            "SELECT id FROM items WHERE content_hash = ?1",
            [&n.hash],
            |r| r.get(0),
        )
        .optional()?;
    if let Some(id) = existing {
        conn.execute(
            "UPDATE items SET last_used_at = ?2,
                rich_rtf = COALESCE(rich_rtf, ?3), rich_html = COALESCE(rich_html, ?4),
                expires_at = CASE WHEN ?5 IS NULL THEN expires_at ELSE ?5 END
             WHERE id = ?1",
            params![id, now, n.rtf, n.html, n.expires_at],
        )?;
        return Ok((id, false));
    }
    let kind = n.kind.unwrap_or(Kind::Text);
    let row = FullRow {
        id: 0,
        kind: kind.as_str().into(),
        content_hash: n.hash.clone(),
        text_content: n.text.clone(),
        rich_rtf: n.rtf.clone(),
        rich_html: n.html.clone(),
        file_paths: if n.file_paths.is_empty() {
            None
        } else {
            Some(serde_json::to_string(&n.file_paths).unwrap_or_default())
        },
        image_path: n.image_path.clone(),
        thumb_path: n.thumb_path.clone(),
        width: n.width,
        height: n.height,
        byte_size: Some(n.byte_size),
        source_bundle_id: n.source.as_ref().map(|s| s.bundle_id.clone()),
        source_app_name: n.source.as_ref().map(|s| s.name.clone()),
        source_color: n.source.as_ref().and_then(|s| s.color.clone()),
        language: n.language.clone(),
        created_at: now,
        last_used_at: now,
        expires_at: n.expires_at,
        ..Default::default()
    };
    let id = insert_full(conn, &row, false)?;
    Ok((id, true))
}

// ---------------------------------------------------------------------------------------------
// Listing & search
// ---------------------------------------------------------------------------------------------

const LIST_COLS: &str = "i.id, i.kind, substr(i.text_content, 1, 2000), length(i.text_content), \
    (i.rich_rtf IS NOT NULL OR i.rich_html IS NOT NULL), i.file_paths, i.image_path, i.thumb_path, \
    i.width, i.height, i.byte_size, i.source_bundle_id, i.source_app_name, \
    COALESCE(i.source_color, sa.color), sa.icon_path, i.link_title, i.link_favicon_path, i.language, \
    i.pinned, i.space_id, i.use_count, i.created_at, i.last_used_at, i.expires_at, \
    (SELECT group_concat(tag_id) FROM item_tags WHERE item_id = i.id)";

const LIST_FROM: &str =
    "FROM items i LEFT JOIN source_apps sa ON sa.bundle_id = i.source_bundle_id";

fn item_from_row(r: &SqlRow) -> Res<Item> {
    let kind: String = r.get(1)?;
    let files: Option<String> = r.get(5)?;
    let tags: Option<String> = r.get(24)?;
    Ok(Item {
        id: r.get(0)?,
        kind: Kind::parse(&kind).unwrap_or(Kind::Text),
        preview: r.get::<_, Option<String>>(2)?.unwrap_or_default(),
        char_count: r.get::<_, Option<i64>>(3)?.unwrap_or(0),
        has_rich: r.get::<_, i64>(4)? != 0,
        file_paths: files
            .and_then(|f| serde_json::from_str(&f).ok())
            .unwrap_or_default(),
        image_path: r.get(6)?,
        thumb_path: r.get(7)?,
        width: r.get(8)?,
        height: r.get(9)?,
        byte_size: r.get::<_, Option<i64>>(10)?.unwrap_or(0),
        source_bundle_id: r.get(11)?,
        source_app_name: r.get(12)?,
        source_color: r.get(13)?,
        source_icon_path: r.get(14)?,
        link_title: r.get(15)?,
        link_favicon_path: r.get(16)?,
        language: r.get(17)?,
        pinned: r.get::<_, i64>(18)? != 0,
        space_id: r.get(19)?,
        use_count: r.get(20)?,
        created_at: r.get(21)?,
        last_used_at: r.get(22)?,
        expires_at: r.get(23)?,
        tag_ids: tags
            .map(|t| t.split(',').filter_map(|x| x.parse().ok()).collect())
            .unwrap_or_default(),
    })
}

/// Build an FTS5 MATCH expression from free text: every token becomes a quoted prefix query,
/// tokens are AND-ed. Returns `None` if there is nothing searchable.
pub fn fts_query(text: &str) -> Option<String> {
    let tokens: Vec<String> = text
        .split(|c: char| c.is_whitespace())
        .map(|t| {
            t.chars()
                .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-' || *c == '.' || *c == '@')
                .collect::<String>()
        })
        .flat_map(|t| {
            // FTS5's unicode61 tokenizer splits on punctuation; mirror that so prefixes work.
            t.split(|c: char| !c.is_alphanumeric())
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string())
                .collect::<Vec<_>>()
        })
        .collect();
    if tokens.is_empty() {
        return None;
    }
    Some(
        tokens
            .iter()
            .map(|t| format!("\"{}\"*", t.replace('"', "")))
            .collect::<Vec<_>>()
            .join(" "),
    )
}

pub fn get_items(conn: &Connection, q: &ItemQuery) -> Res<Vec<Item>> {
    // Page over ids first (index-only), then fetch display columns for that page only.
    let mut sql =
        String::from("SELECT i.id FROM items i WHERE (i.expires_at IS NULL OR i.expires_at > ?)");
    let mut args: Vec<Value> = vec![Value::Integer(now_ms())];

    let text = q.text.trim();
    if !text.is_empty() {
        let fts = fts_query(text);
        let mut parts = Vec::new();
        if let Some(f) = fts {
            parts.push("i.id IN (SELECT rowid FROM items_fts WHERE items_fts MATCH ?)".to_string());
            args.push(Value::Text(f));
        }
        // Tag names are searchable too.
        parts.push("EXISTS (SELECT 1 FROM item_tags it JOIN tags t ON t.id = it.tag_id WHERE it.item_id = i.id AND t.name LIKE ?)".into());
        args.push(Value::Text(format!("{}%", text)));
        sql.push_str(&format!(" AND ({})", parts.join(" OR ")));
    }
    if !q.kinds.is_empty() {
        sql.push_str(&format!(" AND i.kind IN ({})", placeholders(q.kinds.len())));
        args.extend(q.kinds.iter().map(|k| Value::Text(k.as_str().into())));
    }
    if !q.apps.is_empty() {
        sql.push_str(&format!(
            " AND i.source_bundle_id IN ({})",
            placeholders(q.apps.len())
        ));
        args.extend(q.apps.iter().map(|a| Value::Text(a.clone())));
    }
    for name in &q.app_names {
        sql.push_str(" AND (i.source_app_name LIKE ? OR i.source_bundle_id LIKE ?)");
        let like = format!("%{}%", name);
        args.push(Value::Text(like.clone()));
        args.push(Value::Text(like));
    }
    for tag in &q.tags {
        sql.push_str(" AND EXISTS (SELECT 1 FROM item_tags it JOIN tags t ON t.id = it.tag_id WHERE it.item_id = i.id AND t.name = ? COLLATE NOCASE)");
        args.push(Value::Text(tag.clone()));
    }
    if let Some(space) = q.space_id {
        sql.push_str(" AND i.space_id = ?");
        args.push(Value::Integer(space));
    }
    for name in &q.space_names {
        sql.push_str(" AND i.space_id IN (SELECT id FROM spaces WHERE name LIKE ?)");
        args.push(Value::Text(format!("{}%", name)));
    }
    if let Some(p) = q.pinned {
        sql.push_str(" AND i.pinned = ?");
        args.push(Value::Integer(p as i64));
    }
    if let Some(since) = q.since {
        sql.push_str(" AND i.last_used_at >= ?");
        args.push(Value::Integer(since));
    }
    let order = match q.sort {
        Sort::Recent => " ORDER BY i.last_used_at DESC, i.id DESC",
        Sort::MostUsed => " ORDER BY i.use_count DESC, i.last_used_at DESC, i.id DESC",
        Sort::PinnedFirst => " ORDER BY i.pinned DESC, i.last_used_at DESC, i.id DESC",
    };
    sql.push_str(order);
    let limit = if q.limit <= 0 { 60 } else { q.limit.min(500) };
    sql.push_str(" LIMIT ? OFFSET ?");
    let sql = format!(
        "SELECT {LIST_COLS} FROM ({sql}) page JOIN items i ON i.id = page.id {}{order}",
        &LIST_FROM["FROM items i ".len()..]
    );
    args.push(Value::Integer(limit));
    args.push(Value::Integer(q.offset.max(0)));

    let mut st = conn.prepare_cached(&sql)?;
    let rows = st.query_map(params_from_iter(args), item_from_row)?;
    rows.collect()
}

fn placeholders(n: usize) -> String {
    vec!["?"; n].join(",")
}

pub fn get_item(conn: &Connection, id: i64) -> Res<Option<Item>> {
    conn.query_row(
        &format!("SELECT {LIST_COLS} {LIST_FROM} WHERE i.id = ?1"),
        [id],
        item_from_row,
    )
    .optional()
}

pub fn get_item_detail(conn: &Connection, id: i64) -> Res<Option<ItemDetail>> {
    let Some(item) = get_item(conn, id)? else {
        return Ok(None);
    };
    let (text, html, ocr) = conn.query_row(
        "SELECT text_content, rich_html, ocr_text FROM items WHERE id = ?1",
        [id],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    )?;
    Ok(Some(ItemDetail {
        item,
        text,
        html,
        ocr_text: ocr,
    }))
}

pub fn latest(conn: &Connection, n: i64) -> Res<Vec<Item>> {
    get_items(
        conn,
        &ItemQuery {
            limit: n,
            ..Default::default()
        },
    )
}

pub fn counts(conn: &Connection) -> Res<Counts> {
    conn.query_row(
        "SELECT count(*), COALESCE(sum(pinned), 0) FROM items WHERE expires_at IS NULL OR expires_at > ?1",
        [now_ms()],
        |r| Ok(Counts { all: r.get(0)?, pinned: r.get(1)? }),
    )
}

// ---------------------------------------------------------------------------------------------
// Item mutations
// ---------------------------------------------------------------------------------------------

pub fn mark_used(conn: &Connection, id: i64, move_to_top: bool) -> Res<()> {
    if move_to_top {
        conn.execute(
            "UPDATE items SET use_count = use_count + 1, last_used_at = ?2 WHERE id = ?1",
            params![id, now_ms()],
        )?;
    } else {
        conn.execute(
            "UPDATE items SET use_count = use_count + 1 WHERE id = ?1",
            [id],
        )?;
    }
    Ok(())
}

pub fn set_pinned(conn: &Connection, ids: &[i64], value: bool) -> Res<()> {
    for id in ids {
        conn.execute(
            "UPDATE items SET pinned = ?2 WHERE id = ?1",
            params![id, value as i64],
        )?;
    }
    Ok(())
}

pub fn delete_items(conn: &Connection, ids: &[i64]) -> Res<usize> {
    let mut n = 0;
    for id in ids {
        n += conn.execute("DELETE FROM items WHERE id = ?1", [id])?;
    }
    Ok(n)
}

pub fn tag_ids_for(conn: &Connection, id: i64) -> Res<Vec<i64>> {
    let mut st = conn.prepare_cached("SELECT tag_id FROM item_tags WHERE item_id = ?1")?;
    let rows = st.query_map([id], |r| r.get(0))?;
    rows.collect()
}

/// Update the text of an item; recomputes the hash so dedup keeps working.
/// Returns `Err` if another item already has exactly this content.
pub fn update_text(
    conn: &Connection,
    id: i64,
    text: &str,
    kind: Kind,
    language: Option<String>,
    hash: &str,
) -> Res<()> {
    conn.execute(
        "UPDATE items SET text_content = ?2, kind = ?3, language = ?4, content_hash = ?5,
            rich_rtf = NULL, rich_html = NULL, byte_size = ?6 WHERE id = ?1",
        params![id, text, kind.as_str(), language, hash, text.len() as i64],
    )?;
    Ok(())
}

pub fn move_to_space(conn: &Connection, ids: &[i64], space: Option<i64>) -> Res<()> {
    for id in ids {
        conn.execute(
            "UPDATE items SET space_id = ?2 WHERE id = ?1",
            params![id, space],
        )?;
    }
    Ok(())
}

pub fn set_item_tags(conn: &Connection, id: i64, tags: &[i64]) -> Res<()> {
    conn.execute("DELETE FROM item_tags WHERE item_id = ?1", [id])?;
    for t in tags {
        conn.execute(
            "INSERT OR IGNORE INTO item_tags (item_id, tag_id) VALUES (?1, ?2)",
            params![id, t],
        )?;
    }
    Ok(())
}

pub fn set_link_meta(
    conn: &Connection,
    id: i64,
    title: Option<&str>,
    favicon: Option<&str>,
) -> Res<()> {
    conn.execute(
        "UPDATE items SET link_title = COALESCE(?2, link_title), link_favicon_path = COALESCE(?3, link_favicon_path) WHERE id = ?1",
        params![id, title, favicon],
    )?;
    Ok(())
}

pub fn set_ocr_text(conn: &Connection, id: i64, text: &str) -> Res<()> {
    conn.execute(
        "UPDATE items SET ocr_text = ?2 WHERE id = ?1",
        params![id, text],
    )?;
    Ok(())
}

// ---------------------------------------------------------------------------------------------
// Spaces
// ---------------------------------------------------------------------------------------------

pub fn list_spaces(conn: &Connection) -> Res<Vec<Space>> {
    let mut st = conn.prepare(
        "SELECT s.id, s.name, s.color, s.icon, s.position,
            (SELECT count(*) FROM items i WHERE i.space_id = s.id)
         FROM spaces s ORDER BY s.position, s.id",
    )?;
    let rows = st.query_map([], |r| {
        Ok(Space {
            id: r.get(0)?,
            name: r.get(1)?,
            color: r.get(2)?,
            icon: r.get(3)?,
            position: r.get::<_, Option<i64>>(4)?.unwrap_or(0),
            count: r.get(5)?,
        })
    })?;
    rows.collect()
}

pub fn create_space(
    conn: &Connection,
    name: &str,
    color: Option<&str>,
    icon: Option<&str>,
) -> Res<i64> {
    let pos: i64 = conn.query_row(
        "SELECT COALESCE(max(position), -1) + 1 FROM spaces",
        [],
        |r| r.get(0),
    )?;
    conn.execute(
        "INSERT INTO spaces (name, color, icon, position) VALUES (?1, ?2, ?3, ?4)",
        params![name, color, icon, pos],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn update_space(
    conn: &Connection,
    id: i64,
    name: &str,
    color: Option<&str>,
    icon: Option<&str>,
) -> Res<()> {
    conn.execute(
        "UPDATE spaces SET name = ?2, color = ?3, icon = ?4 WHERE id = ?1",
        params![id, name, color, icon],
    )?;
    Ok(())
}

pub fn delete_space(conn: &Connection, id: i64) -> Res<()> {
    conn.execute("DELETE FROM spaces WHERE id = ?1", [id])?;
    Ok(())
}

pub fn reorder_spaces(conn: &Connection, ids: &[i64]) -> Res<()> {
    for (pos, id) in ids.iter().enumerate() {
        conn.execute(
            "UPDATE spaces SET position = ?2 WHERE id = ?1",
            params![id, pos as i64],
        )?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------------------------
// Tags
// ---------------------------------------------------------------------------------------------

pub fn list_tags(conn: &Connection) -> Res<Vec<Tag>> {
    let mut st = conn.prepare(
        "SELECT t.id, t.name, t.color, (SELECT count(*) FROM item_tags it WHERE it.tag_id = t.id)
         FROM tags t ORDER BY t.name COLLATE NOCASE",
    )?;
    let rows = st.query_map([], |r| {
        Ok(Tag {
            id: r.get(0)?,
            name: r.get(1)?,
            color: r.get(2)?,
            count: r.get(3)?,
        })
    })?;
    rows.collect()
}

pub fn create_tag(conn: &Connection, name: &str, color: Option<&str>) -> Res<i64> {
    if let Some(id) = conn
        .query_row(
            "SELECT id FROM tags WHERE name = ?1 COLLATE NOCASE",
            [name],
            |r| r.get(0),
        )
        .optional()?
    {
        return Ok(id);
    }
    conn.execute(
        "INSERT INTO tags (name, color) VALUES (?1, ?2)",
        params![name, color],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn update_tag(conn: &Connection, id: i64, name: &str, color: Option<&str>) -> Res<()> {
    conn.execute(
        "UPDATE tags SET name = ?2, color = ?3 WHERE id = ?1",
        params![id, name, color],
    )?;
    Ok(())
}

pub fn delete_tag(conn: &Connection, id: i64) -> Res<()> {
    conn.execute("DELETE FROM tags WHERE id = ?1", [id])?;
    Ok(())
}

/// Move every assignment of `from` tags onto `into`, then delete `from`.
pub fn merge_tags(conn: &Connection, from: &[i64], into: i64) -> Res<()> {
    for f in from.iter().filter(|f| **f != into) {
        conn.execute(
            "INSERT OR IGNORE INTO item_tags (item_id, tag_id) SELECT item_id, ?2 FROM item_tags WHERE tag_id = ?1",
            params![f, into],
        )?;
        conn.execute("DELETE FROM tags WHERE id = ?1", [f])?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------------------------
// Source apps
// ---------------------------------------------------------------------------------------------

pub fn upsert_source_app(conn: &Connection, app: &SourceApp) -> Res<()> {
    conn.execute(
        "INSERT INTO source_apps (bundle_id, name, color, icon_path, last_seen) VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(bundle_id) DO UPDATE SET name = excluded.name,
            color = COALESCE(excluded.color, source_apps.color),
            icon_path = COALESCE(excluded.icon_path, source_apps.icon_path),
            last_seen = excluded.last_seen",
        params![app.bundle_id, app.name, app.color, app.icon_path, now_ms()],
    )?;
    Ok(())
}

pub fn get_source_app(conn: &Connection, bundle_id: &str) -> Res<Option<SourceApp>> {
    conn.query_row(
        "SELECT bundle_id, name, color, icon_path FROM source_apps WHERE bundle_id = ?1",
        [bundle_id],
        |r| {
            Ok(SourceApp {
                bundle_id: r.get(0)?,
                name: r.get(1)?,
                color: r.get(2)?,
                icon_path: r.get(3)?,
            })
        },
    )
    .optional()
}

pub fn list_source_apps(conn: &Connection, limit: i64) -> Res<Vec<SourceAppInfo>> {
    let mut st = conn.prepare(
        "SELECT sa.bundle_id, sa.name, sa.color, sa.icon_path, count(i.id) AS n, max(i.last_used_at) AS seen
         FROM source_apps sa JOIN items i ON i.source_bundle_id = sa.bundle_id
         GROUP BY sa.bundle_id ORDER BY seen DESC LIMIT ?1",
    )?;
    let rows = st.query_map([limit], |r| {
        Ok(SourceAppInfo {
            bundle_id: r.get(0)?,
            name: r.get(1)?,
            color: r.get(2)?,
            icon_path: r.get(3)?,
            count: r.get(4)?,
            last_seen: r.get(5)?,
        })
    })?;
    rows.collect()
}

// ---------------------------------------------------------------------------------------------
// Excluded apps & settings
// ---------------------------------------------------------------------------------------------

pub fn list_excluded(conn: &Connection) -> Res<Vec<ExcludedApp>> {
    let mut st =
        conn.prepare("SELECT bundle_id, name FROM excluded_apps ORDER BY name COLLATE NOCASE")?;
    let rows = st.query_map([], |r| {
        Ok(ExcludedApp {
            bundle_id: r.get(0)?,
            name: r.get(1)?,
        })
    })?;
    rows.collect()
}

pub fn add_excluded(conn: &Connection, bundle_id: &str, name: Option<&str>) -> Res<()> {
    conn.execute(
        "INSERT OR REPLACE INTO excluded_apps (bundle_id, name) VALUES (?1, ?2)",
        params![bundle_id, name],
    )?;
    Ok(())
}

pub fn remove_excluded(conn: &Connection, bundle_id: &str) -> Res<()> {
    conn.execute(
        "DELETE FROM excluded_apps WHERE bundle_id = ?1",
        [bundle_id],
    )?;
    Ok(())
}

pub fn excluded_ids(conn: &Connection) -> Res<Vec<String>> {
    let mut st = conn.prepare("SELECT bundle_id FROM excluded_apps")?;
    let rows = st.query_map([], |r| r.get(0))?;
    rows.collect()
}

pub fn all_settings(conn: &Connection) -> Res<Vec<(String, String)>> {
    let mut st = conn.prepare("SELECT key, value FROM settings")?;
    let rows = st.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?;
    rows.collect()
}

pub fn set_setting(conn: &Connection, key: &str, value: &str) -> Res<()> {
    conn.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )?;
    Ok(())
}

// ---------------------------------------------------------------------------------------------
// Retention
// ---------------------------------------------------------------------------------------------

/// Delete expired/old items. Pinned items and items in custom spaces never expire
/// (except sensitive items with an explicit `expires_at`). Returns number of deleted rows.
pub fn cleanup(conn: &Connection, now: i64, max_items: i64, retention_days: i64) -> Res<usize> {
    let mut n = conn.execute(
        "DELETE FROM items WHERE expires_at IS NOT NULL AND expires_at <= ?1",
        [now],
    )?;
    if retention_days > 0 {
        let cutoff = now - retention_days * 86_400_000;
        n += conn.execute(
            "DELETE FROM items WHERE pinned = 0 AND space_id IS NULL AND last_used_at < ?1",
            [cutoff],
        )?;
    }
    if max_items > 0 {
        n += conn.execute(
            "DELETE FROM items WHERE id IN (
                SELECT id FROM items WHERE pinned = 0 AND space_id IS NULL
                ORDER BY last_used_at DESC LIMIT -1 OFFSET ?1)",
            [max_items],
        )?;
    }
    Ok(n)
}

pub fn clear_history(conn: &Connection, keep_pinned: bool) -> Res<usize> {
    if keep_pinned {
        conn.execute(
            "DELETE FROM items WHERE pinned = 0 AND space_id IS NULL",
            [],
        )
    } else {
        conn.execute("DELETE FROM items", [])
    }
}

/// Every file path referenced by an item (images, thumbnails, favicons).
pub fn referenced_files(conn: &Connection) -> Res<std::collections::HashSet<String>> {
    let mut st = conn.prepare(
        "SELECT image_path FROM items WHERE image_path IS NOT NULL
         UNION SELECT thumb_path FROM items WHERE thumb_path IS NOT NULL
         UNION SELECT link_favicon_path FROM items WHERE link_favicon_path IS NOT NULL",
    )?;
    let rows = st.query_map([], |r| r.get::<_, String>(0))?;
    rows.collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_memory;

    fn text_item(text: &str) -> NewItem {
        NewItem {
            kind: Some(Kind::Text),
            hash: crate::clipboard::hash::text_hash(text),
            text: Some(text.into()),
            byte_size: text.len() as i64,
            source: Some(SourceApp {
                bundle_id: "com.apple.Safari".into(),
                name: "Safari".into(),
                color: Some("#1E88E5".into()),
                icon_path: None,
            }),
            ..Default::default()
        }
    }

    #[test]
    fn dedup_moves_to_top() {
        let c = open_memory();
        let (a, new_a) = upsert_capture(&c, &text_item("hello"), 1000).unwrap();
        let (_b, _) = upsert_capture(&c, &text_item("world"), 2000).unwrap();
        let (a2, new_a2) = upsert_capture(&c, &text_item("hello"), 3000).unwrap();
        assert!(new_a);
        assert!(!new_a2);
        assert_eq!(a, a2);
        let n: i64 = c
            .query_row("SELECT count(*) FROM items", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 2);
        let items = get_items(&c, &ItemQuery::default()).unwrap();
        assert_eq!(items[0].id, a);
    }

    #[test]
    fn fts_prefix_accents_case() {
        let c = open_memory();
        upsert_capture(&c, &text_item("Perché il caffè è buono"), 1).unwrap();
        upsert_capture(&c, &text_item("Something else entirely"), 2).unwrap();
        let q = |t: &str| {
            get_items(
                &c,
                &ItemQuery {
                    text: t.into(),
                    ..Default::default()
                },
            )
            .unwrap()
            .len()
        };
        assert_eq!(q("caffe"), 1);
        assert_eq!(q("CAF"), 1);
        assert_eq!(q("perche buon"), 1);
        assert_eq!(q("some"), 1);
        assert_eq!(q("safari"), 2, "source app name is indexed");
        assert_eq!(q("nothing"), 0);
    }

    #[test]
    fn fts_stays_in_sync_on_update_and_delete() {
        let c = open_memory();
        let (id, _) = upsert_capture(&c, &text_item("alpha"), 1).unwrap();
        update_text(&c, id, "beta", Kind::Text, None, "h2").unwrap();
        let q = |t: &str| {
            get_items(
                &c,
                &ItemQuery {
                    text: t.into(),
                    ..Default::default()
                },
            )
            .unwrap()
            .len()
        };
        assert_eq!(q("alpha"), 0);
        assert_eq!(q("beta"), 1);
        delete_items(&c, &[id]).unwrap();
        assert_eq!(q("beta"), 0);
    }

    #[test]
    fn filters_and_tags() {
        let c = open_memory();
        let (a, _) = upsert_capture(&c, &text_item("one"), 1).unwrap();
        let (b, _) = upsert_capture(&c, &text_item("two"), 2).unwrap();
        let t = create_tag(&c, "Lavoro", Some("#f00")).unwrap();
        assert_eq!(
            create_tag(&c, "lavoro", None).unwrap(),
            t,
            "tag names are case-insensitive"
        );
        set_item_tags(&c, a, &[t]).unwrap();
        set_pinned(&c, &[b], true).unwrap();
        let r = get_items(
            &c,
            &ItemQuery {
                tags: vec!["lavoro".into()],
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].id, a);
        assert_eq!(r[0].tag_ids, vec![t]);
        let r = get_items(
            &c,
            &ItemQuery {
                pinned: Some(true),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(r[0].id, b);
        let r = get_items(
            &c,
            &ItemQuery {
                text: "lav".into(),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(r.len(), 1, "tag names are searchable");
        let r = get_items(
            &c,
            &ItemQuery {
                app_names: vec!["safari".into()],
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(r.len(), 2);
        let snippets = list_spaces(&c).unwrap()[0].id;
        move_to_space(&c, &[a], Some(snippets)).unwrap();
        let r = get_items(
            &c,
            &ItemQuery {
                space_names: vec!["snip".into()],
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(r.len(), 1);
        merge_tags(&c, &[t], create_tag(&c, "Work", None).unwrap()).unwrap();
        assert_eq!(list_tags(&c).unwrap().len(), 1);
    }

    #[test]
    fn retention_keeps_pinned_and_spaces() {
        let c = open_memory();
        let day = 86_400_000;
        let now = 100 * day;
        let (old, _) = upsert_capture(&c, &text_item("old"), now - 95 * day).unwrap();
        let (pinned, _) = upsert_capture(&c, &text_item("pinned"), now - 95 * day).unwrap();
        let (spaced, _) = upsert_capture(&c, &text_item("spaced"), now - 95 * day).unwrap();
        let (fresh, _) = upsert_capture(&c, &text_item("fresh"), now - day).unwrap();
        let mut sensitive = text_item("secret");
        sensitive.expires_at = Some(now - 1);
        let (exp, _) = upsert_capture(&c, &sensitive, now - 10).unwrap();
        set_pinned(&c, &[pinned], true).unwrap();
        move_to_space(&c, &[spaced], Some(1)).unwrap();
        cleanup(&c, now, 5000, 90).unwrap();
        let ids: Vec<i64> = all_full(&c).unwrap().iter().map(|r| r.id).collect();
        assert!(!ids.contains(&old));
        assert!(!ids.contains(&exp));
        assert!(ids.contains(&pinned) && ids.contains(&spaced) && ids.contains(&fresh));
    }

    #[test]
    fn retention_max_items() {
        let c = open_memory();
        for i in 0..10 {
            upsert_capture(&c, &text_item(&format!("item {i}")), i).unwrap();
        }
        cleanup(&c, 100, 3, 0).unwrap();
        let left = get_items(&c, &ItemQuery::default()).unwrap();
        assert_eq!(left.len(), 3);
        assert_eq!(left[0].preview, "item 9");
    }

    #[test]
    fn undo_roundtrip_keeps_id() {
        let c = open_memory();
        let (id, _) = upsert_capture(&c, &text_item("keep me"), 1).unwrap();
        let row = get_full(&c, id).unwrap().unwrap();
        delete_items(&c, &[id]).unwrap();
        insert_full(&c, &row, true).unwrap();
        assert_eq!(get_full(&c, id).unwrap().unwrap(), row);
        assert_eq!(
            get_items(
                &c,
                &ItemQuery {
                    text: "keep".into(),
                    ..Default::default()
                }
            )
            .unwrap()
            .len(),
            1
        );
    }

    #[test]
    fn search_10k_under_50ms() {
        let c = open_memory();
        c.execute_batch("BEGIN").unwrap();
        for i in 0..10_000 {
            upsert_capture(
                &c,
                &text_item(&format!(
                    "entry number {i} lorem ipsum dolor sit amet {}",
                    i * 7
                )),
                i,
            )
            .unwrap();
        }
        c.execute_batch("COMMIT").unwrap();
        let t = std::time::Instant::now();
        let r = get_items(
            &c,
            &ItemQuery {
                text: "lorem 699".into(),
                limit: 60,
                ..Default::default()
            },
        )
        .unwrap();
        let elapsed = t.elapsed();
        assert!(!r.is_empty());
        assert!(elapsed.as_millis() < 50, "search took {elapsed:?}");
    }

    /// Profiling with 50k items (BRIEF §7 phase 7). Run with `cargo test --release -- --ignored`.
    #[test]
    #[ignore]
    fn profile_50k() {
        let c = open_memory();
        c.execute_batch("BEGIN").unwrap();
        let words = [
            "riunione", "fattura", "progetto", "caffè", "deploy", "password", "milano", "report",
            "design", "codice",
        ];
        for i in 0..50_000i64 {
            let w = words[(i as usize) % words.len()];
            let t = format!(
                "{w} numero {i} lorem ipsum dolor sit amet consectetur {} {}",
                i * 13,
                words[(i as usize * 7) % words.len()]
            );
            upsert_capture(&c, &text_item(&t), i).unwrap();
        }
        c.execute_batch("COMMIT").unwrap();
        for (label, q) in [
            (
                "recent page",
                ItemQuery {
                    limit: 60,
                    ..Default::default()
                },
            ),
            (
                "fts word",
                ItemQuery {
                    text: "caffe".into(),
                    limit: 60,
                    ..Default::default()
                },
            ),
            (
                "fts prefix",
                ItemQuery {
                    text: "prog 4999".into(),
                    limit: 60,
                    ..Default::default()
                },
            ),
            (
                "deep page",
                ItemQuery {
                    offset: 40_000,
                    limit: 60,
                    ..Default::default()
                },
            ),
            (
                "app filter",
                ItemQuery {
                    app_names: vec!["saf".into()],
                    limit: 60,
                    ..Default::default()
                },
            ),
        ] {
            let t = std::time::Instant::now();
            let r = get_items(&c, &q).unwrap();
            let ms = t.elapsed().as_secs_f64() * 1000.0;
            println!("{label:12} {ms:7.2} ms  ({} rows)", r.len());
            assert!(ms < 50.0, "{label} took {ms} ms");
        }
        let t = std::time::Instant::now();
        cleanup(&c, 50_000, 5000, 0).unwrap();
        println!("cleanup→5000 {:7.2} ms", t.elapsed().as_secs_f64() * 1000.0);
    }
}
