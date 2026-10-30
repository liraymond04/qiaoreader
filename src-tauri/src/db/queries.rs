//! Read-side SQL.
//!
//! Every query returns [`DictEntry`] values with their senses attached, so the
//! Svelte layer never has to make a second round trip per word.

use std::collections::{HashMap, HashSet};

use rusqlite::{Connection, params, params_from_iter};

use crate::dictionary::pinyin;
use crate::error::AppResult;

use super::models::{DictEntry, DictSense};

/// Hard ceiling for any single lookup/search; protects the IPC channel.
pub const MAX_RESULTS: usize = 100;

const ENTRY_COLUMNS: &str =
    "e.id, e.traditional, e.simplified, e.pinyin, e.pinyin_display, e.pinyin_plain, e.word_length";

/// True for CJK ideographs (including extensions A–G), false for punctuation
/// and Latin text. Used to route a search query to the headword vs. FTS path.
pub fn is_han(c: char) -> bool {
    matches!(
        c as u32,
        0x3400..=0x4DBF      // CJK Unified Ideographs Extension A
        | 0x4E00..=0x9FFF    // CJK Unified Ideographs
        | 0xF900..=0xFAFF    // CJK Compatibility Ideographs
        | 0x20000..=0x2FA1F  // Extensions B–F
        | 0x30000..=0x3134F  // Extension G
    )
}

pub fn contains_han(text: &str) -> bool {
    text.chars().any(is_han)
}

/// Exact match on either script. Ordered by headword length then id so the most
/// common (shortest) reading is first.
pub fn lookup(conn: &Connection, word: &str, limit: usize) -> AppResult<Vec<DictEntry>> {
    let limit = clamp_limit(limit);
    let word = word.trim();
    if word.is_empty() {
        return Ok(Vec::new());
    }

    let sql = format!(
        "SELECT {ENTRY_COLUMNS}
           FROM entries e
          WHERE e.simplified = ?1 OR e.traditional = ?1
          ORDER BY e.word_length ASC,
                   -- CC-CEDICT capitalises proper nouns; show the common reading first.
                   CASE WHEN e.pinyin GLOB '[A-Z]*' THEN 1 ELSE 0 END ASC,
                   e.simplified ASC,
                   e.id ASC
          LIMIT ?2"
    );

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params![word, limit as i64], map_entry)?;
    let entries = rows.collect::<Result<Vec<_>, _>>()?;
    hydrate_senses(conn, entries)
}

/// Multi-strategy search used by the reference panel's search box.
///
/// * Han characters -> exact then prefix match on both scripts.
/// * Anything else -> pinyin prefix first (so typing `nihao` works), then FTS5
///   over English glosses, then a substring fallback for short queries.
pub fn search(conn: &Connection, query: &str, limit: usize) -> AppResult<Vec<DictEntry>> {
    let limit = clamp_limit(limit);
    let query = query.trim();
    if query.is_empty() {
        return Ok(Vec::new());
    }

    let mut collected: Vec<DictEntry> = Vec::new();
    let mut seen: HashSet<i64> = HashSet::new();

    if contains_han(query) {
        let sql = format!(
            "SELECT {ENTRY_COLUMNS}
               FROM entries e
              WHERE e.simplified = ?1
                 OR e.traditional = ?1
                 OR e.simplified LIKE ?1 || '%'
                 OR e.traditional LIKE ?1 || '%'
              ORDER BY CASE WHEN e.simplified = ?1 OR e.traditional = ?1 THEN 0 ELSE 1 END,
                       e.word_length ASC,
                       e.simplified ASC,
                       e.id ASC
              LIMIT ?2"
        );
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(params![query, limit as i64], map_entry)?;
        push_unique(
            &mut collected,
            &mut seen,
            rows.collect::<Result<Vec<_>, _>>()?,
        );
    } else {
        let (plain, compact) = pinyin::normalize_for_search(query);

        if !compact.is_empty() {
            let sql = format!(
                "SELECT {ENTRY_COLUMNS}
                   FROM entries e
                  WHERE e.pinyin_compact LIKE ?1 || '%'
                     OR e.pinyin_plain LIKE ?2 || '%'
                  ORDER BY e.word_length ASC, e.simplified ASC, e.id ASC
                  LIMIT ?3"
            );
            let mut stmt = conn.prepare(&sql)?;
            let rows = stmt.query_map(params![compact, plain, limit as i64], map_entry)?;
            push_unique(
                &mut collected,
                &mut seen,
                rows.collect::<Result<Vec<_>, _>>()?,
            );
        }

        if collected.len() < limit {
            let fts = build_fts_query(query);
            if !fts.is_empty() {
                let sql = format!(
                    "SELECT {ENTRY_COLUMNS}
                       FROM search_index
                       JOIN senses s ON s.id = search_index.rowid
                       JOIN entries e ON e.id = s.entry_id
                      WHERE search_index MATCH ?1
                      ORDER BY rank
                      LIMIT ?2"
                );
                // A malformed MATCH expression is user-facing input, not a bug:
                // treat it as "no matches" and let the LIKE fallback try.
                if let Ok(mut stmt) = conn.prepare(&sql) {
                    // One entry can match through several senses; over-fetch so
                    // de-duplication still leaves a full page of results.
                    let remaining = ((limit - collected.len()) * 4) as i64;
                    if let Ok(rows) = stmt.query_map(params![fts, remaining], map_entry) {
                        let hits = rows.collect::<Result<Vec<_>, _>>().unwrap_or_default();
                        push_unique(&mut collected, &mut seen, hits);
                    }
                }
            }
        }

        if collected.is_empty() {
            let sql = format!(
                "SELECT {ENTRY_COLUMNS}
                   FROM entries e
                  WHERE EXISTS (
                        SELECT 1 FROM senses s
                         WHERE s.entry_id = e.id
                           AND s.definition LIKE '%' || ?1 || '%'
                  )
                  ORDER BY e.word_length ASC, e.id ASC
                  LIMIT ?2"
            );
            let mut stmt = conn.prepare(&sql)?;
            let rows = stmt.query_map(params![query, limit as i64], map_entry)?;
            push_unique(
                &mut collected,
                &mut seen,
                rows.collect::<Result<Vec<_>, _>>()?,
            );
        }
    }

    collected.truncate(limit);
    hydrate_senses(conn, collected)
}

