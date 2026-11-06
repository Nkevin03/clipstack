//! Opt-in link enrichment (title + favicon). This is the only network access in Clipstack and
//! runs only when "Fetch link titles and icons" is enabled in Privacy settings.

use crate::db::queries as q;
use crate::state::AppState;
use regex::Regex;
use std::sync::{Arc, LazyLock};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

static TITLE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?is)<title[^>]*>(.*?)</title>").unwrap());
static OG_TITLE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?is)<meta[^>]+property=["']og:title["'][^>]+content=["']([^"']+)["']"#).unwrap()
});
static ICON: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(?is)<link[^>]+rel=["'][^"']*icon[^"']*["'][^>]*>"#).unwrap());
static HREF: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(?is)href=["']([^"']+)["']"#).unwrap());

fn decode_entities(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&#x27;", "'")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn parse_title(html: &str) -> Option<String> {
    OG_TITLE
        .captures(html)
        .or_else(|| TITLE.captures(html))
        .map(|c| decode_entities(c[1].trim()))
        .filter(|t| !t.is_empty())
}

pub fn parse_icon(html: &str, base: &url::Url) -> url::Url {
    ICON.find_iter(html)
        .filter_map(|m| HREF.captures(m.as_str()).map(|c| c[1].to_string()))
        .find_map(|h| base.join(&h).ok())
        .unwrap_or_else(|| base.join("/favicon.ico").expect("valid"))
}

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(6)))
        .user_agent("Clipstack/1.0 (link preview)")
        .build()
        .into()
}

pub fn spawn(app: AppHandle, id: i64, link: String) {
    std::thread::Builder::new()
        .name("linkmeta".into())
        .spawn(move || {
            let Ok(base) = url::Url::parse(link.trim()) else {
                return;
            };
            if !matches!(base.scheme(), "http" | "https") {
                return;
            }
            let agent = agent();
            let html = agent.get(base.as_str()).call().ok().and_then(|mut r| {
                r.body_mut()
                    .with_config()
                    .limit(512 * 1024)
                    .read_to_string()
                    .ok()
            });
            let title = html.as_deref().and_then(parse_title);
            let icon_url = parse_icon(html.as_deref().unwrap_or(""), &base);
            let state = app.state::<Arc<AppState>>().inner().clone();
            let favicon = agent
                .get(icon_url.as_str())
                .call()
                .ok()
                .and_then(|mut r| {
                    r.body_mut()
                        .with_config()
                        .limit(256 * 1024)
                        .read_to_vec()
                        .ok()
                })
                .and_then(|bytes| image::load_from_memory(&bytes).ok())
                .and_then(|img| {
                    let host = base
                        .host_str()
                        .unwrap_or("site")
                        .replace(|c: char| !c.is_alphanumeric() && c != '.', "_");
                    let p = state.paths.favicons.join(format!("{host}.png"));
                    img.thumbnail(64, 64)
                        .save(&p)
                        .ok()
                        .map(|_| p.to_string_lossy().into_owned())
                });
            if title.is_some() || favicon.is_some() {
                let _ = q::set_link_meta(&state.db(), id, title.as_deref(), favicon.as_deref());
                let _ = app.emit("items:changed", ());
            }
        })
        .ok();
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_title_and_icon() {
        let html = r#"<html><head><title> Hello &amp; welcome </title><link rel="shortcut icon" href="/static/i.png"></head>"#;
        assert_eq!(parse_title(html).as_deref(), Some("Hello & welcome"));
        let base = url::Url::parse("https://example.com/a/b").unwrap();
        assert_eq!(
            parse_icon(html, &base).as_str(),
            "https://example.com/static/i.png"
        );
        assert_eq!(
            parse_icon("", &base).as_str(),
            "https://example.com/favicon.ico"
        );
        let og = r#"<meta property="og:title" content="OG Title"><title>x</title>"#;
        assert_eq!(parse_title(og).as_deref(), Some("OG Title"));
    }
}
