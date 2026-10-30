//! Streaming CC-CEDICT -> SQLite importer.
//!
//! The whole import runs in a single transaction: either the database contains
//! a complete, internally consistent dictionary, or the previous contents are
//! untouched. This matters for the pre-check interstitial, which must never see
//! a half-built dictionary.
//!
//! Performance shape: one prepared statement per table, one transaction, one
//! `read_until` per line. A 125 000-entry file imports in a couple of seconds on
//! a laptop and needs no temporary tables.

use std::io::BufRead;
use std::path::Path;
use std::time::Instant;

use rusqlite::params;
use serde::Serialize;

use crate::db::{Database, schema};
use crate::error::{AppError, AppResult};

use super::cedict::{self, CedictHeader};
use super::source::{self, SourceInfo};

/// Keep at most this many malformed-line diagnostics in the response; the file
/// holds 125k lines and a truncated file should not flood the IPC channel.
const MAX_WARNINGS: usize = 25;

/// How often (in source lines) progress is reported. ~2k lines is well under a
/// frame at import speed, so the UI never appears frozen.
const PROGRESS_EVERY: usize = 2_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ImportPhase {
    Opening,
    Reading,
    Writing,
    Finalizing,
    Done,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportProgress {
    pub phase: ImportPhase,
    /// Lines consumed so far.
    pub processed: i64,
    /// Entries declared by the file header, or 0 when unknown.
    pub total: i64,
    /// Entries written so far.
    pub entries: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceSummary {
    pub path: String,
    pub bytes: u64,
    pub sha256: String,
    pub modified_unix: Option<i64>,
}

impl From<&SourceInfo> for SourceSummary {
    fn from(info: &SourceInfo) -> Self {
        Self {
            path: info.path.display().to_string(),
            bytes: info.bytes,
            sha256: info.sha256.clone(),
            modified_unix: info.modified_unix,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportOutcome {
    /// True when the existing database already matched this exact source file
    /// and `force` was not requested.
    pub skipped: bool,
    pub entries: i64,
    pub senses: i64,
    pub skipped_lines: i64,
    pub warnings: Vec<String>,
    pub elapsed_ms: u64,
    pub source: SourceSummary,
}

/// Parse `source_path` and replace the dictionary contents.
///
/// * `force == false` and an unchanged source hash -> no work done, `skipped`.
/// * Any parse failure for individual lines is counted and reported; the import
///   only fails if *no* line can be used.
pub fn import<F>(
    db: &mut Database,
    source_path: &Path,
    force: bool,
    mut on_progress: F,
) -> AppResult<ImportOutcome>
where
    F: FnMut(&ImportProgress),
{
    let started = Instant::now();
    report(&mut on_progress, ImportPhase::Opening, 0, 0, 0);

    let source = source::inspect(source_path)?;

    if !force {
        let stored_sha = db.meta(schema::keys::SOURCE_SHA256)?;
        if stored_sha.as_deref() == Some(source.sha256.as_str()) && db.entry_count()? > 0 {
            let entries = db.entry_count()?;
            let senses = db.sense_count()?;
            return Ok(ImportOutcome {
                skipped: true,
                entries,
                senses,
                skipped_lines: db.meta_i64(schema::keys::SKIPPED_LINES)?.unwrap_or(0),
                warnings: Vec::new(),
                elapsed_ms: started.elapsed().as_millis() as u64,
                source: SourceSummary::from(&source),
            });
        }
    }

    let mut reader = source::open_reader(&source.path)?;

    // Bulk-import tuning. WAL is already on; NORMAL keeps crash safety without
    // an fsync per statement, which would dominate the runtime here.
    db.connection().execute_batch(
        "PRAGMA synchronous = NORMAL;
         PRAGMA temp_store = MEMORY;
         PRAGMA cache_size = -65536;",
    )?;

    let tx = db.connection_mut().transaction()?;
    // `search_index` is external-content over `senses`, so clearing the senses
    // is enough; it is rebuilt from them below.
    tx.execute_batch("DELETE FROM senses; DELETE FROM entries;")?;

    let mut header = CedictHeader::default();
    let mut extra_warnings: usize = 0;
    let mut line_no: usize = 0;
    let mut declared_entries: i64 = 0;
    let mut entries_written: i64 = 0;
    let mut senses_written: i64 = 0;
    let mut skipped_lines: i64 = 0;
    let mut warnings: Vec<String> = Vec::new();
    let mut buffer: Vec<u8> = Vec::with_capacity(2048);

    {
        let mut entry_stmt = tx.prepare(
            "INSERT INTO entries
                 (id, traditional, simplified, pinyin, pinyin_display,
                  pinyin_plain, pinyin_compact, word_length, source_line)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        )?;
        let mut sense_stmt =
            tx.prepare("INSERT INTO senses (entry_id, position, definition) VALUES (?1, ?2, ?3)")?;

        loop {
            buffer.clear();
            let read = reader.read_until(b'\n', &mut buffer)?;
            if read == 0 {
                break;
            }
            line_no += 1;

            // Lossy decoding keeps one corrupt byte from aborting a 125k-line
            // import; malformed lines are counted just below.
            let raw = String::from_utf8_lossy(&buffer);
            let line = raw.trim_end_matches(['\r', '\n']);

            if let Some((key, value)) = cedict::parse_header_line(line) {
                header.insert(key, value);
                if key == "entries" {
                    declared_entries = header.declared_entries().unwrap_or(0);
                }
                continue;
            }
            if cedict::is_ignorable_line(line) {
                continue;
            }

            match cedict::parse_entry_line(line, line_no) {
                Ok(entry) => {
                    entries_written += 1;
                    let id = entries_written;

                    entry_stmt.execute(params![
                        id,
                        entry.traditional,
                        entry.simplified,
                        entry.pinyin,
                        entry.pinyin_display,
                        entry.pinyin_plain,
                        entry.pinyin_compact,
                        entry.word_length,
                        line_no as i64,
                    ])?;

                    for (position, definition) in entry.senses.iter().enumerate() {
                        sense_stmt.execute(params![id, position as i64, definition])?;
                        senses_written += 1;
                    }
                }
                Err(error) => {
                    skipped_lines += 1;
                    if warnings.len() < MAX_WARNINGS {
                        warnings.push(error.to_string());
                    } else {
                        extra_warnings += 1;
                    }
                }
            }

            if line_no.is_multiple_of(PROGRESS_EVERY) {
                report(
                    &mut on_progress,
                    ImportPhase::Reading,
                    line_no as i64,
                    declared_entries,
                    entries_written,
                );
            }
        }
    }

    if entries_written == 0 {
        // `tx` is dropped here, rolling back, so the old dictionary survives.
        return Err(AppError::EmptyDictionary {
            path: source.path.display().to_string(),
        });
    }

    // Populate the external-content FTS index in one pass once every sense is
    // in place. This is substantially cheaper than indexing row by row.
    tx.execute(
        "INSERT INTO search_index(search_index) VALUES('rebuild')",
        [],
    )?;

    if extra_warnings > 0 {
        warnings.push(format!(
            "{extra_warnings} additional malformed line(s) were skipped"
        ));
    }

    // Final reading tick covers files smaller than PROGRESS_EVERY lines.
    report(
        &mut on_progress,
        ImportPhase::Reading,
        line_no as i64,
        declared_entries,
        entries_written,
    );
    report(
        &mut on_progress,
        ImportPhase::Writing,
        line_no as i64,
        declared_entries,
        entries_written,
    );

    let mut meta: Vec<(&str, String)> = vec![
        (schema::keys::SOURCE_PATH, source.path.display().to_string()),
        (schema::keys::SOURCE_SHA256, source.sha256.clone()),
        (schema::keys::SOURCE_BYTES, source.bytes.to_string()),
        (schema::keys::ENTRY_COUNT, entries_written.to_string()),
        (schema::keys::SENSE_COUNT, senses_written.to_string()),
        (schema::keys::BUILT_AT_UNIX, source::now_unix().to_string()),
        (schema::keys::SKIPPED_LINES, skipped_lines.to_string()),
    ];

    if let Some(modified) = source.modified_unix {
        meta.push((schema::keys::SOURCE_MODIFIED_UNIX, modified.to_string()));
    }

    for (key, value) in [
        (schema::keys::CEDICT_VERSION, header.get("version")),
        (schema::keys::CEDICT_SUBVERSION, header.get("subversion")),
        (schema::keys::CEDICT_FORMAT, header.get("format")),
        (schema::keys::CEDICT_DATE, header.get("date")),
        (schema::keys::CEDICT_PUBLISHER, header.get("publisher")),
        (schema::keys::CEDICT_LICENSE, header.get("license")),
    ] {
        if let Some(value) = value {
            meta.push((key, value.to_string()));
        }
    }
    if declared_entries > 0 {
        meta.push((
            schema::keys::CEDICT_DECLARED_ENTRIES,
            declared_entries.to_string(),
        ));
    }

    for (key, value) in &meta {
        tx.execute(
            "INSERT INTO meta (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
    }

    report(
        &mut on_progress,
        ImportPhase::Finalizing,
        line_no as i64,
        declared_entries,
        entries_written,
    );

    tx.commit()?;

    // Build statistics for the query planner, then fold the WAL back into the
    // main file so the on-disk database is one self-contained artifact.
    db.connection()
        .execute_batch("PRAGMA optimize; ANALYZE; PRAGMA wal_checkpoint(TRUNCATE);")?;

    let elapsed_ms = started.elapsed().as_millis() as u64;
    db.set_meta(schema::keys::BUILD_MS, &elapsed_ms.to_string())?;

    report(
        &mut on_progress,
        ImportPhase::Done,
        line_no as i64,
        declared_entries,
        entries_written,
    );

    Ok(ImportOutcome {
        skipped: false,
        entries: entries_written,
        senses: senses_written,
        skipped_lines,
        warnings,
        elapsed_ms,
        source: SourceSummary::from(&source),
    })
}

fn report<F>(on_progress: &mut F, phase: ImportPhase, processed: i64, total: i64, entries: i64)
where
    F: FnMut(&ImportProgress),
{
    on_progress(&ImportProgress {
        phase,
        processed,
        total,
        entries,
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;
    use std::path::PathBuf;

    const SAMPLE: &str = "# CC-CEDICT\n\
#! version=1\n\
#! subversion=0\n\
#! entries=4\n\
#! date=2026-10-29T01:05:57Z\n\
#! publisher=MDBG\n\
#! license=https://creativecommons.org/licenses/by-sa/4.0/\n\
你好 你好 [ni3 hao3] /hello/hi/\n\
漢語 汉语 [han4 yu3] /Chinese language/\n\
好 好 [hao3] /good/well/\n\
愛 爱 [ai4] /to love/affection/\n\
this line is malformed\n";

    fn write_sample(dir: &Path, name: &str, contents: &str) -> PathBuf {
        let path = dir.join(name);
        std::fs::write(&path, contents).expect("write sample source");
        path
    }

    fn fresh_db(dir: &Path) -> Database {
        Database::open(dir.join("dictionary.db")).expect("open test database")
    }

    #[test]
    fn imports_sample_and_records_metadata() {
        let dir = tempfile::tempdir().expect("temp dir");
        let source = write_sample(dir.path(), "cedict.txt", SAMPLE);
        let mut db = fresh_db(dir.path());

        let outcome = import(&mut db, &source, false, |_| {}).expect("import");

        assert!(!outcome.skipped);
        assert_eq!(outcome.entries, 4);
        assert_eq!(outcome.senses, 7);
        assert_eq!(outcome.skipped_lines, 1);
        assert_eq!(outcome.warnings.len(), 1);
        assert!(
            outcome.warnings[0].contains("line 12"),
            "{:?}",
            outcome.warnings
        );

        assert_eq!(db.entry_count().expect("count"), 4);
        assert_eq!(db.sense_count().expect("count"), 7);
        assert!(db.is_ready().expect("ready"));

        let status = db.status().expect("status");
        assert!(status.ready);
        assert_eq!(status.entry_count, 4);
        assert_eq!(status.sense_count, 7);
        assert_eq!(status.cedict_version.as_deref(), Some("1"));
        assert_eq!(status.cedict_publisher.as_deref(), Some("MDBG"));
        assert_eq!(status.cedict_date.as_deref(), Some("2026-10-29T01:05:57Z"));
        assert_eq!(
            status.cedict_license.as_deref(),
            Some("https://creativecommons.org/licenses/by-sa/4.0/")
        );
        assert_eq!(
            status.source_path.as_deref(),
            Some(source.display().to_string().as_str())
        );
        assert!(status.built_at_unix.is_some());
        assert!(status.source_sha256.is_some());
    }

    #[test]
    fn import_is_idempotent_until_forced() {
        let dir = tempfile::tempdir().expect("temp dir");
        let source = write_sample(dir.path(), "cedict.txt", SAMPLE);
        let mut db = fresh_db(dir.path());

        import(&mut db, &source, false, |_| {}).expect("first import");

        let second = import(&mut db, &source, false, |_| {}).expect("second import");
        assert!(second.skipped);
        assert_eq!(second.entries, 4);

        let forced = import(&mut db, &source, true, |_| {}).expect("forced import");
        assert!(!forced.skipped);
        assert_eq!(forced.entries, 4);
        assert_eq!(db.entry_count().expect("count"), 4);
    }

    #[test]
    fn failed_import_preserves_previous_dictionary() {
        let dir = tempfile::tempdir().expect("temp dir");
        let source = write_sample(dir.path(), "source.txt", SAMPLE);
        let empty = write_sample(dir.path(), "empty.txt", "# only comments\n#! entries=0\n");
        let mut db = fresh_db(dir.path());

        import(&mut db, &source, false, |_| {}).expect("first import");
        let error = import(&mut db, &empty, true, |_| {}).expect_err("empty import must fail");
        assert!(matches!(error, AppError::EmptyDictionary { .. }));

        // The transaction rolled back: the old dictionary is still queryable.
        assert_eq!(db.entry_count().expect("count"), 4);
        assert_eq!(db.lookup("汉语", 10).expect("lookup").len(), 1);
    }

    #[test]
    fn lookup_matches_both_scripts_and_orders_results() {
        let dir = tempfile::tempdir().expect("temp dir");
        let source = write_sample(dir.path(), "cedict.txt", SAMPLE);
        let mut db = fresh_db(dir.path());
        import(&mut db, &source, false, |_| {}).expect("import");

        let simplified = db.lookup("汉语", 10).expect("lookup simplified");
        assert_eq!(simplified.len(), 1);
        assert_eq!(simplified[0].traditional, "漢語");
        assert_eq!(simplified[0].pinyin, "han4 yu3");
        assert_eq!(simplified[0].pinyin_display, "hàn yǔ");
        assert_eq!(simplified[0].senses[0].definition, "Chinese language");
        assert_eq!(simplified[0].word_length, 2);

        let traditional = db.lookup("漢語", 10).expect("lookup traditional");
        assert_eq!(traditional.len(), 1);
        assert_eq!(traditional[0].id, simplified[0].id);

        assert!(db.lookup("不存在", 10).expect("miss").is_empty());
        assert!(db.lookup("   ", 10).expect("blank").is_empty());
    }

    #[test]
    fn search_handles_han_pinyin_and_english() {
        let dir = tempfile::tempdir().expect("temp dir");
        let source = write_sample(dir.path(), "cedict.txt", SAMPLE);
        let mut db = fresh_db(dir.path());
        import(&mut db, &source, false, |_| {}).expect("import");

        let by_han = db.search("汉", 10).expect("han search");
        assert_eq!(by_han.len(), 1);
        assert_eq!(by_han[0].simplified, "汉语");

        let by_pinyin_spaced = db.search("ni hao", 10).expect("pinyin search");
        assert!(
            by_pinyin_spaced
                .iter()
                .any(|entry| entry.simplified == "你好")
        );

        let by_pinyin_compact = db.search("hanyu", 10).expect("compact pinyin search");
        assert!(
            by_pinyin_compact
                .iter()
                .any(|entry| entry.simplified == "汉语")
        );

        let by_numeric_pinyin = db.search("ni3hao3", 10).expect("numeric pinyin search");
        assert!(
            by_numeric_pinyin
                .iter()
                .any(|entry| entry.simplified == "你好")
        );

        let by_english = db.search("love", 10).expect("english search");
        assert_eq!(by_english.len(), 1);
        assert_eq!(by_english[0].simplified, "爱");
        assert_eq!(by_english[0].senses.len(), 2);

        let by_english_phrase = db.search("Chinese language", 10).expect("phrase search");
        assert!(
            by_english_phrase
                .iter()
                .any(|entry| entry.simplified == "汉语")
        );

        // FTS operators in user input must not produce SQL/parser errors.
        let hostile = db.search("\" OR 1=1 -- *", 10).expect("hostile query");
        assert!(hostile.is_empty());
    }

    #[test]
    fn imports_crlf_and_gzip_sources() {
        let dir = tempfile::tempdir().expect("temp dir");
        let crlf = SAMPLE.replace('\n', "\r\n");
        let plain = write_sample(dir.path(), "crlf.txt", &crlf);
        let mut db = fresh_db(dir.path());

        let outcome = import(&mut db, &plain, false, |_| {}).expect("crlf import");
        assert_eq!(outcome.entries, 4);
        assert_eq!(outcome.skipped_lines, 1);

        let gz_path = dir.path().join("cedict.txt.gz");
        {
            use std::io::Write;
            let file = std::fs::File::create(&gz_path).expect("create gz");
            let mut encoder = flate2::write::GzEncoder::new(file, flate2::Compression::fast());
            encoder.write_all(SAMPLE.as_bytes()).expect("gzip write");
            encoder.finish().expect("gzip finish");
        }

        let mut gz_db = Database::open(dir.path().join("gz.db")).expect("open gz db");
        let gz_outcome = import(&mut gz_db, &gz_path, false, |_| {}).expect("gzip import");
        assert_eq!(gz_outcome.entries, 4);
        assert_eq!(gz_db.lookup("爱", 10).expect("lookup").len(), 1);
    }

    #[test]
    fn reports_progress_through_all_phases() {
        let dir = tempfile::tempdir().expect("temp dir");
        let source = write_sample(dir.path(), "cedict.txt", SAMPLE);
        let mut db = fresh_db(dir.path());

        let mut phases = Vec::new();
        import(&mut db, &source, false, |progress| {
            phases.push(progress.phase);
        })
        .expect("import");

        assert_eq!(phases.first(), Some(&ImportPhase::Opening));
        assert!(phases.contains(&ImportPhase::Reading));
        assert!(phases.contains(&ImportPhase::Writing));
        assert!(phases.contains(&ImportPhase::Finalizing));
        assert_eq!(phases.last(), Some(&ImportPhase::Done));
    }

    /// Full-corpus verification. Ignored by default because it writes ~40 MB and
    /// takes a few seconds:
    ///
    /// ```sh
    /// cargo test --lib imports_the_real_cc_cedict_file -- --ignored --nocapture
    /// ```
    #[test]
    #[ignore = "imports the full 125k-entry CC-CEDICT file"]
    fn imports_the_real_cc_cedict_file() {
        let source = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("resources/cc-cedict/cedict_1_0_ts_utf-8_mdbg.txt.gz");
        if !source.is_file() {
            eprintln!("SKIP: {} is not present", source.display());
            return;
        }

        let dir = tempfile::tempdir().expect("temp dir");
        let mut db = Database::open(dir.path().join("full.db")).expect("open full db");

        let mut last_processed = 0;
        let outcome = import(&mut db, &source, false, |progress| {
            last_processed = progress.processed;
        })
        .expect("full import");

        eprintln!(
            "imported {} entries / {} senses in {} ms ({} skipped lines)",
            outcome.entries, outcome.senses, outcome.elapsed_ms, outcome.skipped_lines
        );
        assert!(outcome.entries > 100_000, "entries: {}", outcome.entries);
        assert_eq!(outcome.skipped_lines, 0);
        assert!(last_processed > 0);

        let status = db.status().expect("status");
        assert!(status.ready);
        assert_eq!(status.entry_count, outcome.entries);
        assert_eq!(status.sense_count, outcome.senses);
        assert_eq!(status.cedict_publisher.as_deref(), Some("MDBG"));
        assert!(status.cedict_date.is_some());

        // Spot-check real entries across scripts and tones.
        let nihao = db.lookup("你好", 10).expect("lookup 你好");
        assert_eq!(nihao[0].pinyin_display, "nǐ hǎo");
        assert!(
            nihao[0]
                .senses
                .iter()
                .any(|sense| sense.definition.contains("hello"))
        );

        let traditional = db.lookup("漢語", 10).expect("lookup 漢語");
        assert!(
            traditional
                .iter()
                .any(|entry| entry.simplified == "汉语" && entry.pinyin == "Han4 yu3")
        );

        let umlaut = db.lookup("女", 10).expect("lookup 女");
        assert!(umlaut.iter().any(|entry| entry.pinyin_display == "nǚ"));

        let english = db.search("dictionary", 10).expect("search dictionary");
        assert!(!english.is_empty());

        let pinyin = db.search("xiexie", 10).expect("search xiexie");
        assert!(pinyin.iter().any(|entry| entry.simplified == "谢谢"));
    }
}
