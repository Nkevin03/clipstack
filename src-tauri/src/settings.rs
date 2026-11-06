//! User settings, persisted one key per row in the `settings` table (value = JSON).

use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Serialize, Deserialize, TS, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum Layout {
    Top,
    Bottom,
    Center,
}

#[derive(Serialize, Deserialize, TS, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum SensitivePolicy {
    /// Never store detected secrets.
    Skip,
    /// Store, then delete after `sensitive_expire_minutes`.
    Expire,
    /// Store normally.
    Keep,
}

#[derive(Serialize, Deserialize, TS, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct Settings {
    pub shortcut: String,
    pub pause_shortcut: Option<String>,
    /// Starts/stops a copy sequence (copy A, B, C → ⌘V ⌘V ⌘V pastes C, B, A).
    pub sequence_shortcut: Option<String>,
    /// "lastFirst" (C, B, A) | "firstFirst" (A, B, C)
    pub sequence_order: String,
    /// Loop over the sequence instead of consuming it.
    pub sequence_cycle: bool,
    /// After every ⌘V show a small window with the last `paste_peek_count` copies.
    pub paste_peek: bool,
    pub paste_peek_count: i64,
    pub layout: Layout,
    /// Windows-style: open a compact history popup next to the mouse cursor.
    pub cursor_popup: bool,
    /// "auto" | "light" | "dark"
    pub theme: String,
    /// "auto" | "it" | "en"
    pub language: String,
    pub accent: String,
    pub use_system_accent: bool,
    /// "small" | "medium" | "large"
    pub card_size: String,
    /// "comfortable" | "compact"
    pub density: String,
    pub sound_on_copy: bool,
    pub launch_at_login: bool,
    pub auto_paste: bool,
    pub move_to_top_after_paste: bool,

    /// "newline" | "space" | "comma" | "none"
    pub multi_separator: String,
    pub max_text_bytes: i64,
    pub max_image_bytes: i64,
    pub max_items: i64,
    pub retention_days: i64,
    pub sensitive_policy: SensitivePolicy,
    pub sensitive_expire_minutes: i64,
    pub fetch_link_meta: bool,
    pub ocr_enabled: bool,
    pub encrypt_db: bool,
    pub onboarding_done: bool,
    pub sidebar_collapsed: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            shortcut: "CommandOrControl+Shift+V".into(),
            pause_shortcut: None,
            sequence_shortcut: Some("CommandOrControl+Alt+Shift+C".into()),
            sequence_order: "lastFirst".into(),
            sequence_cycle: false,
            paste_peek: false,
            paste_peek_count: 3,
            layout: Layout::Top,
            cursor_popup: false,
            theme: "auto".into(),
            language: "auto".into(),
            accent: "#6366F1".into(),
            use_system_accent: false,
            card_size: "medium".into(),
            density: "comfortable".into(),
            sound_on_copy: false,
            launch_at_login: false,
            auto_paste: true,
            move_to_top_after_paste: true,
            multi_separator: "newline".into(),
            max_text_bytes: 1024 * 1024,
            max_image_bytes: 25 * 1024 * 1024,
            max_items: 5000,
            retention_days: 90,
            sensitive_policy: SensitivePolicy::Expire,
            sensitive_expire_minutes: 10,
            fetch_link_meta: false,
            ocr_enabled: true,
            encrypt_db: false,
            onboarding_done: false,
            sidebar_collapsed: false,
        }
    }
}

impl Settings {
    pub fn load(conn: &Connection) -> anyhow::Result<Settings> {
        let mut map = match serde_json::to_value(Settings::default())? {
            serde_json::Value::Object(m) => m,
            _ => unreachable!(),
        };
        for (k, v) in crate::db::queries::all_settings(conn)? {
            if map.contains_key(&k)
                && let Ok(val) = serde_json::from_str(&v)
            {
                map.insert(k, val);
            }
        }
        // A single bad value must not reset everything: fall back per key.
        match serde_json::from_value::<Settings>(serde_json::Value::Object(map.clone())) {
            Ok(s) => Ok(s),
            Err(_) => {
                let defaults = serde_json::to_value(Settings::default())?;
                let mut good = defaults.as_object().cloned().unwrap_or_default();
                for (k, v) in map {
                    let mut probe = good.clone();
                    probe.insert(k.clone(), v.clone());
                    if serde_json::from_value::<Settings>(serde_json::Value::Object(probe)).is_ok()
                    {
                        good.insert(k, v);
                    }
                }
                Ok(serde_json::from_value(serde_json::Value::Object(good))?)
            }
        }
    }

    /// Validate and apply one key; returns the new settings. Does not persist.
    pub fn with(&self, key: &str, value: serde_json::Value) -> anyhow::Result<Settings> {
        let mut v = serde_json::to_value(self)?;
        let obj = v.as_object_mut().unwrap();
        if !obj.contains_key(key) {
            anyhow::bail!("unknown setting {key}");
        }
        obj.insert(key.to_string(), value);
        Ok(serde_json::from_value(v)?)
    }
}

pub fn persist(conn: &Connection, key: &str, value: &serde_json::Value) -> anyhow::Result<()> {
    crate::db::queries::set_setting(conn, key, &serde_json::to_string(value)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_and_validation() {
        let c = crate::db::open_memory();
        let s = Settings::load(&c).unwrap();
        assert_eq!(s, Settings::default());
        let s2 = s.with("layout", serde_json::json!("center")).unwrap();
        persist(&c, "layout", &serde_json::json!("center")).unwrap();
        assert_eq!(Settings::load(&c).unwrap().layout, Layout::Center);
        assert_eq!(s2.layout, Layout::Center);
        assert!(s.with("layout", serde_json::json!("sideways")).is_err());
        assert!(s.with("nope", serde_json::json!(1)).is_err());
        // Corrupt value falls back to default for that key only.
        crate::db::queries::set_setting(&c, "maxItems", "\"lots\"").unwrap();
        let s3 = Settings::load(&c).unwrap();
        assert_eq!(s3.max_items, 5000);
        assert_eq!(s3.layout, Layout::Center);
    }
}
