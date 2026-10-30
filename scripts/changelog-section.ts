#!/usr/bin/env bun
/**
 * Print the CHANGELOG section for one release, without the heading.
 *
 *   bun run changelog:section --version 0.1.0
 *   bun run changelog:section --tag v0.1.0
 */
import { readFileSync } from "node:fs";
import { join } from "node:path";

const args = process.argv.slice(2);
const valueIndex = args.findIndex(
  (arg) => arg === "--version" || arg === "--tag",
);
const raw = valueIndex >= 0 ? args[valueIndex + 1] : undefined;
const version = raw?.replace(/^v/, "");

if (!version) {
  console.error("usage: bun run changelog:section --version <x.y.z>");
  process.exit(1);
}

const changelog = readFileSync(join(import.meta.dir, "..", "CHANGELOG.md"), "utf8");
const marker = `## [${version}]`;
const start = changelog.indexOf(marker);
if (start === -1) {
  console.error(`CHANGELOG.md has no section for version ${version}`);
  process.exit(1);
}

const next = changelog.indexOf("\n## [", start + marker.length);
const section = changelog
  .slice(start, next === -1 ? undefined : next)
  .trim()
  .replace(/^## \[[^\]]+\][^\n]*\n?/, "")
  .trim();

if (!section) {
  console.error(`CHANGELOG.md section ${version} is empty`);
  process.exit(1);
}

console.log(section);
