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
mod text;

// Public so examples and future integration tests can drive the core without
// booting a Tauri window. `commands`/`state` stay private: they are IPC glue.
pub mod db;
pub mod dictionary;
pub mod error;

use tauri::Manager;

use state::AppState;

/// GTK 3 draws GL-composited windows on Wayland through a 60 Hz frame clock no
/// matter how fast the display is (GNOME GTK issue #8461). WebKitGTK 4.1 is a
/// GTK 3 app, so its accelerated-compositing path inherits that cap. The Cairo
/// path follows the display's real refresh rate, which matters on high-refresh
/// laptops. QiaoReader is a text renderer, so the CPU trade-off is acceptable;
/// set `QIAOREADER_ALLOW_GPU_COMPOSITING=1` to keep GPU compositing instead.
#[cfg(target_os = "linux")]
fn configure_linux_rendering() {
    use std::env;

    if env::var_os("WAYLAND_DISPLAY").is_none() {
        return;
    }
    if env::var("GDK_BACKEND").is_ok_and(|backend| backend.eq_ignore_ascii_case("x11")) {
        return;
    }
    if env::var_os("QIAOREADER_ALLOW_GPU_COMPOSITING").is_some() {
        return;
    }
    // Respect an explicit override from the caller.
    if env::var_os("WEBKIT_DISABLE_COMPOSITING_MODE").is_some() {
        return;
    }

    // SAFETY: called before Tauri creates a window, spawns its runtime threads,
    // or initializes GTK/WebKit, so no other thread can be reading the
    // environment while it is mutated.
    unsafe {
        env::set_var("WEBKIT_DISABLE_COMPOSITING_MODE", "1");
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[cfg(target_os = "linux")]
    configure_linux_rendering();

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
            commands::text::process_raw_text,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
