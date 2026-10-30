//! Locating and opening the CC-CEDICT source file.
//!
//! Search order (first hit wins):
//! 1. an explicit path from the caller,
//! 2. `$QIAOREADER_CC_CEDICT` (handy for development and CI),
//! 3. `<app data>/dictionary/<name>` (a copy the user dropped in),
//! 4. the Tauri resource directory (bundled app),
//! 5. `src-tauri/resources/cc-cedict/<name>` (running from `cargo`/`bun tauri dev`).
//!
//! Gzip is detected by magic bytes, not by extension, so
//! `cedict_1_0_ts_utf-8_mdbg.txt` and `.txt.gz` are both accepted.

use std::fs::File;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use flate2::read::GzDecoder;
use sha2::{Digest, Sha256};

use crate::error::{AppError, AppResult};

/// Accepted file names, in preference order. `.gz` first because that is what
/// upstream publishes.
pub const SOURCE_FILE_NAMES: [&str; 4] = [
    "cedict_1_0_ts_utf-8_mdbg.txt.gz",
    "cedict_1_0_ts_utf-8_mdbg.txt",
    "cc-cedict.txt.gz",
    "cc-cedict.txt",
];

/// Environment override used by tests and by developers who keep the dictionary
/// outside the repository.
pub const ENV_SOURCE_PATH: &str = "QIAOREADER_CC_CEDICT";

/// Facts about the file a database was (or will be) built from.
#[derive(Debug, Clone)]
pub struct SourceInfo {
    pub path: PathBuf,
    pub bytes: u64,
    pub modified_unix: Option<i64>,
    pub sha256: String,
}

/// Result of probing every known location.
#[derive(Debug, Clone)]
pub struct SourceLookup {
    pub found: Option<PathBuf>,
    pub searched: Vec<PathBuf>,
}

/// Build the ordered candidate list. Pure so it can be unit tested without Tauri.
pub fn candidate_paths(
    explicit: Option<&Path>,
    app_data_dir: &Path,
    resource_dir: Option<&Path>,
    dev_resources_dir: Option<&Path>,
) -> Vec<PathBuf> {
    let mut candidates: Vec<PathBuf> = Vec::new();

    if let Some(explicit) = explicit.filter(|path| !path.as_os_str().is_empty()) {
        candidates.push(explicit.to_path_buf());
    }

    if let Some(env_path) = std::env::var_os(ENV_SOURCE_PATH)
        .map(PathBuf::from)
        .filter(|path| !path.as_os_str().is_empty())
    {
        candidates.push(env_path);
    }

    let app_dict_dir = app_data_dir.join("dictionary");
    for name in SOURCE_FILE_NAMES {
        candidates.push(app_dict_dir.join(name));
    }

    if let Some(resource_dir) = resource_dir {
        for name in SOURCE_FILE_NAMES {
            candidates.push(resource_dir.join(name));
            candidates.push(resource_dir.join("cc-cedict").join(name));
            candidates.push(resource_dir.join("resources").join("cc-cedict").join(name));
        }
    }

    if let Some(dev_resources_dir) = dev_resources_dir {
        for name in SOURCE_FILE_NAMES {
            candidates.push(dev_resources_dir.join("cc-cedict").join(name));
        }
    }

    dedupe(candidates)
}

fn dedupe(candidates: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut unique: Vec<PathBuf> = Vec::with_capacity(candidates.len());
    for candidate in candidates {
        if !unique.contains(&candidate) {
            unique.push(candidate);
        }
    }
    unique
}

/// Probe every candidate without erroring; the caller decides how to present a
/// miss (the setup UI shows the searched list).
pub fn find_source(
    explicit: Option<&Path>,
    app_data_dir: &Path,
    resource_dir: Option<&Path>,
    dev_resources_dir: Option<&Path>,
) -> SourceLookup {
    let searched = candidate_paths(explicit, app_data_dir, resource_dir, dev_resources_dir);
    let found = searched.iter().find(|path| path.is_file()).cloned();
    SourceLookup { found, searched }
}

/// Like [`find_source`] but turns a miss into [`AppError::SourceNotFound`].
pub fn resolve_source(
    explicit: Option<&Path>,
    app_data_dir: &Path,
    resource_dir: Option<&Path>,
    dev_resources_dir: Option<&Path>,
) -> AppResult<PathBuf> {
    let lookup = find_source(explicit, app_data_dir, resource_dir, dev_resources_dir);
    lookup.found.ok_or(AppError::SourceNotFound {
        searched: lookup.searched,
    })
}

