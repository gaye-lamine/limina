//! R-07.5 (design 9.4): the doc comments of the crate avoid the words
//! "fraud", "lie", "scam" and "hoax". Whole words only, case-insensitive:
//! "supplied" or "believe" do not match. The `EngineError` Display texts are
//! checked by the unit tests in `src/error.rs`.

use std::fs;
use std::path::Path;

const BANNED: [&str; 4] = ["fraud", "lie", "scam", "hoax"];

/// Banned whole words found in `text` (lowercased), in order of appearance.
/// A word is a maximal run of ASCII letters.
fn banned_words_in(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_ascii_alphabetic())
        .map(str::to_ascii_lowercase)
        .filter(|w| BANNED.contains(&w.as_str()))
        .collect()
}

/// The doc-comment text of a source file: the content of every line whose
/// first non-blank characters are `///` or `//!`.
fn doc_lines(source: &str) -> Vec<(usize, String)> {
    source
        .lines()
        .enumerate()
        .filter_map(|(i, line)| {
            let t = line.trim_start();
            let rest = t.strip_prefix("///").or_else(|| t.strip_prefix("//!"))?;
            Some((i + 1, rest.to_string()))
        })
        .collect()
}

#[test]
fn word_matching_is_whole_word_and_case_insensitive() {
    assert_eq!(banned_words_in("a Lie."), vec!["lie"]);
    assert_eq!(
        banned_words_in("HOAX, scam; fraud"),
        vec!["hoax", "scam", "fraud"]
    );
    assert_eq!(banned_words_in("lie_down"), vec!["lie"]);
    assert!(banned_words_in("supplied believe lies fraudulent scammer").is_empty());
    assert!(banned_words_in("").is_empty());
}

#[test]
fn doc_line_extraction_keeps_only_doc_comments() {
    let src = "// lie in a plain comment\n    /// scam in a doc\nlet lie = 1;\n//! hoax inner\n";
    let lines = doc_lines(src);
    assert_eq!(
        lines,
        vec![
            (2, " scam in a doc".to_string()),
            (4, " hoax inner".to_string())
        ]
    );
}

#[test]
fn no_doc_comment_uses_a_banned_word() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let entries = fs::read_dir(&dir).expect("src directory is readable");
    let mut checked = 0;
    let mut offenders = Vec::new();
    for entry in entries {
        let path = entry.expect("directory entry is readable").path();
        if path.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }
        let source = fs::read_to_string(&path).expect("source file is readable");
        checked += 1;
        for (n, text) in doc_lines(&source) {
            for word in banned_words_in(&text) {
                offenders.push(format!("{}:{n}: {word}", path.display()));
            }
        }
    }
    assert!(checked > 0, "no source file was scanned");
    assert!(offenders.is_empty(), "banned words in docs: {offenders:#?}");
}

#[test]
fn banned_list_is_the_four_words_of_r_07_5() {
    assert_eq!(BANNED, ["fraud", "lie", "scam", "hoax"]);
}
