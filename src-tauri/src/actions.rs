//! Text transformations available from the ⌘K actions menu.

use base64::Engine;
use percent_encoding::{NON_ALPHANUMERIC, percent_decode_str, utf8_percent_encode};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Serialize, Deserialize, TS, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum Action {
    Upper,
    Lower,
    Title,
    StripFormatting,
    Trim,
    DedupLines,
    SortLines,
    JsonPretty,
    JsonMinify,
    UrlEncode,
    UrlDecode,
    Base64Encode,
    Base64Decode,
    Count,
}

#[derive(Serialize, Deserialize, TS, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct TextStats {
    pub chars: i64,
    pub chars_no_spaces: i64,
    pub words: i64,
    pub lines: i64,
}

pub fn stats(text: &str) -> TextStats {
    TextStats {
        chars: text.chars().count() as i64,
        chars_no_spaces: text.chars().filter(|c| !c.is_whitespace()).count() as i64,
        words: text.split_whitespace().count() as i64,
        lines: if text.is_empty() {
            0
        } else {
            text.lines().count() as i64
        },
    }
}

fn title_case(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut start = true;
    for c in s.chars() {
        if c.is_alphanumeric() || c == '\'' || c == '’' {
            if start {
                out.extend(c.to_uppercase());
            } else {
                out.extend(c.to_lowercase());
            }
            start = false;
        } else {
            out.push(c);
            start = true;
        }
    }
    out
}

/// Apply a transformation. `Count` returns the text unchanged (use [`stats`]).
pub fn apply(action: Action, text: &str) -> Result<String, String> {
    Ok(match action {
        Action::Upper => text.to_uppercase(),
        Action::Lower => text.to_lowercase(),
        Action::Title => title_case(text),
        // Items keep a plain-text twin of rich content; returning it drops the formatting.
        Action::StripFormatting | Action::Count => text.to_string(),
        Action::Trim => text
            .lines()
            .map(|l| l.trim())
            .collect::<Vec<_>>()
            .join("\n")
            .trim()
            .to_string(),
        Action::DedupLines => {
            let mut seen = std::collections::HashSet::new();
            text.lines()
                .filter(|l| seen.insert(*l))
                .collect::<Vec<_>>()
                .join("\n")
        }
        Action::SortLines => {
            let mut lines: Vec<&str> = text.lines().collect();
            lines.sort_by_key(|l| l.to_lowercase());
            lines.join("\n")
        }
        Action::JsonPretty => {
            let v: serde_json::Value =
                serde_json::from_str(text.trim()).map_err(|e| format!("JSON: {e}"))?;
            serde_json::to_string_pretty(&v).map_err(|e| e.to_string())?
        }
        Action::JsonMinify => {
            let v: serde_json::Value =
                serde_json::from_str(text.trim()).map_err(|e| format!("JSON: {e}"))?;
            serde_json::to_string(&v).map_err(|e| e.to_string())?
        }
        Action::UrlEncode => utf8_percent_encode(text, NON_ALPHANUMERIC).to_string(),
        Action::UrlDecode => percent_decode_str(&text.replace('+', " "))
            .decode_utf8()
            .map_err(|e| e.to_string())?
            .into_owned(),
        Action::Base64Encode => base64::engine::general_purpose::STANDARD.encode(text.as_bytes()),
        Action::Base64Decode => {
            let clean: String = text.chars().filter(|c| !c.is_whitespace()).collect();
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(&clean)
                .or_else(|_| {
                    base64::engine::general_purpose::URL_SAFE_NO_PAD
                        .decode(clean.trim_end_matches('='))
                })
                .map_err(|e| format!("Base64: {e}"))?;
            String::from_utf8(bytes).map_err(|_| "Base64: not UTF-8 text".to_string())?
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn case() {
        assert_eq!(apply(Action::Upper, "ciao è").unwrap(), "CIAO È");
        assert_eq!(apply(Action::Lower, "CIAO").unwrap(), "ciao");
        assert_eq!(
            apply(Action::Title, "hello wORLD, l'amico").unwrap(),
            "Hello World, L'amico"
        );
    }

    #[test]
    fn lines() {
        assert_eq!(apply(Action::Trim, "  a  \n b \n\n").unwrap(), "a\nb");
        assert_eq!(
            apply(Action::DedupLines, "a\nb\na\nc\nb").unwrap(),
            "a\nb\nc"
        );
        assert_eq!(apply(Action::SortLines, "b\nA\nc").unwrap(), "A\nb\nc");
    }

    #[test]
    fn json() {
        assert_eq!(
            apply(Action::JsonMinify, "{ \"a\" : [1, 2] }").unwrap(),
            "{\"a\":[1,2]}"
        );
        assert_eq!(
            apply(Action::JsonPretty, "{\"a\":1}").unwrap(),
            "{\n  \"a\": 1\n}"
        );
        assert!(apply(Action::JsonPretty, "{nope").is_err());
    }

    #[test]
    fn encodings() {
        assert_eq!(
            apply(Action::UrlEncode, "a b&c/è").unwrap(),
            "a%20b%26c%2F%C3%A8"
        );
        assert_eq!(apply(Action::UrlDecode, "a%20b+c%C3%A8").unwrap(), "a b cè");
        assert_eq!(apply(Action::Base64Encode, "ciao").unwrap(), "Y2lhbw==");
        assert_eq!(apply(Action::Base64Decode, "Y2lhbw==").unwrap(), "ciao");
        assert_eq!(apply(Action::Base64Decode, "Y2lhbw").unwrap(), "ciao");
        assert!(apply(Action::Base64Decode, "@@@").is_err());
    }

    #[test]
    fn counting() {
        let s = stats("ciao mondo\nsecond line");
        assert_eq!(
            s,
            TextStats {
                chars: 22,
                chars_no_spaces: 19,
                words: 4,
                lines: 2
            }
        );
    }
}