fn clamp_limit(limit: usize) -> usize {
    limit.clamp(1, MAX_RESULTS)
}

fn push_unique(into: &mut Vec<DictEntry>, seen: &mut HashSet<i64>, entries: Vec<DictEntry>) {
    for entry in entries {
        if seen.insert(entry.id) {
            into.push(entry);
        }
    }
}

fn map_entry(row: &rusqlite::Row<'_>) -> rusqlite::Result<DictEntry> {
    Ok(DictEntry {
        id: row.get(0)?,
        traditional: row.get(1)?,
        simplified: row.get(2)?,
        pinyin: row.get(3)?,
        pinyin_display: row.get(4)?,
        pinyin_plain: row.get(5)?,
        word_length: row.get(6)?,
        senses: Vec::new(),
    })
}

/// Attach senses in one extra query rather than one query per entry.
fn hydrate_senses(conn: &Connection, mut entries: Vec<DictEntry>) -> AppResult<Vec<DictEntry>> {
    if entries.is_empty() {
        return Ok(entries);
    }

    let ids: Vec<i64> = entries.iter().map(|entry| entry.id).collect();
    let placeholders = std::iter::repeat_n("?", ids.len())
        .collect::<Vec<_>>()
        .join(",");
    let sql = format!(
        "SELECT entry_id, position, definition
           FROM senses
          WHERE entry_id IN ({placeholders})
          ORDER BY entry_id ASC, position ASC"
    );

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params_from_iter(ids.iter()), |row| {
        Ok((
            row.get::<_, i64>(0)?,
            DictSense {
                position: row.get(1)?,
                definition: row.get(2)?,
            },
        ))
    })?;

    let mut by_entry: HashMap<i64, Vec<DictSense>> = HashMap::new();
    for row in rows {
        let (entry_id, sense) = row?;
        by_entry.entry(entry_id).or_default().push(sense);
    }

    for entry in &mut entries {
        if let Some(senses) = by_entry.remove(&entry.id) {
            entry.senses = senses;
        }
    }

    Ok(entries)
}

/// Build a safe FTS5 MATCH expression from free-form user input.
///
/// Every token is reduced to alphanumeric characters, so FTS5 operators
/// (`"`, `*`, `-`, `:`, `(`…) can never escape into the query grammar. Each
/// token becomes a prefix term, joined with `AND` for precision.
fn build_fts_query(input: &str) -> String {
    let tokens: Vec<String> = input
        .split(|c: char| !c.is_alphanumeric())
        .filter(|token| !token.is_empty())
        .map(|token| format!("{}*", token.to_lowercase()))
        .collect();
    tokens.join(" AND ")
}
