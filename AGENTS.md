# QiaoReader AI Agent Workspace Protocol

## 🚀 Intent & Source of Truth
You are executing tasks for **QiaoReader**, a specialized JIT reading companion app for heritage Chinese speakers.
* **Requirements Base:** `.harness/prd.md`
* **Current Backlog:** `.harness/build_plan.md`
* **Technical Constraints:** `.harness/system_prompt.md`
* **System Design:** `ARCHITECTURE.md`
* **Public Status:** `STATUS.md`

Always read the files in `.harness/` before starting a milestone. Read `ARCHITECTURE.md` before changing the database layer, IPC surface, or import pipeline. Do not diverge from the specified tech stack (Tauri 2.0 + Svelte 5 + Bun + Rust + SQLite).

## 🛠️ Workspace Engineering Stack
* **Package Manager:** Bun (`bun install`, `bun add`, `bun tauri dev`) — never npm/yarn/pnpm.
* **Frontend:** Svelte 5 (Runes: `$state`, `$props`, `$derived`), TypeScript, TailwindCSS v4
* **Backend:** Rust, `jieba-rs` (Segmentation), `rusqlite` (Embedded DB)
* **UI:** TailwindCSS v4 + shadcn-svelte (Bits UI primitives, components vendored into `$lib/components/ui`)
* **Localization:** `svelte-i18n` with ICU catalogs (`src/lib/i18n/messages/*.json`), English source locale
* **Desktop Platform Targets:** Linux (Arch Linux development host)
* **Mobile Layout Directive:** Locked Landscape Mode Split-Screen UI

## ✅ Definition of Done
A task is complete only when all of the following are true **in the same session**:

1. `bun run verify` passes: rustfmt, clippy with `-D warnings`, `cargo test`, translation parity/ICU checks, `svelte-check --fail-on-warnings`, and a production frontend build.
2. New behaviour has a test, or the handoff explains concretely why it cannot be tested.
3. There are **zero new warnings** in Rust and in Svelte/TypeScript.
4. `.harness/build_plan.md` is updated (`- [x]`) and the session todo list matches it.
5. User-visible changes have a `CHANGELOG.md` entry under `## [Unreleased]` (except in the release commit, which moves the section into the version entry).
6. The handoff lists: files changed, commands run with results, what remains, and the next unchecked task.

Never tick a checkbox because code was written — tick it because verification passed.

## 🦀 Rust Discipline
* **Formatting:** rustfmt only (`cargo fmt --all`). Do not hand-align code.
* **Lints:** `cargo clippy --all-targets -- -D warnings` is the gate. An `#[allow(...)]` needs an inline comment explaining why it is unavoidable.
* **Errors:** fallible code returns `Result<T, AppError>`. No `unwrap`/`expect`/`panic!` in runtime paths — tests may use `expect("reason")`. Tauri commands serialise `{ code, message }`; every new variant gets a stable `code()`.
* **Concurrency:** commands that can exceed ~5 ms are `async` and offload CPU/IO to `spawn_blocking`. Never hold a `MutexGuard` across `.await` and never block the main thread.
* **SQL:** parameterised statements only — never interpolate user input into SQL. Schema changes bump `SCHEMA_VERSION` and ship an idempotent migration. The CC-CEDICT import stays a single transaction, and the external-content FTS index is rebuilt after senses change.
* **Tests:** `#[cfg(test)]` modules live next to the code. Parser tests must cover malformed lines and CRLF. DB tests use `Database::open_in_memory`. The full-corpus test stays `#[ignore]` and is run whenever the parser, schema, pinyin, or importer changes:
  ```sh
  cd src-tauri && cargo test --lib imports_the_real_cc_cedict_file -- --ignored --nocapture
  ```
* **Performance:** text processing (segmentation, pinyin, import) belongs in Rust. Lookups must stay on indexed columns; do not add full-table scans to hot paths.

