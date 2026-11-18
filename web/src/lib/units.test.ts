import { afterEach, describe, expect, it } from 'vitest';
import { GLOSS_UNITS } from './glossary';
import { setLocale, t } from './i18n/index.svelte';
import { UNITS, unitTooltip } from './units';

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

  it('keeps tooltips to the short name, one line per unit', () => {
    expect(unitTooltip('bar')).toBe('bar: pressure');
    expect(unitTooltip('Ga', 'Ma').split('\n')).toEqual(['Ga: billion years', 'Ma: million years']);
    expect(unitTooltip('bar')).not.toContain('Venus');
  });

  it('is translated', () => {
    setLocale('fr');
    expect(unitTooltip('Ma')).toContain('million d’années');
  });
});
