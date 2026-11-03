// Supported locales and the choice of the first one, kept free of Svelte
// state so it can be tested on its own.

/** Locales the web app is translated into; English is the source. */
export const LOCALES = ['en', 'fr'] as const;
export type Locale = (typeof LOCALES)[number];

/** Native name of each locale, for the language selector. */
export const LOCALE_NAMES: Record<Locale, string> = { en: 'English', fr: 'Français' };

export function isLocale(value: unknown): value is Locale {
  return typeof value === 'string' && (LOCALES as readonly string[]).includes(value);
}

/**
 * The locale to start with: the saved choice if any, else the first of the
 * browser's languages that is supported (`fr-CA` gives `fr`), else English.
 */
export function initialLocale(saved: unknown, languages: readonly string[]): Locale {
  if (isLocale(saved)) {
    return saved;
  }
  for (const language of languages) {
    const base = language.toLowerCase().split('-')[0];
    if (isLocale(base)) {
      return base;
    }
  }
  return 'en';
}
