//! Tauri command surface.
//!
//! Commands are thin: they translate IPC arguments, take the database lock, and
//! delegate to the core modules. Anything expensive runs on a blocking pool so
//! the webview never freezes mid-import.

pub mod dictionary;
