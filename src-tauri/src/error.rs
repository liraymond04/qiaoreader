//! Error type shared by the Rust core and the Tauri command layer.
//!
//! Every command returns `Result<T, AppError>`. Tauri needs the error type to
//! be `Serialize`, so it is rendered as a small, stable payload:
//!
//! ```json
//! { "code": "SOURCE_NOT_FOUND", "message": "CC-CEDICT source file not found; looked in: ..." }
//! ```
//!
//! The frontend should branch on `code` and only display `message`.

use std::path::PathBuf;

use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("CC-CEDICT source file not found; looked in: {searched:?}")]
    SourceNotFound { searched: Vec<PathBuf> },

    #[error("malformed CC-CEDICT data at line {line}: {reason}")]
    CedictParse { line: usize, reason: String },

    #[error("no usable CC-CEDICT entries were found in {path}")]
    EmptyDictionary { path: String },

    #[error("bad database location: {0}")]
    Path(String),

    #[error(
        "database schema version {found} is newer than the supported version {supported}; \
         please update QiaoReader"
    )]
    SchemaTooNew { supported: i64, found: i64 },

    #[error("background task failed: {0}")]
    Task(String),

    #[error("internal state lock was poisoned by a previous panic; restart QiaoReader")]
    LockPoisoned,
}

impl AppError {
    /// Stable machine-readable code, mirrored by the TypeScript client.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Sqlite(_) => "SQLITE",
            Self::Io(_) => "IO",
            Self::SourceNotFound { .. } => "SOURCE_NOT_FOUND",
            Self::CedictParse { .. } => "CEDICT_PARSE",
            Self::EmptyDictionary { .. } => "EMPTY_DICTIONARY",
            Self::Path(_) => "PATH",
            Self::SchemaTooNew { .. } => "SCHEMA_TOO_NEW",
            Self::Task(_) => "TASK",
            Self::LockPoisoned => "LOCK_POISONED",
        }
    }
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;

        let mut state = serializer.serialize_struct("AppError", 2)?;
        state.serialize_field("code", self.code())?;
        state.serialize_field("message", &self.to_string())?;
        state.end()
    }
}

/// Convenience alias used throughout the core.
pub type AppResult<T> = Result<T, AppError>;

/// Small helper so `?` can convert a path lookup failure into [`AppError::Path`].
pub trait PathErrorExt<T> {
    fn path_err(self, context: &str) -> AppResult<T>;
}

impl<T, E> PathErrorExt<T> for Result<T, E>
where
    E: std::fmt::Display,
{
    fn path_err(self, context: &str) -> AppResult<T> {
        self.map_err(|e| AppError::Path(format!("{context}: {e}")))
    }
}
