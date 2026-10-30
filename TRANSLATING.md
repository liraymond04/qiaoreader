# Translating QiaoReader

UI strings live in `src/lib/i18n/messages/<locale>.json`. English (`en.json`) is the source of truth; other catalogs must contain the same keys.

Dictionary content is not translated. CC-CEDICT definitions are English content licensed under CC BY-SA 4.0 and are displayed verbatim.

## Message format

Catalogs are flat JSON maps of stable message IDs to [ICU MessageFormat](https://unicode-org.github.io/icu/userguide/format_parse/messages/) strings:

```json
{
  "dict.title": "Dictionary",
  "dict.report.imported": "Imported {entries} entries / {senses} senses in {ms} ms.",
  "dict.warnings": "{count, plural, one {# warning} other {# warnings}}"
}
```

Rules:

- Keys are semantic and stable; never use the English text as the key.
- Use `{name}` for interpolation and `{count, plural, ...}` for plurals.
- Never build a sentence by concatenating fragments.
- Keep every placeholder identical between languages, and translate the text around it, not the placeholder.
- Do not add HTML. Render structure in Svelte and pass values in.

## Adding or updating a locale

1. Copy `src/lib/i18n/messages/en.json` to `<locale>.json`, using a BCP 47 code (`zh-Hant`, `ja`, `fr`).
2. Translate the values.
3. Add `{ code, label }` to `SUPPORTED_LOCALES` in `src/lib/i18n/locales.ts`; `label` is the native name.
4. Run `bun run i18n:status`, then `bun run verify`.
5. Check the layout at that locale; Chinese and English are the current references.

## Checking coverage

```bash
bun run i18n:status   # coverage table, missing/extra keys, placeholder mismatches
bun run i18n:check    # same checks, exits non-zero on any problem
```

`i18n:check` is part of `bun run verify`, so CI fails on missing keys, empty messages, mismatched placeholders, or invalid ICU syntax.

## Weblate

[Weblate](https://weblate.org) is the intended translation manager. It tracks per-language completion, checks placeholders, and commits translations back to Git. The repository is public and GPL-3.0-only, so the hosted libre plan applies.

Hosted setup, once, from a Weblate account:

1. Sign in at <https://hosted.weblate.org/> and create a project named **QiaoReader**.
2. Add a component with:
   - **Source repository:** `https://github.com/liraymond04/qiaoreader.git`
   - **File format:** `JSON file` (identifies flat JSON key/value catalogs)
   - **File mask:** `src/lib/i18n/messages/*.json`
   - **Monolingual base language file:** `src/lib/i18n/messages/en.json`
   - **Source language:** English
   - **Language code style:** BCP 47, so languages map to file names such as `zh-Hans.json`
3. Grant write access for pushing translations back:
   - Add Weblate's SSH public key as a GitHub **deploy key** with write access, and set the component push URL to `git@github.com:liraymond04/qiaoreader.git`; or
   - configure a GitHub app/token in the component settings.
4. Enable pushing from Weblate (or use pull requests) and let `bun run verify` gate the result. Translation commits run through the same key/ICU checks as manual edits.
5. Add the Weblate status badge to the README once the component exists.

Notes:

- Weblate's JSON format has no dedicated plural editor, but it preserves ICU strings intact. `bun run i18n:check` is the authority on ICU validity and placeholder parity.
- Self-hosted Weblate (Docker) and [Tolgee](https://tolgee.io) are viable alternatives if the hosted service is not a fit; the catalog format and checks do not change.
