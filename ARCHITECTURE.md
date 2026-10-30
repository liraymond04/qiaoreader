# Architecture

QiaoReader is a Tauri 2 application. The Rust core owns the dictionary, text processing, and all native I/O; the Svelte 5 frontend renders state and calls Tauri commands. Nothing in the webview reads files or opens network connections directly.

## Data flow

```
CC-CEDICT (cedict_1_0_ts_utf-8_mdbg.txt.gz, ~4 MB)
        │  streaming parse + numeric-tone → diacritic pinyin
        ▼
SQLite dictionary.db
  entries · senses · search_index (FTS5) · meta
        │  rusqlite
        ▼
Tauri commands  ──────────────►  Svelte 5 / TypeScript UI
        ▲
        └── jieba-rs segmentation (Milestone 1, next)
```

## Dictionary import

The importer parses CC-CEDICT line by line and writes the whole dictionary in a single SQLite transaction. If anything fails, the previous database is left untouched.

Rebuilds are gated on a SHA-256 fingerprint of the source file. Calling `dictionary_build` with the same CC-CEDICT release is a no-op; a changed fingerprint triggers a rebuild. The import also records the release metadata from the file's `#!` preamble.

The current CC-CEDICT release contains **125,324 entries / 199,696 senses** and produces a **~41 MB** database.

Malformed lines are counted and reported rather than aborting the import. The import only fails when no usable entry can be read.

### Source resolution

The core probes these locations in order and uses the first match:

1. an explicit `sourcePath` passed to `dictionary_build`,
2. `$QIAOREADER_CC_CEDICT`,
3. `<app data>/dictionary/<name>` (user override),
4. the bundled Tauri resource directory,
5. `src-tauri/resources/cc-cedict/<name>` during development.

Accepted names: `cedict_1_0_ts_utf-8_mdbg.txt.gz`, `cedict_1_0_ts_utf-8_mdbg.txt`, `cc-cedict.txt.gz`, `cc-cedict.txt`. Gzip is detected by magic bytes, not by extension.

## Database schema

| Table | Purpose |
| --- | --- |
| `entries` | One row per CC-CEDICT line: traditional and simplified headwords, raw numeric pinyin, tone-marked display pinyin, two tone-less search forms (`ni hao`, `nihao`), character count, source line |
| `senses` | One row per definition, ordered by position within its entry, so the UI never splits definition strings |
| `search_index` | External-content FTS5 index over `senses.definition` for English search (bm25, `detail=column`) |
| `meta` | Source SHA-256, path, size and mtime, entry/sense counts, build time, and CC-CEDICT version, date, publisher, and license |

Headword and pinyin lookups use B-tree indexes rather than FTS: `unicode61` treats a whole CJK headword as one token, while prefix `LIKE` on an indexed column is smaller and faster. The FTS index stores no copy of the text; it is rebuilt from `senses` after each import.

Schema changes bump `SCHEMA_VERSION` and ship an idempotent migration.

## Tauri command surface

| Command | Arguments | Returns |
| --- | --- | --- |
| `dictionary_status` | — | `DictionaryStatus` |
| `dictionary_source` | — | `DictionarySourceInfo` (path, size, every location probed) |
| `dictionary_build` | `sourcePath?`, `force?` | `DictionaryBuildReport` |
| `dictionary_lookup` | `word`, `limit?` | `DictEntry[]` (exact simplified or traditional match) |
| `dictionary_search` | `query`, `limit?` | `DictEntry[]` (Han characters, pinyin, or English) |

Builds emit `dictionary://import-progress` events with `phase`, `processed`, `total`, and `entries` fields. Errors are serialized as `{ code, message }`.

## Project layout

```
src/                          SvelteKit frontend
  routes/+page.svelte         Temporary backend diagnostic panel
src-tauri/
  src/
    db/                       SQLite: schema, migrations, read queries
    dictionary/               CC-CEDICT parser, pinyin, source lookup, importer
    commands/dictionary.rs    Tauri IPC surface
    state.rs                  Shared app state and app-data paths
  examples/build_dictionary.rs
  resources/cc-cedict/        Fetched dictionary data (gitignored) + attribution
.harness/                     PRD, build plan, technical constraints
scripts/                      CC-CEDICT fetch script, verification gate
.github/workflows/ci.yml      Web and Rust CI
```

## Localization

English is the source locale. User-facing strings are kept in catalogs (planned: `src/lib/i18n/messages/<locale>.json`) keyed by stable IDs; components never embed UI copy. Rust returns machine-readable error codes and the frontend owns the wording, so backend messages can change without touching translations.

The dictionary's English definitions are content, not UI copy, and are displayed verbatim with CC-CEDICT attribution.

Catalogs stay in-repo. Self-hosted Weblate is the intended manager once the reader UI stabilizes; it tracks per-locale completion and pushes updates back through Git. The catalog format is chosen in Milestone 2, constrained to a format Weblate supports (flat JSON or gettext PO).

## Licensing boundary

The application code is GPL-3.0-only. The CC-CEDICT data is a separate work licensed under CC BY-SA 4.0; its attribution and license URL are stored in `meta` and surfaced by `dictionary_status` so the UI can display them.