## 🧡 Svelte / TypeScript Discipline
* **Svelte 5 runes only** for new state (`$state`, `$derived`, `$props`, `$effect`). Use `onMount` for Tauri listeners (SSR is disabled). Do not add legacy `export let` / `$:` to new code.
* **TypeScript:** `strict`; no `any` (use `unknown` and narrow). IPC payload types must match Rust's `#[serde(rename_all = "camelCase")]` exactly.
* **Native boundary:** everything native goes through `invoke` from `@tauri-apps/api/core`. Never add web `fetch` for Anki or filesystem access.
* **Quality:** `svelte-check --fail-on-warnings` must be clean. No leftover `console.log`, dead CSS selectors, or unused imports.
* **UI verification:** run `bun tauri dev` for any UI change and inspect the rendered result (screenshot for visual changes). On this Sway host use `grim -g "<x>,<y> <w>x<h>"`, since `xdotool` cannot drive native Wayland windows; trigger state through the UI or HMR instead.
* **UI components:** use the shadcn-svelte components in `$lib/components/ui` and add new ones with `bunx shadcn-svelte@latest add <name>`. Do not hand-roll dialogs, menus, popovers, or select controls that Bits UI already provides. Components are owned source: adjust them deliberately, not with one-off overrides in pages.
* **Theming:** use semantic tokens (`bg-background`, `text-muted-foreground`, `border-border`); no hardcoded hex colours in components. Light/dark follows the OS through the `.dark` class.
* **Reveal semantics:** hover and focus may link a word and its card, but only click/tap reveals pinyin or definitions. Hiding the selected card clears its highlight. Masked content is blurred, non-selectable, and `aria-hidden`; expose state with `aria-expanded`/`aria-pressed`.
* **Layout:** at 640 px and wider the workspace is a strict 75 % reader / 25 % reference panel; below that the reference dock is a Sheet. Tap-to-reveal cards must remain keyboard-accessible, and touch targets must be at least 44 px with no hover-only affordances.

## 🚨 Warnings, Errors, and Silent Failures
* **Zero-warning policy** in both ecosystems. A warning is debt: fix it, or suppress it with a written reason.
* No swallowed `Result` or empty `catch`: either handle it or surface it through the `{ code, message }` contract.
* Parser/import failures must be counted and reported (skipped lines, warnings) — never silently dropped.
* `unwrap_or` is acceptable only when a documented, sensible default exists.

## 🌿 Git & Commits
* **Branches:** `main` is the integration branch. Work on short-lived `feat/<slug>`, `fix/<slug>`, or `chore/<slug>` branches from `main`; push every branch (including WIP) to `origin` and use draft PRs for remote backup and coordination. Do not keep a long-lived `feat` integration branch; if a shared release-train branch is ever needed, use `develop`/`next` and protect it. Squash-merge verified work; merge when the task is done, not when bumping a version.
* **Main protection:** `main` requires a PR, both CI jobs, an up-to-date branch, linear history, and resolved conversations; force pushes and deletions are blocked for admins too. `v*` tags are immutable. Release commits go through a `chore/release-vX.Y.Z` PR; never push directly to `main`.
* **Conventional Commits:** `type(scope): summary` (≤ 72 chars). Types: `feat`, `fix`, `chore`, `docs`, `test`, `refactor`, `perf`, `build`. Scopes: `dict`, `db`, `ui`, `anki`, `mobile`, `repo`, `harness`.
* **One logical change per commit.** The body explains *why*, notes schema/IPC changes, and records verification, e.g. `Verified: bun run verify`.
* **Versioning:** Semantic Versioning 2.0.0. Before 1.0, bump MINOR for a feature or milestone and PATCH for fixes; breaking changes may occur in a MINOR and must be called out. Keep `package.json`, `src-tauri/tauri.conf.json`, and `src-tauri/Cargo.toml` in sync (`bun run version:check`); annotated tags are `vX.Y.Z` from `main`.
* **Releases:** After the release PR merges, a pushed `vX.Y.Z` tag runs `.github/workflows/release.yml`, which validates the tag/version and builds desktop bundles (and Android when the Android project and signing are configured). Releases are tag-driven, never per-merge; see `RELEASING.md`.
* **Changelog:** Add every user-visible change under `## [Unreleased]` in `CHANGELOG.md`; `bun run version:bump` moves it into the release section.
* **Never commit:** `*.db`, `*.db-wal`, `*.db-shm`, CC-CEDICT source data, `node_modules/`, `build/`, `.svelte-kit/`, `src-tauri/target/`, `.env*`, credentials or keys.
* **Never** use `--no-verify`. Do not force-push `main` or rewrite pushed history unless the user explicitly asks for a rewrite (for example, squashing the initial history); use `--force-with-lease` when they do.
* **Agents** commit and push only when asked, stage only files related to the task, and never push to a remote other than `origin`.
* After pushing, do not poll CI unless a failure blocks the task. Report the run URL and move on; inspect results only when the user asks or a failure blocks the next step.

