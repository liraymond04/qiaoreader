//! QiaoReader Rust core.
//!
//! Layer map:
//! * [`db`] — embedded SQLite (schema, connection, read queries),
//! * [`dictionary`] — CC-CEDICT parsing and import,
//! * [`commands`] — the IPC surface exposed to Svelte,
//! * [`state`] — shared application state,
//! * [`error`] — one error type with a stable `{ code, message }` payload.

mod commands;
mod state;

// Public so examples and future integration tests can drive the core without
// booting a Tauri window. `commands`/`state` stay private: they are IPC glue.
pub mod db;
pub mod dictionary;
pub mod error;

use tauri::Manager;

use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // Opening the database is cheap; the 125k-entry import is explicitly
            // triggered by the pre-check UI via `dictionary_build`.
            let state = AppState::initialize(app.handle())?;
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::dictionary::dictionary_status,
            commands::dictionary::dictionary_source,
            commands::dictionary::dictionary_build,
            commands::dictionary::dictionary_lookup,
            commands::dictionary::dictionary_search,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
