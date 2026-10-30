/**
 * Supported UI locales. Add a locale here after adding its catalog under
 * `messages/<code>.json`; the code must be a BCP 47 tag because it doubles as
 * the file name and the Weblate language code.
 */
export const DEFAULT_LOCALE = 'en';

export type LocaleDefinition = {
  code: string;
  /** Native name, shown in the language picker. */
  label: string;
};

export const SUPPORTED_LOCALES: LocaleDefinition[] = [
  { code: 'en', label: 'English' },
  { code: 'zh-Hans', label: '简体中文' },
];