## 🌐 Localization Discipline
English is the source locale; the UI must be translatable without editing components.

* Every user-facing string lives in a message catalog under `src/lib/i18n/`, keyed by a stable semantic ID (`dict.build.action`). No hardcoded UI copy in new Svelte code.
* Use ICU MessageFormat for interpolation and plurals; never assemble sentences from fragments.
* Rust returns stable machine-readable error `code`s. The frontend maps codes to localized text; the Rust `message` field is diagnostic and must not be rendered as UI copy.
* Format dates, numbers, and relative times with `Intl`, not by hand.
* Every source key must exist in every catalog. `bun run i18n:check` (part of `verify`) fails on missing or extra keys, empty messages, mismatched placeholders, or invalid ICU syntax. Use `bun run i18n:status` for a coverage report.
* Detect the OS locale, allow a persisted user override, and default to English.
* Dictionary content is CC-CEDICT English and is displayed verbatim; it is not UI copy.
* Adding a locale means adding its catalog, updating the key check, and inspecting the layout at that locale.

Translation management: catalogs stay in-repo and reviewable. Hosted [Weblate](https://weblate.org) is the intended manager; it tracks per-language completion and pushes translations back through Git. See `TRANSLATING.md` for the catalog format, locale workflow, and Weblate component settings.

## 📚 Documentation Discipline
* Root Markdown is the documentation until a docs site exists: `README.md` (what it is and how to run it), `ARCHITECTURE.md` (design), `STATUS.md` (milestones), `TRANSLATING.md` (localization), `RELEASING.md` (branches, versions, releases), `CHANGELOG.md` (release history), `AGENTS.md` (process).
* A fact belongs in exactly one of them; link to it instead of duplicating it.
* When user-facing documentation is needed, introduce `docs/` with MkDocs Material and add `mkdocs build --strict` to `bun run verify`. Content moves out of the README rather than being copied.

## 📋 Task & Todo Tracking
* Mirror `.harness/build_plan.md` into the session todo list before starting multi-step work. Keep **exactly one** milestone task `in_progress`.
* Mark a todo `completed` only after its gate has passed; while code exists but is unverified, leave it `in_progress`.
* Update the markdown checkbox in the same turn as the verified handoff. Add a one-line note for non-obvious work (command + key metric).
* If a milestone's state changes, update `STATUS.md` in the same change as `.harness/build_plan.md`. `STATUS.md` is the public summary; the build plan remains authoritative.
* If blocked, leave the task unchecked, state the concrete blocker, and propose the next command or decision needed.

## 📋 Iteration & Execution Rules

1. **One Task at a Time:** Focus on the active, uncompleted task listed in `.harness/build_plan.md`. Never try to code multiple milestones in a single turn.
2. **Context Preservation:** When editing or extending code blocks (especially Rust `lib.rs` or Svelte routes), provide full or clean contextual diffs. Do not truncate essential logic with `// ... rest of code` unless explicitly authorized.
3. **CORS Enforcement:** All external communication to the local Anki instance (`localhost:8765`) MUST pass through Tauri's native Rust execution thread command invocation layer. Never inject web-native fetch fallbacks that will trigger browser cross-origin policy locks.
4. **Milestone Tracking:** Update the markdown checklist (`- [x]`) in `.harness/build_plan.md` in the same change that proves the task complete.

## 🏁 Current Operational Directives
Begin execution on **Milestone 2: Responsive Split-Screen UI & Anki Connect**. The current state is summarised in `STATUS.md`; the next task is the first unchecked item in `.harness/build_plan.md`.
