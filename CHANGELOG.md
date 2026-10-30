# Changelog

All notable changes to QiaoReader are documented here.
The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2026-10-30

### Added

- Embedded CC-CEDICT SQLite dictionary with a streaming importer, pinyin
  conversion, exact lookup, and multi-strategy search.
- Tauri commands for dictionary status, source discovery, build progress,
  lookup, search, and `process_raw_text` segmentation with `jieba-rs`.
- English and Simplified Chinese localisation with a key, placeholder, and ICU
  parity gate in `bun run verify`.
- Responsive split-screen reader shell with a 75/25 reference dock and a
  compact Sheet, built with Tailwind CSS v4 and shadcn-svelte.
- Hidden, Pinyin, and Full reference modes with per-card reveal, hover linking,
  ruby pinyin, optional tone colours, and a full-entry details dialog.
- Linux Wayland rendering workaround so scrolling follows the display refresh
  rate instead of GTK 3's 60 Hz frame clock.
- GitHub Actions CI for the web and Rust checks, with a weekly full-corpus
  CC-CEDICT import.

### Changed

- Reference cards use larger characters, tighter ruby spacing, and an inline
  details button, leaving more width for definitions.
- README, ARCHITECTURE, STATUS, TRANSLATING, and AGENTS documents establish the
  project's engineering and localisation discipline.
