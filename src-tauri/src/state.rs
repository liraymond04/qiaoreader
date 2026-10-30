//! Tauri application state: the single embedded dictionary connection plus the
//! filesystem locations needed to locate CC-CEDICT.

use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

use tauri::{AppHandle, Manager};

use crate::db::{DB_FILE_NAME, Database};
use crate::dictionary::source::{self, SourceLookup};
use crate::error::{AppError, AppResult, PathErrorExt};
use crate::text::Segmenter;

pub struct AppState {
    /// `<platform app data>/com.liray.qiaoreader` — writable location.
    pub app_data_dir: PathBuf,
    /// Tauri resource directory, present in bundled builds.
    pub resource_dir: Option<PathBuf>,
    db: Mutex<Database>,
    /// Loaded on the first `process_raw_text` so app start stays cheap.
    segmenter: OnceLock<Segmenter>,
}

impl AppState {
    /// Open (or create) the dictionary database and prepare the app state.
    pub fn initialize(app: &AppHandle) -> AppResult<Self> {
        let app_data_dir = app
            .path()
            .app_data_dir()
            .path_err("resolving the app data directory")?;
        std::fs::create_dir_all(&app_data_dir)?;

        let db = Database::open(app_data_dir.join(DB_FILE_NAME))?;
        let resource_dir = app.path().resource_dir().ok();

        Ok(Self {
            app_data_dir,
            resource_dir,
            db: Mutex::new(db),
            segmenter: OnceLock::new(),
        })
    }

    /// Lock the database. Commands hold this only for the duration of one query
    /// or one import, so lock contention is not a concern in practice.
    pub fn db(&self) -> AppResult<MutexGuard<'_, Database>> {
        self.db.lock().map_err(|_| AppError::LockPoisoned)
    }

    /// The shared segmenter, initialized on first use.
    pub fn segmenter(&self) -> &Segmenter {
        self.segmenter.get_or_init(Segmenter::new)
    }

    /// Probe every known location for the CC-CEDICT source file.
    pub fn find_source(&self, explicit: Option<&Path>) -> SourceLookup {
        let dev_resources = dev_resources_dir();
        source::find_source(
            explicit,
            &self.app_data_dir,
            self.resource_dir.as_deref(),
            dev_resources.as_deref(),
        )
    }

    /// Resolve the source file or fail with the searched-path list.
    pub fn resolve_source(&self, explicit: Option<&Path>) -> AppResult<PathBuf> {
        let dev_resources = dev_resources_dir();
        source::resolve_source(
            explicit,
            &self.app_data_dir,
            self.resource_dir.as_deref(),
            dev_resources.as_deref(),
        )
    }
}

/// During `cargo run` / `bun tauri dev` the bundled resource directory may not
/// exist yet, so also look at the crate's `resources/` folder. Only enabled in
/// debug builds so a release binary can never depend on the developer's tree.
fn dev_resources_dir() -> Option<PathBuf> {
    #[cfg(debug_assertions)]
    {
        Some(Path::new(env!("CARGO_MANIFEST_DIR")).join("resources"))
    }
    #[cfg(not(debug_assertions))]
    {
        None
    }
}
