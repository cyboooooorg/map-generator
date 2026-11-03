// The active locale and its messages.
//
// Components read messages through `t` (`t.viewer.layers.elevation`): the
// proxy reads the locale, a `$state`, so templates and `$derived` values
// follow a change of language without a reload. Formatting of numbers,
// dates and units lives in `lib/format.ts`, which reads `getLocale()`.
import { loadSetting, saveSetting } from '../settings';
import type { Widen } from './catalog';
import { en } from './en';
import { initialLocale, type Locale } from './locale';

export { LOCALES, LOCALE_NAMES, type Locale } from './locale';

export type Messages = Widen<typeof en>;

const catalogs: Record<Locale, Messages> = { en };

const SETTING = 'locale';

const current = $state({
  locale: initialLocale(loadSetting<unknown>(SETTING, null), globalThis.navigator?.languages ?? []),
});

/** The active locale. */
export function getLocale(): Locale {
  return current.locale;
}

/** Switches the language, and remembers the choice. */
export function setLocale(locale: Locale): void {
  current.locale = locale;
  saveSetting(SETTING, locale);
}

/** The messages of the active locale. */
export const t: Messages = new Proxy({} as Messages, {
  get(_, key) {
    return catalogs[current.locale][key as keyof Messages];
  },
});

/** The messages of a given locale, for tests and previews. */
export function messagesOf(locale: Locale): Messages {
  return catalogs[locale];
}
