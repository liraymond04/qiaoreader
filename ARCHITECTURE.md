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
        └── jieba-rs segmentation + CC-CEDICT lookups (process_raw_text)
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

## Segmentation

`process_raw_text` runs jieba-rs in default mode with HMM enabled. It emits the input as ordered segments, preserving the whitespace and punctuation that the segmenter skips, and annotates Han tokens with their CC-CEDICT readings. Unique words are returned once in first-appearance order and segments reference them by index, which keeps repeated words out of the IPC payload and gives the reference sidebar its chronological vocabulary list.

The segmenter's dictionary is embedded in the binary and loaded lazily on the first request.

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
| `process_raw_text` | `text` | `ProcessedText` (ordered segments + unique dictionary words) |

Builds emit `dictionary://import-progress` events with `phase`, `processed`, `total`, and `entries` fields. Errors are serialized as `{ code, message }`.

## Project layout

```
src/                          SvelteKit frontend
  routes/+page.svelte         Temporary backend diagnostic panel
src-tauri/
  src/
    db/                       SQLite: schema, migrations, read queries
    dictionary/               CC-CEDICT parser, pinyin, source lookup, importer
    commands/                 Tauri IPC surface
    text.rs                   jieba-rs segmentation + dictionary annotation
    state.rs                  Shared app state and app-data paths
  examples/build_dictionary.rs
  resources/cc-cedict/        Fetched dictionary data (gitignored) + attribution
.harness/                     PRD, build plan, technical constraints
scripts/                      CC-CEDICT fetch script, verification gate
.github/workflows/ci.yml      Web and Rust CI
```

## Localization

English is the source locale; Simplified Chinese (`zh-Hans`) ships as the first translation. Catalogs live in `src/lib/i18n/messages/<locale>.json` as flat maps of stable message IDs to ICU MessageFormat strings and are loaded by `svelte-i18n`. `src/lib/i18n/index.ts` detects the OS language, honours a persisted override in `localStorage`, and falls back to English. Components never embed UI copy.

Rust returns machine-readable error codes and the frontend owns the wording, so backend messages can change without touching translations. The dictionary's English definitions are content, not UI copy, and are displayed verbatim with CC-CEDICT attribution.

`bun run i18n:check` validates key parity, placeholders, and ICU syntax for every catalog, and runs as part of `bun run verify`. Translation management is designed around Weblate's JSON format; see `TRANSLATING.md`. Catalogs stay in-repo so coverage is reviewable in pull requests.

## Licensing boundary

The application code is GPL-3.0-only. The CC-CEDICT data is a separate work licensed under CC BY-SA 4.0; its attribution and license URL are stored in `meta` and surfaced by `dictionary_status` so the UI can display them.
