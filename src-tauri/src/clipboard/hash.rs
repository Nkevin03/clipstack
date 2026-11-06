//! Content hashing used for de-duplication.

use sha2::{Digest, Sha256};

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Hash for textual items. Rich and plain copies of the same text de-duplicate together.
pub fn text_hash(text: &str) -> String {
    let mut h = Sha256::new();
    h.update(b"t:");
    h.update(text.as_bytes());
    hex(&h.finalize())
}

pub fn files_hash(paths: &[String]) -> String {
    let mut h = Sha256::new();
    h.update(b"f:");
    for p in paths {
        h.update(p.as_bytes());
        h.update([0u8]);
    }
    hex(&h.finalize())
}

pub fn bytes_hash(data: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(b"i:");
    h.update(data);
    hex(&h.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stable_and_distinct() {
        assert_eq!(text_hash("a"), text_hash("a"));
        assert_ne!(text_hash("a"), text_hash("b"));
        assert_ne!(text_hash("a"), bytes_hash(b"a"), "domains are separated");
        assert_eq!(text_hash("a").len(), 64);
        assert_ne!(
            files_hash(&["/a".into(), "/b".into()]),
            files_hash(&["/ab".into()])
        );
    }
}
