# QiaoReader

QiaoReader is a just-in-time reading companion for heritage Chinese speakers. It segments Chinese text into words as you read and shows each word's pinyin and English definitions in a side panel, so you can look things up without leaving the page.

Built with Tauri 2 (Rust core), Svelte 5, Bun, Tailwind CSS v4, shadcn-svelte, and an embedded SQLite dictionary generated from CC-CEDICT.

## Requirements

- [Bun](https://bun.sh) 1.2 or newer
- Rust 1.85 or newer (edition 2024) with `rustfmt` and `clippy`
- Tauri 2 Linux dependencies (Arch Linux):

  ```bash
  sudo pacman -S --needed base-devel webkit2gtk-4.1 gtk3 libappindicator-gtk3 \
    librsvg openssl curl wget file
  ```

  On Debian/Ubuntu, see Tauri's [Linux prerequisites](https://v2.tauri.app/start/prerequisites/#linux).
- Anki with [AnkiConnect](https://ankiweb.net/shared/info/2055492159) for the Anki integration (not required yet)

## Getting started

```bash
git clone git@github.com:liraymond04/qiaoreader.git
cd qiaoreader
bun install
bun run dict:fetch
bun tauri dev
```

`bun run dict:fetch` downloads the current CC-CEDICT release (4 MB gzipped) into `src-tauri/resources/cc-cedict/`. The data file is intentionally not committed; pass `--force` to refresh it.

On first launch the dictionary panel reports **not built**. Press **Build dictionary**; the import takes a few seconds and writes the database to `~/.local/share/com.liraymond04.qiaoreader/dictionary.db` on Linux.

To build a production bundle:

```bash
bun tauri build
```

## Translations

UI strings live in `src/lib/i18n/messages/`. See [TRANSLATING.md](TRANSLATING.md) for the catalog format, adding a locale, and the Weblate workflow.

## Releases

Versioned releases are tagged `vX.Y.Z`; see [CHANGELOG.md](CHANGELOG.md) for changes and [RELEASING.md](RELEASING.md) for the release process.

## License

QiaoReader is licensed under the [GNU General Public License v3.0](LICENSE) (`GPL-3.0-only`).

The bundled CC-CEDICT dictionary data is © MDBG and contributors and is licensed separately under [CC BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/).

## Credits

- [CC-CEDICT](https://www.mdbg.net/chinese/dictionary?page=cc-cedict)
- [Tauri](https://v2.tauri.app), [Svelte](https://svelte.dev), [Tailwind CSS](https://tailwindcss.com), [shadcn-svelte](https://shadcn-svelte.com), [rusqlite](https://github.com/rusqlite/rusqlite), and [Bun](https://bun.sh)
