//! SQLite schema for the embedded CC-CEDICT dictionary.
//!
//! Design notes
//! ------------
//! * `entries` holds one row per CC-CEDICT line. The headword pair is stored in
//!   both scripts because segmentation output can be either, and lookups must
//!   hit both.
//! * `senses` normalises CC-CEDICT's slash-delimited definitions. Keeping one
//!   row per sense lets the UI render the numbered definition list without
//!   re-splitting strings in JavaScript, and lets later milestones attach
//!   per-sense metadata (e.g. Anki card state) without a schema rewrite.
//! * `search_index` is an external-content FTS5 index over `senses.definition`
//!   (it stores no second copy of the text). It powers English-gloss search.
//!   Chinese headwords and pinyin use the B-tree indexes instead: `unicode61`
//!   treats a whole headword as one token, while prefix `LIKE` on an indexed
//!   column is both smaller and faster for CJK.
//! * `meta` is a key/value table used by the pre-check interstitial: it records
//!   whether a build completed, which source file produced it (SHA-256), and
//!   the CC-CEDICT release metadata.
//! * `user_version` mirrors [`SCHEMA_VERSION`] so future milestones can migrate
//!   rather than guess.

/// Bump whenever [`SCHEMA_SQL`] changes shape. The migration hook in
/// `db::Database::migrate` refuses to open databases newer than this build.
pub const SCHEMA_VERSION: i64 = 1;

/// Prefix for keys written into the `meta` table.
pub mod keys {
    pub const SCHEMA_VERSION: &str = "schema_version";

    pub const SOURCE_PATH: &str = "dict.source_path";
    pub const SOURCE_SHA256: &str = "dict.source_sha256";
    pub const SOURCE_BYTES: &str = "dict.source_bytes";
    pub const SOURCE_MODIFIED_UNIX: &str = "dict.source_modified_unix";

    pub const ENTRY_COUNT: &str = "dict.entry_count";
    pub const SENSE_COUNT: &str = "dict.sense_count";
    pub const BUILT_AT_UNIX: &str = "dict.built_at_unix";
    pub const BUILD_MS: &str = "dict.build_ms";
    pub const SKIPPED_LINES: &str = "dict.skipped_lines";

    pub const CEDICT_VERSION: &str = "cedict.version";
    pub const CEDICT_SUBVERSION: &str = "cedict.subversion";
    pub const CEDICT_FORMAT: &str = "cedict.format";
    pub const CEDICT_DATE: &str = "cedict.date";
    pub const CEDICT_PUBLISHER: &str = "cedict.publisher";
    pub const CEDICT_LICENSE: &str = "cedict.license";
    pub const CEDICT_DECLARED_ENTRIES: &str = "cedict.declared_entries";
}

/// Full schema. Every statement is idempotent so it can be re-run on an
/// existing database during additive migrations.
pub const SCHEMA_SQL: &str = r#"
PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS meta (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
) WITHOUT ROWID;

CREATE TABLE IF NOT EXISTS entries (
    id             INTEGER PRIMARY KEY,
    traditional    TEXT    NOT NULL,
    simplified     TEXT    NOT NULL,
    -- Raw CC-CEDICT form, e.g. "ni3 hao3". Kept for lossless round-tripping.
    pinyin         TEXT    NOT NULL,
    -- Diacritic form for display, e.g. "nǐ hǎo".
    pinyin_display TEXT    NOT NULL,
    -- Tone-less, space separated romanisation for search, e.g. "ni hao".
    pinyin_plain   TEXT    NOT NULL,
    -- Tone-less, space-less romanisation for compact search, e.g. "nihao".
    pinyin_compact TEXT    NOT NULL,
    -- Character count of the simplified headword; used to order lookups
    -- (shortest/most common first) and to bound segmentation windows.
    word_length    INTEGER NOT NULL,
    -- 1-based line number in the source file, for diagnostics.
    source_line    INTEGER NOT NULL
);

-- One headword can have several pronunciations (e.g. 長 cháng/zhǎng), so the
-- uniqueness key includes pinyin. The pair is verified duplicate-free upstream.
CREATE UNIQUE INDEX IF NOT EXISTS idx_entries_headword_pinyin
    ON entries (traditional, simplified, pinyin);
CREATE INDEX IF NOT EXISTS idx_entries_simplified
    ON entries (simplified);
CREATE INDEX IF NOT EXISTS idx_entries_traditional
    ON entries (traditional);
CREATE INDEX IF NOT EXISTS idx_entries_pinyin_plain
    ON entries (pinyin_plain);
CREATE INDEX IF NOT EXISTS idx_entries_pinyin_compact
    ON entries (pinyin_compact);

CREATE TABLE IF NOT EXISTS senses (
    id         INTEGER PRIMARY KEY,
    entry_id   INTEGER NOT NULL REFERENCES entries (id) ON DELETE CASCADE,
    -- 0-based order within the entry, matching CC-CEDICT's "/a/b/c/" order.
    position   INTEGER NOT NULL,
    definition TEXT    NOT NULL,
    UNIQUE (entry_id, position)
);

-- External-content FTS5 index over `senses.definition`.
--
-- The index stores no copy of the text (it reads from `senses`), which keeps the
-- database ~10 MB smaller than a standalone FTS table. `rowid` is `senses.id`,
-- so a match resolves to an entry via `senses.entry_id`. `detail=column` gives
-- bm25 ranking without position lists, since QiaoReader does term/prefix
-- matching rather than phrase queries.
--
-- Rebuild it after an import with:
--   INSERT INTO search_index(search_index) VALUES('rebuild');
CREATE VIRTUAL TABLE IF NOT EXISTS search_index USING fts5(
    definition,
    content = 'senses',
    content_rowid = 'id',
    detail = 'column',
    tokenize = 'unicode61 remove_diacritics 2'
);
"#;
