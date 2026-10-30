import type { DictEntry, DictWord } from "./types";

export type PinyinToken = { text: string; tone: number };
export type RubySyllable = { char: string; pinyin: string; tone: number };

/** CC-CEDICT numeric tone, where 5 is neutral. Zero means "no tone data". */
export function toneOf(syllable: string): number {
  const match = syllable.match(/([1-5])$/);
  if (!match) return 0;
  const tone = Number(match[1]);
  return tone >= 1 && tone <= 4 ? tone : 0;
}

export function pinyinTokens(entry: DictEntry): PinyinToken[] {
  const numeric = entry.pinyin.trim().split(/\s+/);
  const display = entry.pinyinDisplay.trim().split(/\s+/);

  return display.map((text, index) => ({
    text,
    tone: numeric.length === display.length ? toneOf(numeric[index]) : 0,
  }));
}

/**
 * Align each Han character with its syllable for ruby layout. Returns null when
 * the counts differ (names with middle dots, acronyms, rare multi-syllable
 * characters), in which case the card falls back to a single pinyin line.
 */
export function rubySyllables(word: DictWord): RubySyllable[] | null {
  const entry = word.entries[0];
  if (!entry) return null;

  const chars = Array.from(word.text);
  const numeric = entry.pinyin.trim().split(/\s+/);
  const display = entry.pinyinDisplay.trim().split(/\s+/);

  if (
    chars.length === 0 ||
    chars.length !== numeric.length ||
    chars.length !== display.length
  ) {
    return null;
  }

  return chars.map((char, index) => ({
    char,
    pinyin: display[index],
    tone: toneOf(numeric[index]),
  }));
}

export function toneColor(tone: number): string {
  return tone >= 1 && tone <= 4 ? `var(--tone-${tone})` : "var(--tone-neutral)";
}
