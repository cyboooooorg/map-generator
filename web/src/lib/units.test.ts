import { afterEach, describe, expect, it } from 'vitest';
import { setLocale } from './i18n/index.svelte';
import { UNITS, unitTooltip } from './units';

describe('units', () => {
  afterEach(() => setLocale('en'));

  it('explains each unit with reference values', () => {
    expect(unitTooltip('Ga')).toBe('Ga: a billion years (10⁹ years, 1 000 Ma). Earth and the Solar System: 4.5 Ga · lunar maria: 3 to 3.9 Ga');
    expect(unitTooltip('Ga', 'Ma').split('\n')).toHaveLength(2);
    for (const unit of UNITS) {
      expect(unitTooltip(unit).startsWith(`${unit}:`)).toBe(true);
    }
  });

  it('is translated', () => {
    setLocale('fr');
    expect(unitTooltip('Ma')).toContain('un million d’années');
    expect(unitTooltip('g')).toContain('0,38');
  });
});
