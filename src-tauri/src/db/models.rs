//! Serialisable domain models shared with the Svelte frontend.
//!
//! All structs use `camelCase` on the wire so the TypeScript side needs no
//! translation layer.

use serde::Serialize;

/// A single dictionary headword with its senses.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DictEntry {
    pub id: i64,
    pub traditional: String,
    pub simplified: String,
    /// Raw numeric pinyin as published by CC-CEDICT (`ni3 hao3`).
    pub pinyin: String,
    /// Tone-marked pinyin for display (`nǐ hǎo`).
    pub pinyin_display: String,
    /// Tone-less pinyin for matching (`ni hao`).
    pub pinyin_plain: String,
    /// Character count of the simplified headword.
    pub word_length: i64,
    pub senses: Vec<DictSense>,
}

/// One definition within an entry. CC-CEDICT separates definitions with `/`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DictSense {
    /// 0-based position, matching the source order.
    pub position: i64,
    pub definition: String,
}

/// Snapshot of the embedded dictionary, used by the pre-check interstitial.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DictionaryStatus {
    /// True only when a completed import exists and is queryable.
    pub ready: bool,
    pub db_path: String,
    pub db_bytes: u64,
    pub schema_version: i64,
    pub entry_count: i64,
    pub sense_count: i64,
    /// Absolute path of the CC-CEDICT file the database was built from.
    pub source_path: Option<String>,
    /// SHA-256 of that file; lets a rebuild decide whether it is stale.
    pub source_sha256: Option<String>,
    pub source_bytes: Option<u64>,
    /// Unix epoch seconds of the last successful import.
    pub built_at_unix: Option<i64>,
    pub build_ms: Option<i64>,
    pub skipped_lines: Option<i64>,
    pub cedict_version: Option<String>,
    pub cedict_date: Option<String>,
    pub cedict_publisher: Option<String>,
    pub cedict_license: Option<String>,
}

/// Where the CC-CEDICT source file was looked for, and whether it was found.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DictionarySourceInfo {
    pub found: bool,
    pub path: Option<String>,
    pub bytes: Option<u64>,
    /// Every location that was probed, in priority order (useful for support).
    pub searched: Vec<String>,
}
