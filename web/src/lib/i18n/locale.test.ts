import { describe, expect, it } from 'vitest';
import { initialLocale, isLocale } from './locale';

describe('initialLocale', () => {
  it('keeps a saved supported locale', () => {
    expect(initialLocale('en', ['de'])).toBe('en');
  });

  it('ignores saved values that are not locales', () => {
    expect(initialLocale('klingon', ['en-GB'])).toBe('en');
    expect(initialLocale(42, [])).toBe('en');
  });

  it('takes the first supported browser language, by its base', () => {
    expect(initialLocale(null, ['de-DE', 'en-US'])).toBe('en');
  });

  it('falls back to English', () => {
    expect(initialLocale(null, ['de', 'ja'])).toBe('en');
    expect(isLocale('en')).toBe(true);
  });
});
