#!/usr/bin/env bash
# QiaoReader verification gate — the single command every task must pass.
#
#   bun run verify
#
# Runs, in order: rustfmt check, clippy with warnings denied, Rust tests,
# svelte-check with warnings denied, and a production frontend build.
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."
repo_root="$(pwd)"

step() {
  printf '\n\033[1;36m==> %s\033[0m\n' "$1"
}

step "Rust: rustfmt"
(cd src-tauri && cargo fmt --all --check)

step "Rust: clippy (warnings are errors)"
(cd src-tauri && cargo clippy --all-targets -- -D warnings)

step "Rust: unit tests"
(cd src-tauri && cargo test)

step "Release metadata: version sync"
bun run version:check

step "Translations: catalog parity and ICU"
bun run i18n:check

step "Svelte/TypeScript: svelte-check (warnings are errors)"
bun run check

step "SvelteKit: production build"
bun run build

printf '\n\033[1;32mAll checks passed\033[0m (%s)\n' "$repo_root"
