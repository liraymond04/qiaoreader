//! Dictionary commands exposed to Svelte via `invoke`.
//!
//! ```ts
//! import { invoke } from "@tauri-apps/api/core";
//!
//! const status = await invoke<DictionaryStatus>("dictionary_status");
//! await invoke<DictionaryBuildReport>("dictionary_build", { force: false });
//! const entries = await invoke<DictEntry[]>("dictionary_lookup", { word: "你好" });
//! const hits = await invoke<DictEntry[]>("dictionary_search", { query: "nihao", limit: 20 });
//! ```
//!
//! Events: `dictionary://import-progress` with an [`ImportProgress`] payload.

use std::path::PathBuf;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::db::models::{DictEntry, DictionarySourceInfo, DictionaryStatus};
use crate::dictionary::importer::{self, ImportOutcome};
use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// Emitted while the dictionary is imported; the payload is [`ImportProgress`].
pub const PROGRESS_EVENT: &str = "dictionary://import-progress";

/// Result of a build request, including the fresh status for the UI to adopt
/// without a follow-up round trip.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DictionaryBuildReport {
    pub status: DictionaryStatus,
    pub skipped: bool,
    pub entries_imported: i64,
    pub senses_imported: i64,
    pub skipped_lines: i64,
    pub warnings: Vec<String>,
    pub elapsed_ms: u64,
    pub source_path: String,
    pub source_sha256: String,
}

impl DictionaryBuildReport {
    fn from_outcome(status: DictionaryStatus, outcome: ImportOutcome) -> Self {
        Self {
            status,
            skipped: outcome.skipped,
            entries_imported: outcome.entries,
            senses_imported: outcome.senses,
            skipped_lines: outcome.skipped_lines,
            warnings: outcome.warnings,
            elapsed_ms: outcome.elapsed_ms,
            source_path: outcome.source.path,
            source_sha256: outcome.source.sha256,
        }
    }
}

/// Snapshot used by the pre-check interstitial.
#[tauri::command]
pub async fn dictionary_status(state: tauri::State<'_, AppState>) -> AppResult<DictionaryStatus> {
    state.db()?.status()
}

/// Report where CC-CEDICT was found (or every location probed).
#[tauri::command]
pub async fn dictionary_source(
    state: tauri::State<'_, AppState>,
) -> AppResult<DictionarySourceInfo> {
    let lookup = state.find_source(None);
    let bytes = lookup
        .found
        .as_deref()
        .and_then(|path| std::fs::metadata(path).ok())
        .map(|metadata| metadata.len());

    Ok(DictionarySourceInfo {
        found: lookup.found.is_some(),
        path: lookup
            .found
            .as_deref()
            .map(|path| path.display().to_string()),
        bytes,
        searched: lookup
            .searched
            .iter()
            .map(|path| path.display().to_string())
            .collect(),
    })
}

/// Parse CC-CEDICT into the embedded SQLite database.
///
/// Safe to call on every launch: if the source hash matches the stored one and
/// the database is populated, this returns immediately with `skipped: true`.
/// Pass `force: true` to rebuild regardless.
#[tauri::command]
pub async fn dictionary_build(
    app: AppHandle,
    source_path: Option<String>,
    force: Option<bool>,
) -> AppResult<DictionaryBuildReport> {
    let force = force.unwrap_or(false);
    let explicit: Option<PathBuf> = source_path
        .as_deref()
        .map(str::trim)
        .filter(|path| !path.is_empty())
        .map(PathBuf::from);

    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        let path = state.resolve_source(explicit.as_deref())?;

        let mut db = state.db()?;
        let outcome = importer::import(&mut db, &path, force, |progress| {
            let _ = app.emit(PROGRESS_EVENT, progress);
        })?;
        let status = db.status()?;
        drop(db);

        Ok(DictionaryBuildReport::from_outcome(status, outcome))
    })
    .await
    .map_err(|error| AppError::Task(error.to_string()))?
}

/// Exact headword lookup (simplified or traditional).
///
/// This is the call the JIT reading loop will make for each segmented token.
#[tauri::command]
pub async fn dictionary_lookup(
    state: tauri::State<'_, AppState>,
    word: String,
    limit: Option<u32>,
) -> AppResult<Vec<DictEntry>> {
    if word.trim().is_empty() {
        return Ok(Vec::new());
    }
    let limit = limit.unwrap_or(20) as usize;
    state.db()?.lookup(&word, limit)
}

/// Free-form search: Han characters, numeric/tone-marked pinyin, or English.
#[tauri::command]
pub async fn dictionary_search(
    state: tauri::State<'_, AppState>,
    query: String,
    limit: Option<u32>,
) -> AppResult<Vec<DictEntry>> {
    if query.trim().is_empty() {
        return Ok(Vec::new());
    }
    let limit = limit.unwrap_or(20) as usize;
    state.db()?.search(&query, limit)
}