/// Read file metadata and hash the full file (SHA-256) so rebuilds can detect a
/// changed upstream release.
pub fn inspect(path: &Path) -> AppResult<SourceInfo> {
    let metadata = std::fs::metadata(path)?;
    let modified_unix = metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_secs() as i64);

    Ok(SourceInfo {
        path: path.to_path_buf(),
        bytes: metadata.len(),
        modified_unix,
        sha256: sha256_file(path)?,
    })
}

fn sha256_file(path: &Path) -> AppResult<String> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

/// Open the source as a line reader, transparently decompressing gzip.
pub fn open_reader(path: &Path) -> AppResult<Box<dyn BufRead + Send>> {
    let mut file = File::open(path)?;

    let mut magic = [0u8; 2];
    let read = fill(&mut file, &mut magic)?;
    file.seek(SeekFrom::Start(0))?;

    let buffered = BufReader::with_capacity(128 * 1024, file);
    if read == 2 && magic == [0x1f, 0x8b] {
        Ok(Box::new(BufReader::with_capacity(
            128 * 1024,
            GzDecoder::new(buffered),
        )))
    } else {
        Ok(Box::new(buffered))
    }
}

/// `Read::read` may return fewer bytes; keep reading until the buffer is full or
/// EOF so a two-byte magic number is reliable.
fn fill(reader: &mut File, buffer: &mut [u8]) -> std::io::Result<usize> {
    let mut total = 0;
    while total < buffer.len() {
        match reader.read(&mut buffer[total..]) {
            Ok(0) => break,
            Ok(read) => total += read,
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error),
        }
    }
    Ok(total)
}

/// Unix seconds, or `None` if the host clock is before the epoch.
pub fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn candidate_order_prefers_explicit_then_env_then_resources() {
        let app_data = Path::new("/app/data");
        let resources = Path::new("/app/resources");
        let dev = Path::new("/repo/src-tauri/resources");

        let candidates = candidate_paths(
            Some(Path::new("/custom/cedict.txt.gz")),
            app_data,
            Some(resources),
            Some(dev),
        );

        assert_eq!(candidates[0], PathBuf::from("/custom/cedict.txt.gz"));
        assert!(
            candidates
                .iter()
                .any(|path| path == &app_data.join("dictionary/cedict_1_0_ts_utf-8_mdbg.txt.gz"))
        );
        assert!(
            candidates
                .iter()
                .any(|path| path == &resources.join("cc-cedict/cedict_1_0_ts_utf-8_mdbg.txt.gz"))
        );
        assert!(
            candidates
                .iter()
                .any(|path| path == &dev.join("cc-cedict/cedict_1_0_ts_utf-8_mdbg.txt"))
        );
    }

    #[test]
    fn finds_plain_text_reader() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("cedict.txt");
        std::fs::write(&path, "好 好 [hao3] /good/\n").expect("write");

        let mut reader = open_reader(&path).expect("open");
        let mut line = String::new();
        reader.read_line(&mut line).expect("read");
        assert_eq!(line, "好 好 [hao3] /good/\n");
    }

    #[test]
    fn detects_gzip_by_magic_not_extension() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("dictionary.bin"); // deliberately unusual extension

        {
            let file = File::create(&path).expect("create");
            let mut encoder = flate2::write::GzEncoder::new(file, flate2::Compression::fast());
            encoder
                .write_all("好 好 [hao3] /good/\n".as_bytes())
                .expect("compress");
            encoder.finish().expect("finish");
        }

        let mut reader = open_reader(&path).expect("open");
        let mut line = String::new();
        reader.read_line(&mut line).expect("read");
        assert_eq!(line, "好 好 [hao3] /good/\n");
    }

    #[test]
    fn inspect_reports_size_and_hash() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("cedict.txt");
        std::fs::write(&path, b"abc").expect("write");

        let info = inspect(&path).expect("inspect");
        assert_eq!(info.bytes, 3);
        // SHA-256("abc")
        assert_eq!(
            info.sha256,
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert!(info.modified_unix.is_some());
    }
}
