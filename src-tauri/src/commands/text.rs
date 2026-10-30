//! Text processing commands.

use tauri::{AppHandle, Manager};

use crate::error::{AppError, AppResult};
use crate::state::AppState;
use crate::text::{ProcessedText, process as process_text};

/// Segment raw text with `jieba-rs` and annotate every known word with its
/// CC-CEDICT readings.
///
/// Runs on the blocking pool because long pages can take dozens of milliseconds
/// to segment and look up.
#[tauri::command]
pub async fn process_raw_text(app: AppHandle, text: String) -> AppResult<ProcessedText> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        let db = state.db()?;
        process_text(state.segmenter(), &db, &text)
    })
    .await
    .map_err(|error| AppError::Task(error.to_string()))?
}
