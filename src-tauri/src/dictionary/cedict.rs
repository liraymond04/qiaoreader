//! Parser for the CC-CEDICT text format.
//!
//! The file is UTF-8 with CRLF line endings. Each entry is a single line:
//!
//! ```text
//! 漢語 汉语 [han4 yu3] /Chinese language/
//! 好 好 [hao3] /good/well/proper/good to/easy to/very/so/(suffix indicating completion or readiness)/
//! ```
//!
//! * fields 1 and 2 are traditional and simplified headwords (no spaces),
//! * field 3 is bracketed, numeric-tone pinyin,
//! * the remainder is `/`-delimited definitions,
//! * `#` lines are comments and `#! key=value` lines are release metadata.
//!
//! Parsing is intentionally permissive about the *source* and strict about the
//! *line*: a malformed line yields an error carrying its line number so the
//! importer can skip it, report it, and keep the other 125 000 entries.

use std::collections::BTreeMap;

use crate::error::AppError;

use super::pinyin;

/// One parsed CC-CEDICT line, ready for insertion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CedictEntry {
    pub traditional: String,
    pub simplified: String,
    /// Raw numeric pinyin, e.g. `ni3 hao3`.
    pub pinyin: String,
    /// Tone-marked display form, e.g. `nǐ hǎo`.
    pub pinyin_display: String,
    /// Tone-less spaced form, e.g. `ni hao`.
    pub pinyin_plain: String,
    /// Tone-less compact form, e.g. `nihao`.
    pub pinyin_compact: String,
    pub word_length: i64,
    pub senses: Vec<String>,
}

/// Release metadata from the `#!` preamble.
#[derive(Debug, Clone, Default)]
pub struct CedictHeader {
    fields: BTreeMap<String, String>,
}

impl CedictHeader {
    pub fn insert(&mut self, key: &str, value: &str) {
        self.fields.insert(key.to_string(), value.to_string());
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.fields.get(key).map(String::as_str)
    }

    /// Number of entries the publisher says the file contains, for progress
    /// reporting and for sanity-checking the import.
    pub fn declared_entries(&self) -> Option<i64> {
        self.get("entries")?.trim().parse().ok()
    }
}

/// Parse a `#! key=value` line. Returns `None` for ordinary comments.
pub fn parse_header_line(line: &str) -> Option<(&str, &str)> {
    let rest = line.strip_prefix("#!")?;
    let (key, value) = rest.split_once('=')?;
    let key = key.trim();
    if key.is_empty() {
        return None;
    }
    Some((key, value.trim()))
}

/// True when a line carries no entry (`#` comment, `#!` metadata, or blank).
pub fn is_ignorable_line(line: &str) -> bool {
    let line = line.trim_end_matches(['\r', '\n']);
    line.is_empty() || line.starts_with('#')
}

