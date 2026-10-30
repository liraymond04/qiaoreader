#!/usr/bin/env bun
/**
 * Translation coverage and consistency check.
 *
 *   bun run i18n:status   report coverage
 *   bun run i18n:check    exit non-zero on any problem (used by `bun run verify`)
 *
 * Catalogs are flat JSON: message ID -> ICU MessageFormat string. English
 * (`en.json`) is the source of truth.
 */
import { existsSync, readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";

import { IntlMessageFormat } from "intl-messageformat";

import { DEFAULT_LOCALE, SUPPORTED_LOCALES } from "../src/lib/i18n/locales";

const messagesDir = join(import.meta.dir, "..", "src", "lib", "i18n", "messages");
const checkOnly = process.argv.includes("--check");
const problems: string[] = [];

type Catalog = Record<string, string>;

function loadCatalog(code: string): Catalog {
  const file = join(messagesDir, `${code}.json`);
  if (!existsSync(file)) {
    problems.push(`${code}: missing catalog ${file}`);
    return {};
  }

  let parsed: unknown;
  try {
    parsed = JSON.parse(readFileSync(file, "utf8"));
  } catch (error) {
    problems.push(`${code}: invalid JSON: ${(error as Error).message}`);
    return {};
  }

  if (typeof parsed !== "object" || parsed === null || Array.isArray(parsed)) {
    problems.push(`${code}: catalog must be a JSON object`);
    return {};
  }

  const catalog: Catalog = {};
  for (const [key, value] of Object.entries(parsed as Record<string, unknown>)) {
    if (typeof value !== "string") {
      problems.push(`${code}: "${key}" must be a string`);
      continue;
    }
    if (value.trim() === "") {
      problems.push(`${code}: "${key}" is empty`);
      continue;
    }
    catalog[key] = value;
  }
  return catalog;
}

/** Argument names referenced by an ICU message, including plural arguments. */
function placeholders(message: string): Set<string> {
  const found = new Set<string>();
  for (const match of message.matchAll(/\{\s*([a-zA-Z_][\w.]*)/g)) {
    found.add(match[1]);
  }
  return found;
}

function validateIcu(code: string, catalog: Catalog): void {
  for (const [key, message] of Object.entries(catalog)) {
    try {
      // Parsing is enough; formatting would require sample values.
      new IntlMessageFormat(message, code);
    } catch (error) {
      problems.push(`${code}: "${key}" is not valid ICU: ${(error as Error).message}`);
    }
  }
}

const supportedCodes = new Set(SUPPORTED_LOCALES.map((item) => item.code));
const files = readdirSync(messagesDir)
  .filter((file) => file.endsWith(".json"))
  .map((file) => file.replace(/\.json$/, ""))
  .sort();

for (const file of files) {
  if (!supportedCodes.has(file)) {
    problems.push(`${file}: catalog exists but is not listed in src/lib/i18n/locales.ts`);
  }
}

const source = loadCatalog(DEFAULT_LOCALE);
const sourceKeys = Object.keys(source).sort();
validateIcu(DEFAULT_LOCALE, source);

const rows: Array<{
  locale: string;
  translated: number;
  total: number;
  missing: string[];
  extra: string[];
  coverage: number;
}> = [];

for (const { code } of SUPPORTED_LOCALES) {
  const catalog = code === DEFAULT_LOCALE ? source : loadCatalog(code);
  if (code !== DEFAULT_LOCALE) {
    validateIcu(code, catalog);
  }

  const keys = new Set(Object.keys(catalog));
  const missing = sourceKeys.filter((key) => !keys.has(key));
  const extra = Object.keys(catalog)
    .filter((key) => !sourceKeys.includes(key))
    .sort();

  let translated = 0;
  for (const key of sourceKeys) {
    if (!(key in catalog)) continue;
    translated += 1;

    if (code === DEFAULT_LOCALE) continue;

    const expected = [...placeholders(source[key])].sort();
    const actual = [...placeholders(catalog[key])].sort();
    if (expected.join(",") !== actual.join(",")) {
      problems.push(
        `${code}: "${key}" placeholders differ (source: {${expected.join(", ")}}; translation: {${actual.join(", ")}})`,
      );
    }
  }

  rows.push({
    locale: code,
    translated,
    total: sourceKeys.length,
    missing,
    extra,
    coverage: sourceKeys.length === 0 ? 100 : (translated / sourceKeys.length) * 100,
  });
}

const nameWidth = Math.max("Locale".length, ...rows.map((row) => row.locale.length));
console.log(
  `${"Locale".padEnd(nameWidth)}  Translated  Missing  Extra  Coverage`,
);
for (const row of rows) {
  console.log(
    `${row.locale.padEnd(nameWidth)}  ${`${row.translated}/${row.total}`.padStart(10)}  ${String(row.missing.length).padStart(7)}  ${String(row.extra.length).padStart(5)}  ${`${row.coverage.toFixed(1)}%`.padStart(8)}`,
  );
  if (row.missing.length > 0) {
    console.log(`  missing: ${row.missing.slice(0, 8).join(", ")}${row.missing.length > 8 ? ", …" : ""}`);
  }
  if (row.extra.length > 0) {
    console.log(`  extra:   ${row.extra.slice(0, 8).join(", ")}${row.extra.length > 8 ? ", …" : ""}`);
  }
}

if (problems.length > 0) {
  console.error(`\n${problems.length} translation problem(s):`);
  for (const problem of problems) {
    console.error(`- ${problem}`);
  }
  if (checkOnly) {
    process.exitCode = 1;
  }
} else {
  console.log("\nAll translation catalogs are consistent.");
}
