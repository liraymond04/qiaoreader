//! Build the embedded SQLite dictionary from CC-CEDICT without launching the
//! GUI. Useful for inspecting the database, benchmarking the import, or
//! preparing an artifact for a packaging step.
//!
//! ```sh
//! # Defaults: resources/cc-cedict/*.gz -> ./dictionary.db
//! cargo run --example build_dictionary
//!
//! # Explicit source and destination
//! cargo run --example build_dictionary -- /path/to/cedict.txt.gz /tmp/dict.db
//! ```

use std::path::{Path, PathBuf};

use qiaoreader_lib::db::Database;
use qiaoreader_lib::dictionary::importer::{self, ImportPhase};
use qiaoreader_lib::dictionary::source;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);

    let explicit = args.next().map(PathBuf::from);
    let destination = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("dictionary.db"));

    let source_path = match explicit {
        Some(path) => path,
        None => {
            let dev_resources = Path::new(env!("CARGO_MANIFEST_DIR")).join("resources");
            source::resolve_source(None, Path::new("."), None, Some(&dev_resources))?
        }
    };

    println!("source:      {}", source_path.display());
    println!("destination: {}", destination.display());

    let started = std::time::Instant::now();
    let mut db = Database::open(&destination)?;
    let outcome = importer::import(&mut db, &source_path, true, |progress| {
        if progress.phase == ImportPhase::Done {
            println!(
                "  read {} source lines, {} entries written",
                progress.processed, progress.entries
            );
        }
    })?;

    let status = db.status()?;
    println!("entries:     {}", status.entry_count);
    println!("senses:      {}", status.sense_count);
    println!("source sha:  {}", outcome.source.sha256);
    println!("cedict date: {:?}", status.cedict_date);
    println!("licence:     {:?}", status.cedict_license);
    println!("skipped:     {} line(s)", outcome.skipped_lines);
    println!(
        "database:    {:.1} MiB on disk",
        status.db_bytes as f64 / (1024.0 * 1024.0)
    );
    println!(
        "elapsed:     {} ms (import {})",
        started.elapsed().as_millis(),
        outcome.elapsed_ms
    );

    // Warm query sanity check so a broken schema fails loudly here, not in the UI.
    let entries = db.lookup("你好", 5)?;
    let first = entries
        .first()
        .ok_or("lookup for 你好 returned nothing after a successful import")?;
    println!(
        "lookup 你好: {} [{}] -> {} sense(s)",
        first.simplified,
        first.pinyin_display,
        first.senses.len()
    );

    Ok(())
}