/// Parse one dictionary line.
///
/// Returns `Err` with the supplied `line_no` when the line does not match the
/// documented grammar. Callers decide whether one bad line is fatal.
pub fn parse_entry_line(line: &str, line_no: usize) -> Result<CedictEntry, AppError> {
    let line = line.trim_end_matches(['\r', '\n']);

    let malformed = |reason: &str| AppError::CedictParse {
        line: line_no,
        reason: reason.to_string(),
    };

    let (traditional, rest) = line
        .split_once(' ')
        .ok_or_else(|| malformed("expected `traditional simplified [pinyin] /definitions/`"))?;
    let (simplified, rest) = rest
        .split_once(' ')
        .ok_or_else(|| malformed("missing simplified headword"))?;

    if traditional.is_empty() || simplified.is_empty() {
        return Err(malformed("empty headword"));
    }

    let rest = rest.trim_start();
    let rest = rest
        .strip_prefix('[')
        .ok_or_else(|| malformed("missing `[` before pinyin"))?;
    let (pinyin_raw, rest) = rest
        .split_once(']')
        .ok_or_else(|| malformed("missing `]` after pinyin"))?;
    let pinyin_raw = pinyin_raw.trim();
    if pinyin_raw.is_empty() {
        return Err(malformed("empty pinyin"));
    }

    let definitions = rest.trim_start();
    let definitions = definitions
        .strip_prefix('/')
        .ok_or_else(|| malformed("expected `/` before definitions"))?;
    // The list is slash-terminated; tolerate a missing final slash rather than
    // rejecting an otherwise usable entry.
    let definitions = definitions.strip_suffix('/').unwrap_or(definitions);

    let senses: Vec<String> = definitions
        .split('/')
        .map(str::trim)
        .filter(|sense| !sense.is_empty())
        .map(str::to_string)
        .collect();
    if senses.is_empty() {
        return Err(malformed("entry has no definitions"));
    }

    let pinyin_plain = pinyin::numeric_to_plain(pinyin_raw);
    let pinyin_display = pinyin::numeric_to_display(pinyin_raw);

    Ok(CedictEntry {
        traditional: traditional.to_string(),
        simplified: simplified.to_string(),
        pinyin: pinyin_raw.to_string(),
        pinyin_display,
        pinyin_compact: pinyin::compact(&pinyin_plain),
        pinyin_plain,
        word_length: simplified.chars().count() as i64,
        senses,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_typical_line() {
        let entry =
            parse_entry_line("漢語 汉语 [han4 yu3] /Chinese language/", 7).expect("parse entry");

        assert_eq!(entry.traditional, "漢語");
        assert_eq!(entry.simplified, "汉语");
        assert_eq!(entry.pinyin, "han4 yu3");
        assert_eq!(entry.pinyin_display, "hàn yǔ");
        assert_eq!(entry.pinyin_plain, "han yu");
        assert_eq!(entry.pinyin_compact, "hanyu");
        assert_eq!(entry.word_length, 2);
        assert_eq!(entry.senses, vec!["Chinese language"]);
    }

    #[test]
    fn splits_multiple_senses_in_order() {
        let entry = parse_entry_line("好 好 [hao3] /good/well/proper/good to/easy to/very/so/", 1)
            .expect("parse entry");

        assert_eq!(entry.senses.len(), 7);
        assert_eq!(entry.senses[0], "good");
        assert_eq!(entry.senses[6], "so");
    }

    #[test]
    fn tolerates_crlf_and_missing_trailing_slash() {
        let entry = parse_entry_line("好 好 [hao3] /good/well\r\n", 1).expect("parse entry");
        assert_eq!(entry.senses, vec!["good", "well"]);
    }

    #[test]
    fn parses_unromanised_acronyms() {
        let entry = parse_entry_line(
            "ACG ACG [A C G] /acronym for \"anime, comics and games\"/",
            1,
        )
        .expect("parse entry");
        assert_eq!(entry.pinyin, "A C G");
        assert_eq!(entry.pinyin_display, "A C G");
        assert_eq!(entry.word_length, 3);
        // The quoted text is part of the definition, not the grammar.
        assert_eq!(entry.senses[0], "acronym for \"anime, comics and games\"");
    }

    #[test]
    fn parses_umlaut_and_multiword_pinyin() {
        let entry = parse_entry_line(
            "旅遊不便保險 旅游不便保险 [lu : 3 you2 bu4 bian4 bao3 xian3] /travel insurance/",
            1,
        );
        // This malformed-in-the-wild spelling appears inside definitions only;
        // as a headword pinyin it should still parse without panicking.
        let entry = entry.expect("parse entry");
        assert_eq!(entry.pinyin, "lu : 3 you2 bu4 bian4 bao3 xian3");
        assert_eq!(entry.word_length, 6);
    }

    #[test]
    fn rejects_malformed_lines_with_line_numbers() {
        let err = parse_entry_line("no-brackets-here", 42).expect_err("must fail");
        match err {
            AppError::CedictParse { line, .. } => assert_eq!(line, 42),
            other => panic!("unexpected error: {other}"),
        }

        assert!(parse_entry_line("漢語 汉语 han4 yu3 /Chinese/", 1).is_err());
        assert!(parse_entry_line("漢語 汉语 [han4 yu3] Chinese", 1).is_err());
        assert!(parse_entry_line("漢語 汉语 [han4 yu3] //", 1).is_err());
        assert!(parse_entry_line("漢語 汉语 [] /Chinese/", 1).is_err());
    }

    #[test]
    fn recognises_comments_and_metadata() {
        assert!(is_ignorable_line("# CC-CEDICT"));
        assert!(is_ignorable_line("#! entries=125324\r"));
        assert!(is_ignorable_line(""));
        assert!(!is_ignorable_line("好 好 [hao3] /good/"));

        assert_eq!(
            parse_header_line("#! entries=125324"),
            Some(("entries", "125324"))
        );
        assert_eq!(
            parse_header_line("#! date=2026-10-29T01:05:57Z"),
            Some(("date", "2026-10-29T01:05:57Z"))
        );
        assert_eq!(parse_header_line("# just a comment"), None);
        assert_eq!(parse_header_line("#!"), None);
    }

    #[test]
    fn header_accumulates_lines() {
        let mut header = CedictHeader::default();
        for line in ["#! version=1", "#! subversion=0", "#! entries=125324"] {
            if let Some((key, value)) = parse_header_line(line) {
                header.insert(key, value);
            }
        }
        assert_eq!(header.get("version"), Some("1"));
        assert_eq!(header.get("subversion"), Some("0"));
        assert_eq!(header.declared_entries(), Some(125_324));
    }
}
