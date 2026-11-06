//! Content type detection for textual clipboard payloads.

use crate::models::Kind;
use regex::Regex;
use std::sync::LazyLock as Lazy;

static HEX_COLOR: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^#(?:[0-9a-fA-F]{3}|[0-9a-fA-F]{4}|[0-9a-fA-F]{6}|[0-9a-fA-F]{8})$").unwrap()
});
static FN_COLOR: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^(?i)(rgba?|hsla?)\(\s*-?[\d.]+%?\s*[, ]\s*-?[\d.]+%?\s*[, ]\s*-?[\d.]+%?\s*(?:[,/]\s*[\d.]+%?\s*)?\)$")
        .unwrap()
});

pub fn is_color(s: &str) -> bool {
    let s = s.trim();
    s.len() <= 64 && (HEX_COLOR.is_match(s) || FN_COLOR.is_match(s))
}

/// A single URL with a scheme browsers understand and no whitespace.
pub fn is_link(s: &str) -> bool {
    let s = s.trim();
    if s.is_empty() || s.len() > 4096 || s.contains(char::is_whitespace) {
        return false;
    }
    match url::Url::parse(s) {
        Ok(u) => match u.scheme() {
            "http" | "https" | "ftp" => u
                .host_str()
                .map(|h| h.contains('.') || h == "localhost")
                .unwrap_or(false),
            "mailto" => s.contains('@'),
            _ => false,
        },
        Err(_) => false,
    }
}

struct Lang {
    name: &'static str,
    patterns: &'static [&'static str],
}

const LANGS: &[Lang] = &[
    Lang {
        name: "rust",
        patterns: &[
            "fn ", "let mut ", "impl ", "pub fn", "::", "-> ", "&self", "println!", "use std",
            "#[derive",
        ],
    },
    Lang {
        name: "typescript",
        patterns: &[
            "interface ",
            ": string",
            ": number",
            "export type",
            "import type",
            " as const",
            "readonly ",
        ],
    },
    Lang {
        name: "javascript",
        patterns: &[
            "const ",
            "let ",
            "function ",
            "=> ",
            "console.log",
            "require(",
            "export default",
            "import ",
            "document.",
        ],
    },
    Lang {
        name: "python",
        patterns: &[
            "def ", "import ", "self.", "elif ", "print(", "__init__", "from ", "None", "lambda ",
        ],
    },
    Lang {
        name: "swift",
        patterns: &[
            "func ",
            "import SwiftUI",
            "import Foundation",
            "guard let",
            "var body",
            "@State",
            "let ",
        ],
    },
    Lang {
        name: "go",
        patterns: &["func ", "package ", ":= ", "fmt.", "go func", "chan "],
    },
    Lang {
        name: "java",
        patterns: &[
            "public class",
            "public static void",
            "System.out",
            "private ",
            "@Override",
            "new ",
        ],
    },
    Lang {
        name: "csharp",
        patterns: &[
            "using System",
            "namespace ",
            "public class",
            "Console.Write",
            "var ",
        ],
    },
    Lang {
        name: "cpp",
        patterns: &["#include", "std::", "int main", "->", "nullptr", "printf("],
    },
    Lang {
        name: "php",
        patterns: &["<?php", "$this->", "echo ", "function "],
    },
    Lang {
        name: "ruby",
        patterns: &["def ", "end\n", "puts ", "require '", "do |"],
    },
    Lang {
        name: "sql",
        patterns: &[
            "SELECT ",
            "FROM ",
            "WHERE ",
            "INSERT INTO",
            "UPDATE ",
            "CREATE TABLE",
            "JOIN ",
            "select ",
            " from ",
        ],
    },
    Lang {
        name: "bash",
        patterns: &[
            "#!/bin/", "echo ", "export ", "$(", "fi\n", "then", "sudo ", "| grep",
        ],
    },
    Lang {
        name: "css",
        patterns: &[
            "{\n", ": ", ";\n", "px;", "color:", "margin", "display:", "@media",
        ],
    },
    Lang {
        name: "xml",
        patterns: &["</", "/>", "<?xml", "<div", "class=\""],
    },
];

/// Heuristic score of how much `text` looks like source code.
fn code_score(text: &str) -> i32 {
    let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
    if lines.is_empty() {
        return 0;
    }
    let mut score = 0;
    let n = lines.len() as i32;
    let ends_code = lines
        .iter()
        .filter(|l| {
            let t = l.trim_end();
            t.ends_with(';')
                || t.ends_with('{')
                || t.ends_with('}')
                || t.ends_with(')')
                || t.ends_with(':')
                || t.ends_with(',')
        })
        .count() as i32;
    let indented = lines
        .iter()
        .filter(|l| l.starts_with("  ") || l.starts_with('\t'))
        .count() as i32;
    score += ends_code * 10 / n.max(1) * 2;
    score += indented * 10 / n.max(1);
    let symbols = text.chars().filter(|c| "{}()[];=<>".contains(*c)).count() as i32;
    let chars = text.chars().count().max(1) as i32;
    score += (symbols * 100 / chars).min(15);
    let keywords = [
        "fn ", "function", "const ", "let ", "var ", "def ", "class ", "import ", "return", "if (",
        "for (", "while (", "=>", "#include", "public ", "private ", "SELECT ", "struct ", "func ",
        "package ", "#!/", "elif ", "<?php",
    ];
    score += keywords.iter().filter(|k| text.contains(*k)).count() as i32 * 4;
    // Prose penalty: long lines made of words ending with sentence punctuation.
    let prose = lines
        .iter()
        .filter(|l| {
            let t = l.trim_end();
            t.split_whitespace().count() > 8
                && (t.ends_with('.') || t.ends_with('?') || t.ends_with('!'))
        })
        .count() as i32;
    score -= prose * 20 / n.max(1);
    if n == 1 {
        score -= 8;
    }
    score
}

