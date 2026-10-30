#!/usr/bin/env bun
/**
 * Keep the three version fields in sync and move CHANGELOG entries.
 *
 *   bun run version:check            # verify package.json / tauri.conf.json / Cargo.toml agree
 *   bun run version:check --tag v0.2.0
 *   bun run version:bump 0.2.0       # update versions and cut an Unreleased section
 */
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

const root = join(import.meta.dir, "..");
const packagePath = join(root, "package.json");
const tauriPath = join(root, "src-tauri", "tauri.conf.json");
const cargoPath = join(root, "src-tauri", "Cargo.toml");
const changelogPath = join(root, "CHANGELOG.md");

const args = process.argv.slice(2);
const check = args.includes("--check");
const tagIndex = args.indexOf("--tag");
const expectedTag = tagIndex >= 0 ? args[tagIndex + 1] : undefined;
const positional = args.filter(
  (arg, index) =>
    !arg.startsWith("--") && (tagIndex < 0 || index !== tagIndex + 1),
);
const requestedVersion = positional[0];

function fail(message: string): never {
  console.error(`error: ${message}`);
  process.exit(1);
}

const packageJson = JSON.parse(readFileSync(packagePath, "utf8")) as {
  version: string;
};
const tauriJson = JSON.parse(readFileSync(tauriPath, "utf8")) as {
  version: string;
};
const cargoToml = readFileSync(cargoPath, "utf8");
const cargoVersion = cargoToml.match(
  /^\[package\][\s\S]*?^version\s*=\s*"([^"]+)"/m,
)?.[1];

if (!cargoVersion) {
  fail("could not read the package version from src-tauri/Cargo.toml");
}

const versions = {
  "package.json": packageJson.version,
  "src-tauri/tauri.conf.json": tauriJson.version,
  "src-tauri/Cargo.toml": cargoVersion,
};

if (check) {
  const unique = new Set(Object.values(versions));
  if (unique.size !== 1) {
    console.error("Version mismatch:");
    for (const [file, version] of Object.entries(versions)) {
      console.error(`  ${file}: ${version}`);
    }
    process.exit(1);
  }

  const version = [...unique][0];
  if (expectedTag && expectedTag !== `v${version}`) {
    fail(`tag ${expectedTag} does not match version v${version}`);
  }

  console.log(`Versions in sync: ${version}`);
  process.exit(0);
}

if (!requestedVersion) {
  fail("usage: bun run version:bump <major.minor.patch>");
}

if (!/^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$/.test(requestedVersion)) {
  fail(`"${requestedVersion}" is not a valid SemVer version`);
}

packageJson.version = requestedVersion;
tauriJson.version = requestedVersion;
writeFileSync(packagePath, `${JSON.stringify(packageJson, null, 2)}\n`);
writeFileSync(tauriPath, `${JSON.stringify(tauriJson, null, 2)}\n`);

const updatedCargo = cargoToml.replace(
  /(^\[package\][\s\S]*?^version\s*=\s*")[^"]+(")/m,
  `$1${requestedVersion}$2`,
);
if (updatedCargo === cargoToml) {
  fail("could not update the version in src-tauri/Cargo.toml");
}
writeFileSync(cargoPath, updatedCargo);

const date = new Date().toISOString().slice(0, 10);
const changelog = readFileSync(changelogPath, "utf8");
const unreleasedStart = changelog.indexOf("## [Unreleased]");
if (unreleasedStart === -1) {
  fail("CHANGELOG.md is missing a `## [Unreleased]` section");
}

const nextHeading = changelog.indexOf("\n## [", unreleasedStart + 1);
const unreleasedBlock =
  nextHeading === -1
    ? changelog.slice(unreleasedStart)
    : changelog.slice(unreleasedStart, nextHeading);
const body = unreleasedBlock
  .replace(/^## \[Unreleased\]\s*/, "")
  .trim();

const release =
  `## [${requestedVersion}] - ${date}\n\n` + (body ? `${body}\n\n` : "");
const updatedChangelog =
  changelog.slice(0, unreleasedStart) +
  `## [Unreleased]\n\n${release}` +
  (nextHeading === -1 ? "" : changelog.slice(nextHeading + 1));
writeFileSync(changelogPath, updatedChangelog);

console.log(`Updated versions to ${requestedVersion}.`);
console.log("Next:");
console.log(`  git switch -c chore/release-v${requestedVersion}`);
console.log("  cargo check   # refresh src-tauri/Cargo.lock");
console.log("  git add package.json src-tauri/tauri.conf.json src-tauri/Cargo.toml src-tauri/Cargo.lock CHANGELOG.md");
console.log(`  git commit -m "chore(release): v${requestedVersion}"`);
console.log(`  git push -u origin chore/release-v${requestedVersion}`);
console.log("  # open a PR, wait for CI, squash-merge it, then:");
console.log("  git switch main && git pull --ff-only");
console.log(`  git tag -a v${requestedVersion} -m "QiaoReader v${requestedVersion}"`);
console.log(`  git push origin v${requestedVersion}`);
