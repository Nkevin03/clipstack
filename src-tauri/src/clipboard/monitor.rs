//! Background thread polling `NSPasteboard.changeCount` every 250 ms.

use super::{hash, images, kinds, pasteboard, source};
use crate::db::queries::{self, NewItem};
use crate::models::{Kind, SourceApp};
use crate::privacy;
use crate::settings::{SensitivePolicy, Settings};
use crate::state::AppState;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

pub const POLL: Duration = Duration::from_millis(250);

/// Truncate to at most `max` bytes on a char boundary.
pub fn truncate_bytes(s: &str, max: usize) -> &str {
    if s.len() <= max {
        return s;
    }
    let mut end = max;
    while !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

/// Outcome of analysing a pasteboard snapshot (pure, no I/O except nothing).
#[derive(Debug)]
pub enum Plan {
    Skip(&'static str),
    Text(NewItem),
    Files(NewItem),
    Image {
        bytes: Vec<u8>,
        fmt: pasteboard::ImageFormat,
        hash: String,
    },
}

pub fn plan(snap: pasteboard::Snapshot, s: &Settings, now: i64) -> Plan {
    if privacy::is_flagged(&snap) {
        return Plan::Skip("concealed/transient");
    }
    if !snap.files.is_empty() {
        let byte_size = snap
            .files
            .iter()
            .filter_map(|f| std::fs::metadata(f).ok())
            .map(|m| m.len() as i64)
            .sum();
        return Plan::Files(NewItem {
            kind: Some(Kind::File),
            hash: hash::files_hash(&snap.files),
            text: Some(snap.files.join("\n")),
            file_paths: snap.files,
            byte_size,
            ..Default::default()
        });
    }
    let text = snap.text.as_deref().or(snap.url.as_deref()).unwrap_or("");
    if !text.trim().is_empty() {
        let max = s.max_text_bytes.max(1) as usize;
        let text = truncate_bytes(text, max).to_string();
        let mut expires_at = None;
        if privacy::detect_sensitive(&text).is_some() {
            match s.sensitive_policy {
                SensitivePolicy::Skip => return Plan::Skip("sensitive"),
                SensitivePolicy::Expire => {
                    expires_at = Some(now + s.sensitive_expire_minutes.max(1) * 60_000)
                }
                SensitivePolicy::Keep => {}
            }
        }
        let rich_size = snap.rtf.as_ref().map(|r| r.len()).unwrap_or(0)
            + snap.html.as_ref().map(|h| h.len()).unwrap_or(0);
        let keep_rich = rich_size > 0 && rich_size <= max && expires_at.is_none();
        let (kind, language) = kinds::detect(&text, keep_rich);
        let rich_ok = keep_rich && kind == Kind::Rich;
        return Plan::Text(NewItem {
            kind: Some(kind),
            hash: hash::text_hash(&text),
            byte_size: text.len() as i64,
            text: Some(text),
            rtf: if rich_ok { snap.rtf } else { None },
            html: if rich_ok { snap.html } else { None },
            language,
            expires_at,
            ..Default::default()
        });
    }
    if let Some((bytes, fmt)) = snap.image {
        if bytes.len() as i64 > s.max_image_bytes {
            return Plan::Skip("image too large");
        }
        let hash = hash::bytes_hash(&bytes);
        return Plan::Image { bytes, fmt, hash };
    }
    Plan::Skip("empty")
}

struct Monitor {
    app: AppHandle,
    state: Arc<AppState>,
    sources: HashMap<String, SourceApp>,
}

impl Monitor {
    fn source_for(&mut self, front: &source::Frontmost) -> SourceApp {
        if let Some(s) = self.sources.get(&front.bundle_id) {
            return s.clone();
        }
        let cached = queries::get_source_app(&self.state.db(), &front.bundle_id)
            .ok()
            .flatten();
        let app = match cached {
            Some(c)
                if c.icon_path
                    .as_ref()
                    .map(|p| std::path::Path::new(p).exists())
                    .unwrap_or(false) =>
            {
                c
            }
            _ => {
                let icon =
                    source::save_app_icon(front.pid, &front.bundle_id, &self.state.paths.icons);
                let color = icon
                    .as_ref()
                    .and_then(|p| std::fs::read(p).ok())
                    .and_then(|b| source::dominant_color(&b));
                let app = SourceApp {
                    bundle_id: front.bundle_id.clone(),
                    name: front.name.clone(),
                    color,
                    icon_path: icon.map(|p| p.to_string_lossy().into_owned()),
                };
                let _ = queries::upsert_source_app(&self.state.db(), &app);
                app
            }
        };
        self.sources.insert(front.bundle_id.clone(), app.clone());
        app
    }

    fn handle_change(&mut self) {
        let settings = self.state.settings();
        let front = source::frontmost();
        if let Some(f) = &front {
            let excluded = queries::excluded_ids(&self.state.db()).unwrap_or_default();
            if excluded.iter().any(|b| b == &f.bundle_id) {
                log::debug!("skip: excluded app {}", f.bundle_id);
                return;
            }
            if f.bundle_id == self.app.config().identifier {
                return;
            }
        }
        let snap = pasteboard::read(settings.max_image_bytes.max(0) as usize);
        let now = queries::now_ms();
        let mut item = match plan(snap, &settings, now) {
            Plan::Skip(reason) => {
                log::debug!("skip: {reason}");
                return;
            }
            Plan::Text(n) | Plan::Files(n) => n,
            Plan::Image { bytes, fmt, hash } => {
                let p = &self.state.paths;
                let Some(saved) = images::save(&bytes, fmt, &hash, &p.images, &p.thumbs) else {
                    log::warn!("could not decode clipboard image");
                    return;
                };
                NewItem {
                    kind: Some(Kind::Image),
                    hash,
                    image_path: Some(saved.path),
                    thumb_path: saved.thumb,
                    width: Some(saved.width),
                    height: Some(saved.height),
                    byte_size: bytes.len() as i64,
                    ..Default::default()
                }
            }
        };
        if let Some(f) = &front {
            item.source = Some(self.source_for(f));
        }
        let result = {
            let db = self.state.db();
            queries::upsert_capture(&db, &item, now)
                .and_then(|(id, inserted)| Ok((queries::get_item(&db, id)?, inserted)))
        };
        match result {
            Ok((Some(saved), inserted)) => {
                log::info!(
                    "captured #{} kind={} new={} app={:?} bytes={}",
                    saved.id,
                    saved.kind.as_str(),
                    inserted,
                    saved.source_app_name,
                    saved.byte_size
                );
                crate::hooks::after_capture(&self.app, &settings, &saved, inserted);
                let _ = self.app.emit("clipboard:new-item", &saved);
            }
            Ok((None, _)) => {}
            Err(e) => log::error!("db error while saving capture: {e}"),
        }
    }
}

pub fn start(app: AppHandle) {
    let state = app.state::<Arc<AppState>>().inner().clone();
    std::thread::Builder::new()
        .name("clipboard-monitor".into())
        .spawn(move || {
            let mut m = Monitor {
                app,
                state,
                sources: HashMap::new(),
            };
            let mut last = pasteboard::change_count();
            loop {
                std::thread::sleep(POLL);
                let cc = pasteboard::change_count();
                if cc == last {
                    continue;
                }
                last = cc;
                if cc == m.state.own_change_count.load(Ordering::SeqCst) {
                    continue; // our own write (paste) — avoid the loop
                }
                if m.state.paused.load(Ordering::SeqCst) {
                    continue;
                }
                m.handle_change();
            }
        })
        .expect("spawn clipboard monitor");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clipboard::pasteboard::Snapshot;

    fn snap_text(t: &str) -> Snapshot {
        Snapshot {
            types: vec!["public.utf8-plain-text".into()],
            text: Some(t.into()),
            ..Default::default()
        }
    }

    #[test]
    fn concealed_is_never_planned() {
        let mut s = snap_text("hunter2");
        s.types.push(pasteboard::CONCEALED.into());
        assert!(matches!(plan(s, &Settings::default(), 0), Plan::Skip(_)));
        let mut s = snap_text("x");
        s.types.push(pasteboard::TRANSIENT.into());
        assert!(matches!(plan(s, &Settings::default(), 0), Plan::Skip(_)));
    }

    #[test]
    fn sensitive_policies() {
        let card = "4111 1111 1111 1111";
        let mut s = Settings {
            sensitive_policy: SensitivePolicy::Skip,
            ..Default::default()
        };
        assert!(matches!(
            plan(snap_text(card), &s, 0),
            Plan::Skip("sensitive")
        ));
        s.sensitive_policy = SensitivePolicy::Expire;
        s.sensitive_expire_minutes = 5;
        match plan(snap_text(card), &s, 1000) {
            Plan::Text(n) => assert_eq!(n.expires_at, Some(1000 + 5 * 60_000)),
            p => panic!("{p:?}"),
        }
        s.sensitive_policy = SensitivePolicy::Keep;
        match plan(snap_text(card), &s, 0) {
            Plan::Text(n) => assert_eq!(n.expires_at, None),
            p => panic!("{p:?}"),
        }
    }

    #[test]
    fn text_is_truncated_on_char_boundary() {
        let s = Settings {
            max_text_bytes: 5,
            ..Default::default()
        };
        match plan(snap_text("aèèèè"), &s, 0) {
            Plan::Text(n) => assert_eq!(n.text.as_deref(), Some("aèè")),
            p => panic!("{p:?}"),
        }
    }

    #[test]
    fn files_take_priority_and_rich_detected() {
        let snap = Snapshot {
            files: vec!["/tmp/a.txt".into()],
            text: Some("a.txt".into()),
            ..Default::default()
        };
        assert!(matches!(
            plan(snap, &Settings::default(), 0),
            Plan::Files(_)
        ));
        let snap = Snapshot {
            text: Some("hello".into()),
            html: Some("<b>hello</b>".into()),
            ..Default::default()
        };
        match plan(snap, &Settings::default(), 0) {
            Plan::Text(n) => {
                assert_eq!(n.kind, Some(Kind::Rich));
                assert!(n.html.is_some());
            }
            p => panic!("{p:?}"),
        }
        let snap = Snapshot {
            text: Some("https://a.com".into()),
            html: Some("<a>".into()),
            ..Default::default()
        };
        match plan(snap, &Settings::default(), 0) {
            Plan::Text(n) => {
                assert_eq!(n.kind, Some(Kind::Link));
                assert!(n.html.is_none(), "rich payload only kept for rich kind");
            }
            p => panic!("{p:?}"),
        }
    }

    #[test]
    fn image_size_limit() {
        let snap = Snapshot {
            image: Some((vec![0; 10], pasteboard::ImageFormat::Png)),
            ..Default::default()
        };
        let s = Settings {
            max_image_bytes: 5,
            ..Default::default()
        };
        assert!(matches!(plan(snap, &s, 0), Plan::Skip(_)));
    }
}
