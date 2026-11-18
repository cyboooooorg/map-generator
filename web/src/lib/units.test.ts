import { afterEach, describe, expect, it } from 'vitest';
import { GLOSS_UNITS } from './glossary';
import { setLocale, t } from './i18n/index.svelte';
import { UNITS } from './units';

describe('units', () => {
  afterEach(() => setLocale('en'));

  it('explains every glossed unit but the degree Celsius, in both languages', () => {
    expect([...UNITS].sort()).toEqual(GLOSS_UNITS.filter((u) => u !== '°C').sort());
    for (const locale of ['en', 'fr'] as const) {
      setLocale(locale);
      for (const unit of UNITS) {
        expect(t.units[unit].name.length, `${locale} ${unit} name`).toBeGreaterThan(0);
        expect(t.units[unit].examples.length, `${locale} ${unit} examples`).toBeGreaterThan(0);
      }
    }
  });

  it('includes the units of the star and planet cards', () => {
    for (const unit of ['M⊕', 'R⊕', 'M♃', 'S⊕', 'K', 'W/m²'] as const) expect(UNITS).toContain(unit);
  });
});
