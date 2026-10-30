# CC-CEDICT source data

QiaoReader builds its embedded SQLite dictionary from **CC-CEDICT**, the
community-maintained Chinese–English dictionary published by MDBG.

- Project home: <https://www.mdbg.net/chinese/dictionary?page=cc-cedict>
- Downloads: <https://www.mdbg.net/chinese/export/cedict/cedict_1_0_ts_utf-8_mdbg.txt.gz>

## Licence and attribution

CC-CEDICT is licensed under the
[Creative Commons Attribution-ShareAlike 4.0 International License](https://creativecommons.org/licenses/by-sa/4.0/).

> CC-CEDICT © MDBG and contributors, licensed under CC BY-SA 4.0.

The licence URL, publisher, and release date are parsed from the file's `#!`
preamble and stored in the database's `meta` table. They are exposed to the UI
through `dictionary_status` (`cedictLicense`, `cedictPublisher`, `cedictDate`),
so any surface that presents dictionary content can show the attribution
without hard-coding it.

## Fetching the file

The data file is **not** committed to git (about 4 MB gzipped). Download it with:

```sh
bun run dict:fetch
```

This writes `cedict_1_0_ts_utf-8_mdbg.txt.gz` here. Pass `--force` to refresh an
existing copy:

```sh
bun scripts/fetch-cc-cedict.ts --force
```

## Where the Rust core looks for it

First hit wins (see `src-tauri/src/dictionary/source.rs`):

1. an explicit path passed to `dictionary_build({ sourcePath })`,
2. `$QIAOREADER_CC_CEDICT`,
3. `<app data>/dictionary/<name>` (user-supplied override),
4. the bundled Tauri resource directory,
5. this directory (`src-tauri/resources/cc-cedict/`) during development.

Accepted file names: `cedict_1_0_ts_utf-8_mdbg.txt.gz`,
`cedict_1_0_ts_utf-8_mdbg.txt`, `cc-cedict.txt.gz`, `cc-cedict.txt`.
Gzip is detected by magic bytes, so the extension is only a hint.

## Updating

The importer hashes the source file (SHA-256) and stores the hash in `meta`.
`dictionary_build` without `force` becomes a no-op when the hash is unchanged,
so shipping a newer CC-CEDICT release rebuilds the dictionary automatically on
the next launch.
