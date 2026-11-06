//! Privacy rules: pasteboard flags, excluded apps and sensitive-data detection.

use regex::Regex;
use std::sync::LazyLock;

use crate::clipboard::pasteboard::{AUTO_GENERATED, CONCEALED, Snapshot, TRANSIENT};

/// Content marked by the source app as concealed/transient must never be stored.
pub fn is_flagged(snap: &Snapshot) -> bool {
    snap.has_type(CONCEALED) || snap.has_type(TRANSIENT) || snap.has_type(AUTO_GENERATED)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sensitive {
    CreditCard,
    ApiKey,
    PrivateKey,
    Iban,
}

static CARD_CANDIDATE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b(?:\d[ -]?){13,19}\b").unwrap());
static PRIVATE_KEY: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"-----BEGIN (?:[A-Z0-9]+ )*PRIVATE KEY-----").unwrap());
static IBAN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b[A-Z]{2}\d{2}(?: ?[A-Z0-9]){11,30}\b").unwrap());
static API_KEYS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    [
        r"\bAKIA[0-9A-Z]{16}\b",                                  // AWS access key
        r"\bgh[pousr]_[A-Za-z0-9]{36,}\b",                        // GitHub
        r"\bgithub_pat_[A-Za-z0-9_]{50,}\b",                      // GitHub fine-grained
        r"\bsk-(?:ant-|proj-|live_|test_)?[A-Za-z0-9_\-]{20,}\b", // OpenAI / Anthropic / Stripe style
        r"\b[rs]k_(?:live|test)_[A-Za-z0-9]{20,}\b",              // Stripe
        r"\bxox[abprs]-[A-Za-z0-9-]{10,}\b",                      // Slack
        r"\bAIza[0-9A-Za-z_\-]{35}\b",                            // Google API
        r"\bglpat-[A-Za-z0-9_\-]{20,}\b",                         // GitLab
        r"\beyJ[A-Za-z0-9_-]{10,}\.eyJ[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}\b", // JWT
    ]
    .iter()
    .map(|p| Regex::new(p).unwrap())
    .collect()
});

pub fn luhn_valid(digits: &str) -> bool {
    let ds: Vec<u32> = digits.chars().filter_map(|c| c.to_digit(10)).collect();
    if ds.len() < 13 || ds.len() > 19 {
        return false;
    }
    let sum: u32 = ds
        .iter()
        .rev()
        .enumerate()
        .map(|(i, &d)| {
            if i % 2 == 1 {
                let x = d * 2;
                if x > 9 { x - 9 } else { x }
            } else {
                d
            }
        })
        .sum();
    sum % 10 == 0
}

pub fn iban_valid(candidate: &str) -> bool {
    let s: String = candidate.chars().filter(|c| !c.is_whitespace()).collect();
    if s.len() < 15 || s.len() > 34 {
        return false;
    }
    let rearranged = format!("{}{}", &s[4..], &s[..4]);
    let mut rem: u64 = 0;
    for c in rearranged.chars() {
        let v = match c {
            '0'..='9' => c as u64 - '0' as u64,
            'A'..='Z' => c as u64 - 'A' as u64 + 10,
            _ => return false,
        };
        rem = if v >= 10 {
            (rem * 100 + v) % 97
        } else {
            (rem * 10 + v) % 97
        };
    }
    rem == 1
}

/// Detect secrets in a piece of text. Only scans the first 64 KB.
pub fn detect_sensitive(text: &str) -> Option<Sensitive> {
    let t = if text.len() > 65_536 {
        let mut end = 65_536;
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        &text[..end]
    } else {
        text
    };
    if PRIVATE_KEY.is_match(t) {
        return Some(Sensitive::PrivateKey);
    }
    if API_KEYS.iter().any(|r| r.is_match(t)) {
        return Some(Sensitive::ApiKey);
    }
    if CARD_CANDIDATE.find_iter(t).any(|m| luhn_valid(m.as_str())) {
        return Some(Sensitive::CreditCard);
    }
    let upper = t.to_uppercase();
    if IBAN.find_iter(&upper).any(|m| iban_valid(m.as_str())) {
        return Some(Sensitive::Iban);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cards() {
        assert_eq!(
            detect_sensitive("4111 1111 1111 1111"),
            Some(Sensitive::CreditCard)
        );
        assert_eq!(
            detect_sensitive("my card 5500-0000-0000-0004 ok"),
            Some(Sensitive::CreditCard)
        );
        assert_eq!(detect_sensitive("4111 1111 1111 1112"), None, "fails Luhn");
        assert_eq!(detect_sensitive("order 12345"), None);
    }

    #[test]
    fn keys() {
        assert_eq!(
            detect_sensitive("AKIAIOSFODNN7EXAMPLE"),
            Some(Sensitive::ApiKey)
        );
        assert_eq!(
            detect_sensitive("ghp_abcdefghijklmnopqrstuvwxyz0123456789"),
            Some(Sensitive::ApiKey)
        );
        assert_eq!(
            detect_sensitive("sk-ant-api03-abcdefghijklmnopqrstuvwxyz"),
            Some(Sensitive::ApiKey)
        );
        assert_eq!(
            detect_sensitive(
                "-----BEGIN OPENSSH PRIVATE KEY-----\nabc\n-----END OPENSSH PRIVATE KEY-----"
            ),
            Some(Sensitive::PrivateKey)
        );
        assert_eq!(
            detect_sensitive("-----BEGIN PRIVATE KEY-----"),
            Some(Sensitive::PrivateKey)
        );
        assert_eq!(detect_sensitive("-----BEGIN PUBLIC KEY-----"), None);
    }

    #[test]
    fn ibans() {
        assert_eq!(
            detect_sensitive("IT60 X054 2811 1010 0000 0123 456"),
            Some(Sensitive::Iban)
        );
        assert_eq!(
            detect_sensitive("GB82WEST12345698765432"),
            Some(Sensitive::Iban)
        );
        assert_eq!(detect_sensitive("GB82WEST12345698765433"), None);
    }

    #[test]
    fn ordinary_text() {
        assert_eq!(
            detect_sensitive("Ciao, ci vediamo alle 18 in via Roma 12."),
            None
        );
        assert_eq!(detect_sensitive("https://example.com/path?id=42"), None);
    }
}
