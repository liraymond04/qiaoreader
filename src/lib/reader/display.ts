export type DisplayMode = "hidden" | "pinyin" | "full";

export const DEFAULT_DISPLAY_MODE: DisplayMode = "hidden";

export function isDisplayMode(value: string | null): value is DisplayMode {
  return value === "hidden" || value === "pinyin" || value === "full";
}
