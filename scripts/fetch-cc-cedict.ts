#!/usr/bin/env bun
/**
 * Download the current CC-CEDICT release into src-tauri/resources/cc-cedict/.
 *
 *   bun run dict:fetch           # skip if a copy already exists
 *   bun run dict:fetch --force   # always re-download
 *
 * The file is ~4 MB gzipped (~10 MB of UTF-8 text) and is intentionally not
 * committed. CC-CEDICT is CC BY-SA 4.0 — see the README next to it.
 */
import { existsSync, mkdirSync, statSync } from "node:fs";
import { dirname, join } from "node:path";

const SOURCE_URL =
  "https://www.mdbg.net/chinese/export/cedict/cedict_1_0_ts_utf-8_mdbg.txt.gz";

const repositoryRoot = join(import.meta.dir, "..");
const target = join(
  repositoryRoot,
  "src-tauri",
  "resources",
  "cc-cedict",
  "cedict_1_0_ts_utf-8_mdbg.txt.gz",
);

const force = process.argv.includes("--force");

if (existsSync(target) && !force) {
  const size = (statSync(target).size / (1024 * 1024)).toFixed(1);
  console.log(`CC-CEDICT is already present (${size} MB): ${target}`);
  console.log("Pass --force to re-download.");
  process.exit(0);
}

console.log(`Downloading ${SOURCE_URL} ...`);
const response = await fetch(SOURCE_URL);
if (!response.ok) {
  throw new Error(`Download failed: HTTP ${response.status} ${response.statusText}`);
}

const bytes = new Uint8Array(await response.arrayBuffer());
if (bytes.byteLength < 1_000_000) {
  throw new Error(`Refusing to write a suspiciously small file (${bytes.byteLength} bytes)`);
}
// gzip magic number: a plain-text error page must never masquerade as the data.
if (bytes[0] !== 0x1f || bytes[1] !== 0x8b) {
  throw new Error("Downloaded payload is not gzip data (did the upstream URL change?)");
}

mkdirSync(dirname(target), { recursive: true });
await Bun.write(target, bytes);

console.log(
  `Wrote ${(bytes.byteLength / (1024 * 1024)).toFixed(1)} MB to ${target}`,
);
console.log("Next: build the dictionary from the app, or run `cargo run --example build_dictionary`.");
