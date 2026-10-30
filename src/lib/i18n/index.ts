import { browser } from '$app/environment';
import { init, locale, register, waitLocale } from 'svelte-i18n';
import { get } from 'svelte/store';

import { DEFAULT_LOCALE, SUPPORTED_LOCALES } from './locales';

const STORAGE_KEY = 'qiaoreader.locale';

register('en', () => import('./messages/en.json'));
register('zh-Hans', () => import('./messages/zh-Hans.json'));

/**
 * Start svelte-i18n with the stored locale, or the best match for the OS
 * language, falling back to English.
 */
export function initI18n(): void {
  init({
    fallbackLocale: DEFAULT_LOCALE,
    initialLocale: detectLocale(),
  });
}

/** Wait for the initial catalog before the first paint. */
export function preloadLocale(): Promise<void> {
  return waitLocale();
}

/** Switch locale and persist the choice for the next launch. */
export function setLocale(code: string): void {
  if (!isSupportedLocale(code) || code === getLocale()) return;
  if (browser) localStorage.setItem(STORAGE_KEY, code);
  locale.set(code);
}

export function getLocale(): string {
  return get(locale) ?? DEFAULT_LOCALE;
}

export function isSupportedLocale(code: string): boolean {
  return SUPPORTED_LOCALES.some((item) => item.code === code);
}

function detectLocale(): string {
  if (!browser) return DEFAULT_LOCALE;

  const stored = localStorage.getItem(STORAGE_KEY);
  if (stored && isSupportedLocale(stored)) return stored;

  for (const candidate of navigator.languages ?? [navigator.language]) {
    if (isSupportedLocale(candidate)) return candidate;

    const base = candidate.split('-')[0];
    const match = SUPPORTED_LOCALES.find(
      (item) => item.code.split('-')[0] === base,
    );
    if (match) return match.code;
  }

  return DEFAULT_LOCALE;
}