pub fn guess_language(text: &str) -> Option<&'static str> {
    guess_language_hits(text).map(|(n, _)| n)
}

fn guess_language_hits(text: &str) -> Option<(&'static str, usize)> {
    let t = text.trim();
    if (t.starts_with('{') && t.ends_with('}')) || (t.starts_with('[') && t.ends_with(']')) {
        if serde_json::from_str::<serde_json::Value>(t).is_ok() {
            return Some(("json", 10));
        }
    }
    if t.starts_with("<!DOCTYPE html") || t.starts_with("<html") {
        return Some(("html", 10));
    }
    let mut best: Option<(&str, usize)> = None;
    for lang in LANGS {
        let hits = lang.patterns.iter().filter(|p| t.contains(*p)).count();
        if hits >= 2 && best.map(|(_, b)| hits > b).unwrap_or(true) {
            best = Some((lang.name, hits));
        }
    }
    best
}

/// Decide the kind of a textual payload.
pub fn detect(text: &str, has_rich: bool) -> (Kind, Option<String>) {
    let trimmed = text.trim();
    if is_color(trimmed) {
        return (Kind::Color, None);
    }
    if is_link(trimmed) {
        return (Kind::Link, None);
    }
    let json = guess_language(trimmed) == Some("json") && trimmed.len() > 2;
    let score = code_score(trimmed);
    let strong_lang = score >= 10
        && trimmed.contains('\n')
        && guess_language_hits(trimmed)
            .map(|(_, h)| h >= 3)
            .unwrap_or(false);
    if json || score >= 22 || strong_lang {
        let lang = guess_language(trimmed).map(String::from);
        return (Kind::Code, lang);
    }
    if has_rich {
        return (Kind::Rich, None);
    }
    (Kind::Text, None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colors() {
        for c in [
            "#fff",
            "#6366F1",
            "#6366f1cc",
            "rgb(10, 20, 30)",
            "rgba(10,20,30,0.5)",
            "hsl(240 50% 50%)",
            "HSL(240, 50%, 50%)",
        ] {
            assert_eq!(detect(c, false).0, Kind::Color, "{c}");
        }
        for c in ["#ggg", "fff", "#12345", "rgb(1,2)"] {
            assert_ne!(detect(c, false).0, Kind::Color, "{c}");
        }
    }

    #[test]
    fn links() {
        assert_eq!(detect("https://example.com/a?b=c", false).0, Kind::Link);
        assert_eq!(detect("  http://localhost:3000  ", false).0, Kind::Link);
        assert_eq!(detect("mailto:me@example.com", false).0, Kind::Link);
        assert_ne!(detect("see https://example.com", false).0, Kind::Link);
        assert_ne!(detect("file:///etc/hosts", false).0, Kind::Link);
        assert_ne!(detect("javascript:alert(1)", false).0, Kind::Link);
    }

    #[test]
    fn code() {
        let rust = "fn main() {\n    let mut x = 5;\n    println!(\"{}\", x);\n}";
        assert_eq!(detect(rust, false), (Kind::Code, Some("rust".into())));
        let py = "def hello(name):\n    if name:\n        print(name)\n    return None";
        assert_eq!(detect(py, false), (Kind::Code, Some("python".into())));
        let js = "const add = (a, b) => {\n  return a + b;\n};\nconsole.log(add(1, 2));";
        assert_eq!(detect(js, false), (Kind::Code, Some("javascript".into())));
        assert_eq!(
            detect("{\"a\": 1, \"b\": [1,2]}", false),
            (Kind::Code, Some("json".into()))
        );
        let sql = "SELECT id, name\nFROM users\nWHERE id = 1;";
        assert_eq!(detect(sql, false).0, Kind::Code);
    }

    #[test]
    fn prose_is_text() {
        let prose = "Ciao Marco, ti scrivo per confermare l'appuntamento di domani alle dieci.\nFammi sapere se per te va ancora bene, altrimenti possiamo spostarlo.";
        assert_eq!(detect(prose, false).0, Kind::Text);
        assert_eq!(detect("hello world", false).0, Kind::Text);
        assert_eq!(detect("hello world", true).0, Kind::Rich);
        assert_eq!(detect("Lista: uno, due, tre", false).0, Kind::Text);
    }
}
