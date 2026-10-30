# QiaoReader Engineering Build Plan

- [ ] Milestone 1: Rust Core Setup & Dictionary Integration
  - [x] Initialize workspace configurations
  - [x] Write raw CC-CEDICT file parser to build internal embedded SQLite database
  - [ ] Implement `process_raw_text` command integrating `jieba-rs` tokenizer
- [ ] Milestone 2: Responsive Split-Screen UI & Anki Connect
  - [ ] Create TailwindCSS responsive layout using Svelte components (Left/Right dock toggle rules)
  - [ ] Implement TypeScript API client for local AnkiConnect bindings
- [ ] Milestone 3: Interactive JIT Reading Loop
  - [ ] Program Pre-Check interstitial workspace management state in Svelte
  - [ ] Construct Svelte-native masked tap-to-reveal components using local state ($state)
- [ ] Milestone 4: Mobile Platform Orientation & Compilation
  - [ ] Integrate Tauri native landscape-locking window configurations
  - [ ] Code swipe pagination gesture touch mechanics

- [ ] Milestone 5: Documentation Site & Publishing
  - [ ] Scaffold MkDocs Material under `docs/` and move ARCHITECTURE, STATUS, and TRANSLATING into it
  - [ ] Add `mkdocs build --strict` to `bun run verify` and CI
  - [ ] Publish the documentation site to GitHub Pages

## Cross-cutting

- [x] Localization pipeline: English source, Simplified Chinese catalog, locale switching, key/ICU parity gate
- [ ] Hosted Weblate project and GitHub translation sync
