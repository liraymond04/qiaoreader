# Status

QiaoReader is in early development. This file is the public summary; [`.harness/build_plan.md`](.harness/build_plan.md) is the authoritative backlog and checklist.

## Milestones

| Milestone | State |
| --- | --- |
| 1. Rust core & CC-CEDICT integration | Done |
| 2. Split-screen UI & AnkiConnect | Split-screen shell done; AnkiConnect client next |
| 3. Interactive JIT reading loop | Reference reveal/modes/ruby/details done; pre-check next |
| 4. Mobile orientation & gestures | Not started |
| 5. Documentation site & publishing | Not started |

## Roadmap

- [x] Embedded SQLite schema and CC-CEDICT importer
- [x] `dictionary_status` / `dictionary_source` / `dictionary_build` / `dictionary_lookup` / `dictionary_search`
- [x] `process_raw_text` with `jieba-rs` segmentation
- [x] Split-screen Svelte layout with a 75/25 reference dock
- [ ] AnkiConnect TypeScript client
- [x] Reference panel reveal modes, ruby pinyin, tone colors, and details view
- [ ] Pre-check interstitial and reader-level reveal loop
- [ ] Mobile landscape locking and swipe pagination
- [ ] Documentation site (MkDocs Material) on GitHub Pages

## Foundations

- [x] Localization catalogs (English + Simplified Chinese), locale switching, key/ICU parity gate
- [ ] Hosted Weblate project and GitHub translation sync

## Verification

CI runs the web and Rust checks (including translation parity) on every push and pull request, and imports the real 125k-entry CC-CEDICT corpus on pushes to `main`, manual runs, and a weekly schedule. See [`.github/workflows/ci.yml`](.github/workflows/ci.yml) and the verification gates in [AGENTS.md](AGENTS.md).
