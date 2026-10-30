//! Chinese text segmentation and dictionary annotation.
//!
//! The frontend sends raw text; Rust segments it with `jieba-rs`, looks up every
//! Han token in CC-CEDICT, and returns:
//!
//! * `segments` — the input in reading order, covering every byte, with a
//!   pointer to the unique word entry when the dictionary knows the token;
//! * `words` — the unique dictionary words in first-appearance order, each with
//!   all of its readings and senses.
//!
//! Deduplicating words keeps the IPC payload small (a page repeats 的, 我, 是…)
//! and gives the reference sidebar its chronological vocabulary list.

use std::collections::HashMap;

use jieba_rs::{Jieba, Token, TokenizeMode};
use serde::Serialize;

use crate::db::Database;
use crate::db::models::DictEntry;
use crate::db::queries::is_han;
use crate::error::AppResult;

/// Maximum readings returned for one segmented word.
const MAX_ENTRIES_PER_WORD: usize = 8;

/// Wrapper around `jieba-rs` so the rest of the core does not depend on its
/// concrete API. The default dictionary is embedded in the binary and the
/// structure is immutable after construction, so it is safe to share.
pub struct Segmenter {
    jieba: Jieba,
}

impl Segmenter {
    pub fn new() -> Self {
        Self {
            jieba: Jieba::new(),
        }
    }

    /// Segment in default (non-search) mode with HMM enabled for unknown words.
    pub fn tokenize<'a>(&'a self, text: &'a str) -> Vec<Token<'a>> {
        self.jieba.tokenize(text, TokenizeMode::Default, true)
    }
}

impl Default for Segmenter {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of processing a block of text.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessedText {
    pub segments: Vec<TextSegment>,
    /// Unique dictionary words, in first-appearance order.
    pub words: Vec<DictWord>,
}

/// One contiguous piece of the input, in reading order.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TextSegment {
    pub text: String,
    /// True for segmenter tokens (words); false for whitespace and punctuation
    /// that the segmenter skipped.
    pub is_word: bool,
    /// Index into [`ProcessedText::words`] when CC-CEDICT has an entry.
    pub word_index: Option<usize>,
}

/// A unique word and every reading the dictionary has for it.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DictWord {
    pub text: String,
    pub entries: Vec<DictEntry>,
}

/// Segment `text` and attach dictionary entries.
///
/// Gaps between segmenter tokens (spaces, punctuation) are emitted as
/// non-word segments so concatenating `segments[].text` reproduces the input.
pub fn process(segmenter: &Segmenter, db: &Database, text: &str) -> AppResult<ProcessedText> {
    let tokens = segmenter.tokenize(text);
    let mut processed = ProcessedText::default();
    let mut word_indices: HashMap<&str, usize> = HashMap::new();
    let mut cursor = 0usize;

    for token in tokens {
        let start = token.byte_start.min(text.len());
        let end = token.byte_end.min(text.len());

        if start > cursor {
            processed.segments.push(TextSegment {
                text: text[cursor..start].to_string(),
                is_word: false,
                word_index: None,
            });
        }

        if end <= cursor {
            continue;
        }

        let token_text = &text[start.max(cursor)..end];
        let has_han = token_text.chars().any(is_han);

        let word_index = if has_han {
            if let Some(index) = word_indices.get(token_text) {
                Some(*index)
            } else {
                let entries = db.lookup(token_text, MAX_ENTRIES_PER_WORD)?;
                if entries.is_empty() {
                    None
                } else {
                    let index = processed.words.len();
                    processed.words.push(DictWord {
                        text: token_text.to_string(),
                        entries,
                    });
                    word_indices.insert(token_text, index);
                    Some(index)
                }
            }
        } else {
            None
        };

        processed.segments.push(TextSegment {
            text: token_text.to_string(),
            is_word: has_han || word_index.is_some(),
            word_index,
        });

        cursor = end;
    }

    if cursor < text.len() {
        processed.segments.push(TextSegment {
            text: text[cursor..].to_string(),
            is_word: false,
            word_index: None,
        });
    }

    Ok(processed)
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::sync::OnceLock;

    use crate::db::Database;
    use crate::dictionary::importer;

    const SAMPLE: &str = "\
我 我 [wo3] /I/me/
愛 爱 [ai4] /to love/affection/
北京 北京 [Bei3 jing1] /Beijing/
天安門 天安门 [Tian1 an1 men2] /Tiananmen/
";

    fn segmenter() -> &'static Segmenter {
        static SEGMENTER: OnceLock<Segmenter> = OnceLock::new();
        SEGMENTER.get_or_init(Segmenter::new)
    }

    fn database() -> Database {
        let dir = tempfile::tempdir().expect("temp dir");
        let source = dir.path().join("cedict.txt");
        std::fs::write(&source, SAMPLE).expect("write fixture");
        let mut db = Database::open_in_memory().expect("in-memory db");
        importer::import(&mut db, &source, true, |_| {}).expect("import fixture");
        db
    }

    fn joined(processed: &ProcessedText) -> String {
        processed
            .segments
            .iter()
            .map(|segment| segment.text.as_str())
            .collect()
    }

    #[test]
    fn reproduces_the_input_exactly() {
        let db = database();
        let text = "我 爱北京！ Hello, 123.";

        let processed = process(segmenter(), &db, text).expect("process");

        assert_eq!(joined(&processed), text);
    }

    #[test]
    fn annotates_known_words_with_readings() {
        let db = database();

        let processed = process(segmenter(), &db, "我爱北京").expect("process");

        let beijing = processed
            .words
            .iter()
            .find(|word| word.text == "北京")
            .expect("北京 should be a dictionary word");
        assert_eq!(beijing.entries[0].pinyin_display, "Běi jīng");
        assert_eq!(beijing.entries[0].senses[0].definition, "Beijing");
    }

    #[test]
    fn deduplicates_repeated_words_but_keeps_segment_order() {
        let db = database();

        let processed = process(segmenter(), &db, "北京北京").expect("process");

        assert_eq!(processed.words.len(), 1);
        let indices: Vec<usize> = processed
            .segments
            .iter()
            .filter_map(|segment| segment.word_index)
            .collect();
        assert_eq!(indices, vec![0, 0]);
    }

    #[test]
    fn unknown_text_has_no_words() {
        let db = database();

        let processed = process(segmenter(), &db, "Hello, world!").expect("process");

        assert_eq!(joined(&processed), "Hello, world!");
        assert!(processed.words.is_empty());
    }

    #[test]
    fn empty_input_is_empty_output() {
        let db = database();

        let processed = process(segmenter(), &db, "").expect("process");

        assert!(processed.segments.is_empty());
        assert!(processed.words.is_empty());
    }
}
