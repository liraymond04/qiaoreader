//! CC-CEDICT dictionary subsystem.
//!
//! * [`cedict`] — grammar for the source file,
//! * [`pinyin`] — numeric tone conversion and search normalisation,
//! * [`source`] — locating/decompressing the file,
//! * [`importer`] — streaming the file into SQLite.

pub mod cedict;
pub mod importer;
pub mod pinyin;
pub mod source;
