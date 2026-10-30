# Status

QiaoReader is in early development. This file is the public summary; [`.harness/build_plan.md`](.harness/build_plan.md) is the authoritative backlog and checklist.

## Milestones

| Milestone | State |
| --- | --- |
| 1. Rust core & CC-CEDICT integration | Database, parser, and lookup/search commands done; `process_raw_text` segmentation next |
| 2. Split-screen UI & AnkiConnect | Not started |
| 3. Interactive JIT reading loop | Not started |
| 4. Mobile orientation & gestures | Not started |

## Roadmap

- [x] Embedded SQLite schema and CC-CEDICT importer
- [x] `dictionary_status` / `dictionary_source` / `dictionary_build` / `dictionary_lookup` / `dictionary_search`
- [ ] `process_raw_text` with `jieba-rs` segmentation
- [ ] Split-screen Svelte layout and AnkiConnect client
- [ ] Tap-to-reveal reading loop with pre-check interstitial
- [ ] Mobile landscape locking and swipe pagination

## Verification

CI runs the web and Rust checks on every push and pull request, and imports the real 125k-entry CC-CEDICT corpus on pushes to `main`, manual runs, and a weekly schedule. See [`.github/workflows/ci.yml`](.github/workflows/ci.yml) and the verification gates in [AGENTS.md](AGENTS.md).
