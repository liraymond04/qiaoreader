//! Embedded SQLite layer.
//!
//! [`Database`] owns one `rusqlite::Connection` to the app-data database file.
//! A single connection behind a mutex is deliberate: QiaoReader's reads are
//! point lookups measured in microseconds, while writes happen only during the
//! one-off dictionary import, so a pool would add complexity for no gain.

pub mod models;
pub mod queries;
pub mod schema;

use std::path::{Path, PathBuf};
use std::time::Duration;

use rusqlite::{Connection, OptionalExtension, params};

use crate::error::{AppError, AppResult};

use models::{DictEntry, DictionaryStatus};

/// File name of the dictionary database inside the app data directory.
pub const DB_FILE_NAME: &str = "dictionary.db";

pub struct Database {
    conn: Connection,
    path: PathBuf,
}

impl std::fmt::Debug for Database {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Database")
            .field("path", &self.path)
            .finish()
    }
}

impl Database {
    /// Open (creating if necessary) the dictionary database at `path`.
    pub fn open(path: impl AsRef<Path>) -> AppResult<Self> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        Self::from_connection(conn, path.to_path_buf())
    }

    /// In-memory database used by unit tests.
    #[cfg(test)]
    pub fn open_in_memory() -> AppResult<Self> {
        Self::from_connection(Connection::open_in_memory()?, PathBuf::from(":memory:"))
    }

    fn from_connection(conn: Connection, path: PathBuf) -> AppResult<Self> {
        // WAL keeps reads responsive if a future milestone queries while an
        // import is running. `journal_mode` returns a row, so use query_row.
        let _mode: String = conn.query_row("PRAGMA journal_mode = WAL", [], |row| row.get(0))?;
        conn.execute_batch(
            "PRAGMA foreign_keys = ON;
             PRAGMA synchronous = NORMAL;
             PRAGMA temp_store = MEMORY;
             PRAGMA busy_timeout = 5000;",
        )?;
        // `busy_timeout` is set both ways: the pragma above covers builds, the
        // API call covers connections created before a pragma is applied.
        conn.busy_timeout(Duration::from_secs(5))?;

        let db = Self { conn, path };
        db.migrate()?;
        Ok(db)
    }

    pub fn connection(&self) -> &Connection {
        &self.conn
    }

    pub fn connection_mut(&mut self) -> &mut Connection {
        &mut self.conn
    }

    /// Create or upgrade the schema. Refuses to touch databases produced by a
    /// newer build of the app.
    fn migrate(&self) -> AppResult<()> {
        let found: i64 = self
            .conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))?;

        if found > schema::SCHEMA_VERSION {
            return Err(AppError::SchemaTooNew {
                supported: schema::SCHEMA_VERSION,
                found,
            });
        }

        if found < schema::SCHEMA_VERSION {
            self.conn.execute_batch(schema::SCHEMA_SQL)?;
            self.conn
                .pragma_update(None, "user_version", schema::SCHEMA_VERSION)?;
            self.set_meta(
                schema::keys::SCHEMA_VERSION,
                &schema::SCHEMA_VERSION.to_string(),
            )?;
        }

        Ok(())
    }

    // ----------------------------------------------------------------- meta --

    pub fn meta(&self, key: &str) -> AppResult<Option<String>> {
        let value = self
            .conn
            .query_row(
                "SELECT value FROM meta WHERE key = ?1",
                params![key],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        Ok(value)
    }

    pub fn set_meta(&self, key: &str, value: &str) -> AppResult<()> {
        self.conn.execute(
            "INSERT INTO meta (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        Ok(())
    }

    pub fn meta_i64(&self, key: &str) -> AppResult<Option<i64>> {
        Ok(self
            .meta(key)?
            .and_then(|raw| raw.trim().parse::<i64>().ok()))
    }

    // --------------------------------------------------------------- counts --

    pub fn entry_count(&self) -> AppResult<i64> {
        let count = self
            .conn
            .query_row("SELECT COUNT(*) FROM entries", [], |row| row.get(0))?;
        Ok(count)
    }

    pub fn sense_count(&self) -> AppResult<i64> {
        let count = self
            .conn
            .query_row("SELECT COUNT(*) FROM senses", [], |row| row.get(0))?;
        Ok(count)
    }

    /// True when a completed import exists (the schema itself is always present).
    pub fn is_ready(&self) -> AppResult<bool> {
        let stored = self.meta_i64(schema::keys::ENTRY_COUNT)?.unwrap_or(0);
        if stored <= 0 {
            return Ok(false);
        }
        // Trust but verify: a partially rolled-back import leaves zero rows.
        Ok(self.entry_count()? > 0)
    }

    // --------------------------------------------------------------- status --

    pub fn status(&self) -> AppResult<DictionaryStatus> {
        let entry_count = self.entry_count()?;
        let sense_count = self.sense_count()?;
        let db_bytes = std::fs::metadata(&self.path)
            .map(|meta| meta.len())
            .unwrap_or(0);

        Ok(DictionaryStatus {
            ready: self.is_ready()?,
            db_path: self.path.display().to_string(),
            db_bytes,
            schema_version: schema::SCHEMA_VERSION,
            entry_count,
            sense_count,
            source_path: self.meta(schema::keys::SOURCE_PATH)?,
            source_sha256: self.meta(schema::keys::SOURCE_SHA256)?,
            source_bytes: self
                .meta_i64(schema::keys::SOURCE_BYTES)?
                .map(|bytes| bytes as u64),
            built_at_unix: self.meta_i64(schema::keys::BUILT_AT_UNIX)?,
            build_ms: self.meta_i64(schema::keys::BUILD_MS)?,
            skipped_lines: self.meta_i64(schema::keys::SKIPPED_LINES)?,
            cedict_version: self.meta(schema::keys::CEDICT_VERSION)?,
            cedict_date: self.meta(schema::keys::CEDICT_DATE)?,
            cedict_publisher: self.meta(schema::keys::CEDICT_PUBLISHER)?,
            cedict_license: self.meta(schema::keys::CEDICT_LICENSE)?,
        })
    }

    // -------------------------------------------------------------- lookups --

    /// Exact headword lookup against both simplified and traditional forms.
    ///
    /// This is the hot path used by `process_raw_text` (Milestone 1, next task):
    /// a segmented token goes in, all matching readings with their senses come
    /// out, ordered shortest first so single-character words win ties.
    pub fn lookup(&self, word: &str, limit: usize) -> AppResult<Vec<DictEntry>> {
        queries::lookup(&self.conn, word, limit)
    }

    /// User-facing search that accepts Chinese characters, numeric or tone-marked
    /// pinyin, or English gloss text.
    pub fn search(&self, query: &str, limit: usize) -> AppResult<Vec<DictEntry>> {
        queries::search(&self.conn, query, limit)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_schema_and_reports_empty_status() {
        let db = Database::open_in_memory().expect("open in-memory db");

        assert!(!db.is_ready().expect("ready check"));
        assert_eq!(db.entry_count().expect("entry count"), 0);

        let status = db.status().expect("status");
        assert!(!status.ready);
        assert_eq!(status.entry_count, 0);
        assert_eq!(status.schema_version, schema::SCHEMA_VERSION);

        // The schema version is mirrored in meta as text.
        assert_eq!(
            db.meta(schema::keys::SCHEMA_VERSION).expect("meta"),
            Some(schema::SCHEMA_VERSION.to_string())
        );
    }

    #[test]
    fn fts5_is_compiled_into_bundled_sqlite() {
        // If this fails the bundled build lost FTS5; search would silently break.
        let db = Database::open_in_memory().expect("open in-memory db");
        db.connection()
            .execute(
                "INSERT INTO entries
                     (id, traditional, simplified, pinyin, pinyin_display,
                      pinyin_plain, pinyin_compact, word_length, source_line)
                 VALUES (1, '你好', '你好', 'ni3 hao3', 'nǐ hǎo', 'ni hao', 'nihao', 2, 1)",
                [],
            )
            .expect("insert entry");
        db.connection()
            .execute(
                "INSERT INTO senses (id, entry_id, position, definition)
                 VALUES (1, 1, 0, 'hello; hi')",
                [],
            )
            .expect("insert sense");
        db.connection()
            .execute(
                "INSERT INTO search_index(search_index) VALUES('rebuild')",
                [],
            )
            .expect("rebuild fts");
        let hits: i64 = db
            .connection()
            .query_row(
                "SELECT COUNT(*) FROM search_index WHERE search_index MATCH 'hello'",
                [],
                |row| row.get(0),
            )
            .expect("fts query");
        assert_eq!(hits, 1);
    }

    #[test]
    fn meta_round_trips_and_updates() {
        let db = Database::open_in_memory().expect("open in-memory db");
        db.set_meta("unit.test", "one").expect("set");
        db.set_meta("unit.test", "two").expect("overwrite");
        assert_eq!(db.meta("unit.test").expect("get").as_deref(), Some("two"));
        assert_eq!(db.meta("unit.missing").expect("missing"), None);
    }
}
