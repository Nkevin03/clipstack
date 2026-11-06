//! SQL for each schema version. Migrations are applied in order by `migrations.rs`.

pub const V1: &str = r#"
CREATE TABLE spaces (
  id INTEGER PRIMARY KEY,
  name TEXT NOT NULL,
  color TEXT,
  icon TEXT,
  position INTEGER DEFAULT 0
);

CREATE TABLE items (
  id INTEGER PRIMARY KEY,
  kind TEXT NOT NULL,
  content_hash TEXT NOT NULL UNIQUE,
  text_content TEXT,
  rich_rtf BLOB,
  rich_html TEXT,
  file_paths TEXT,
  image_path TEXT,
  thumb_path TEXT,
  width INTEGER,
  height INTEGER,
  byte_size INTEGER,
  source_bundle_id TEXT,
  source_app_name TEXT,
  source_color TEXT,
  link_title TEXT,
  link_favicon_path TEXT,
  language TEXT,
  ocr_text TEXT,
  pinned INTEGER NOT NULL DEFAULT 0,
  space_id INTEGER REFERENCES spaces(id) ON DELETE SET NULL,
  use_count INTEGER NOT NULL DEFAULT 0,
  created_at INTEGER NOT NULL,
  last_used_at INTEGER NOT NULL,
  expires_at INTEGER
);
CREATE INDEX idx_items_last_used ON items(last_used_at DESC);
CREATE INDEX idx_items_space ON items(space_id);
CREATE INDEX idx_items_kind ON items(kind);
CREATE INDEX idx_items_source ON items(source_bundle_id);

CREATE VIRTUAL TABLE items_fts USING fts5(
  text_content, ocr_text, link_title, source_app_name,
  content='items', content_rowid='id',
  tokenize='unicode61 remove_diacritics 2', prefix='2 3'
);

CREATE TRIGGER items_ai AFTER INSERT ON items BEGIN
  INSERT INTO items_fts(rowid, text_content, ocr_text, link_title, source_app_name)
  VALUES (new.id, new.text_content, new.ocr_text, new.link_title, new.source_app_name);
END;
CREATE TRIGGER items_ad AFTER DELETE ON items BEGIN
  INSERT INTO items_fts(items_fts, rowid, text_content, ocr_text, link_title, source_app_name)
  VALUES ('delete', old.id, old.text_content, old.ocr_text, old.link_title, old.source_app_name);
END;
CREATE TRIGGER items_au AFTER UPDATE OF text_content, ocr_text, link_title, source_app_name ON items BEGIN
  INSERT INTO items_fts(items_fts, rowid, text_content, ocr_text, link_title, source_app_name)
  VALUES ('delete', old.id, old.text_content, old.ocr_text, old.link_title, old.source_app_name);
  INSERT INTO items_fts(rowid, text_content, ocr_text, link_title, source_app_name)
  VALUES (new.id, new.text_content, new.ocr_text, new.link_title, new.source_app_name);
END;

CREATE TABLE tags (id INTEGER PRIMARY KEY, name TEXT NOT NULL UNIQUE COLLATE NOCASE, color TEXT);
CREATE TABLE item_tags (
  item_id INTEGER REFERENCES items(id) ON DELETE CASCADE,
  tag_id  INTEGER REFERENCES tags(id)  ON DELETE CASCADE,
  PRIMARY KEY (item_id, tag_id)
);
CREATE INDEX idx_item_tags_tag ON item_tags(tag_id);

CREATE TABLE excluded_apps (bundle_id TEXT PRIMARY KEY, name TEXT);
CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);

-- Cache of source applications (name, dominant icon colour, icon PNG on disk).
CREATE TABLE source_apps (
  bundle_id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  color TEXT,
  icon_path TEXT,
  last_seen INTEGER NOT NULL
);

INSERT INTO spaces (name, color, icon, position) VALUES ('Snippets', '#10B981', 'code', 0);

INSERT INTO excluded_apps (bundle_id, name) VALUES
  ('com.1password.1password', '1Password'),
  ('com.agilebits.onepassword7', '1Password 7'),
  ('com.bitwarden.desktop', 'Bitwarden'),
  ('com.dashlane.Dashlane', 'Dashlane'),
  ('com.dashlane.dashlanephonefinal', 'Dashlane'),
  ('com.lastpass.LastPass', 'LastPass'),
  ('in.sinew.Enpass-Desktop', 'Enpass'),
  ('org.keepassxc.keepassxc', 'KeePassXC'),
  ('com.apple.keychainaccess', 'Keychain Access'),
  ('com.apple.Passwords', 'Passwords');
"#;

/// Ordered list of migrations; index + 1 == resulting `user_version`.
pub const MIGRATIONS: &[&str] = &[V1];
