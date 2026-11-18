import { describe, expect, it } from 'vitest';
import { setLocale } from './i18n/index.svelte';
import { en } from './i18n/en';
import { fr } from './i18n/fr';
import { FORMULAS, GLOSS_UNITS, NOTATIONS, glossName, glossSegments } from './glossary';

const glossed = (s: string) => glossSegments(s).filter((g) => 'kind' in g).map((g) => g.text);

describe('glossSegments', () => {
  it('round-trips the text', () => {
    const s = 'CO₂ 1.2 bar, 288 K and 5 M⊕.';
    expect(glossSegments(s).map((g) => g.text).join('')).toBe(s);
  });
  it('finds formulas next to punctuation', () => {
    expect(glossed('(N₂), CH₄/N₂ and CO₂.')).toEqual(['N₂', 'CH₄', 'N₂', 'CO₂']);
  });
  it('prefers the longest formula', () => expect(glossed('H₂O and H₂/He')).toEqual(['H₂O', 'H₂/He']));
  it('needs a number before letter units', () => {
    expect(glossed('K-type giant near Mars, a barrier of 4 Ga')).toEqual(['Ga']);
    expect(glossed('288 K')).toEqual(['K']);
  });
  it('reads French numbers and spaces', () => {
    expect(glossed('1,52 UA · 0,006 bar')).toEqual(['UA', 'bar']);
    expect(glossed('4,5 Ga and 3 km²')).toEqual(['Ga', 'km²']);
  });
  it('reads exponent numbers', () => expect(glossed('3.8×10²⁶ W')).toEqual(['W']));
  it('reads every superscript digit before a unit', () => {
    expect(glossed('3×10⁻¹² bar')).toEqual(['bar']);
    expect(glossed('2×10²³ W')).toEqual(['W']);
    expect(glossed('1.2×10¹³ W')).toEqual(['W']);
  });
  it('glosses the French M km² written with a narrow no-break space', () => {
    setLocale('fr');
    try {
      const text = '149\u202fM\u202fkm²';
      expect(glossed(text)).toEqual(['M\u202fkm²']);
      expect(glossSegments(text).map((g) => g.text).join('')).toBe(text);
      expect(glossName('unit', 'M\u202fkm²')).toBe(fr.glossary.units['M km²']);
      expect(glossed('149 M km²')).toEqual(['M km²']);
    } finally {
      setLocale('en');
    }
  });
  it('knows the Jupiter radius, the solar mass and the density unit', () => {
    expect(glossed('1.2 RJ and 0.3 M☉ and 5.51 g/cm³')).toEqual(['RJ', 'M☉', 'g/cm³']);
    expect(glossed('RJ without a number')).toEqual([]);
    expect(glossName('unit', 'g/cm³')).toBe(en.glossary.units['g/cm³']);
  });
  it('names g without a reference value', () => expect(en.glossary.units.g).not.toContain('='));
  it('finds symbol units anywhere', () => expect(glossed('masse en M⊕')).toEqual(['M⊕']));
  it('ignores ASCII lookalikes', () => expect(glossed('He said CO2')).toEqual([]));
  it('does not gloss k₂/Q', () => expect(glossed('k₂/Q')).toEqual([]));
  it('does not cut a longer word', () => expect(glossed('5 kmh, 5 Gyr, 5 km²')).toEqual(['Gyr', 'km²']));
  it('glosses [Fe/H] as an exact token, brackets included', () => {
    expect(glossed('Star metallicity [Fe/H]')).toEqual(['[Fe/H]']);
    expect(glossed('ice line 3 AU · [Fe/H] +0.1 · age 4 Ga')).toEqual(['[Fe/H]', 'AU', 'Ga']);
    expect(glossed('Fe/H and [Fe/H]x and a[Fe/H]')).toEqual([]);
    expect(glossName('notation', '[Fe/H]')).toBe('metallicity: iron abundance relative to the Sun (log scale)');
  });
  it('names the glossed terms', () => {
    expect(glossName('formula', 'CO₂')).not.toBeNull();
    expect(glossName('unit', 'bar')).not.toBeNull();
    expect(glossName('unit', 'zz')).toBeNull();
  });
});

describe('glossary catalogs', () => {
  it('names every formula and unit in both languages', () => {
    for (const catalog of [en, fr]) {
      for (const f of FORMULAS) expect(catalog.glossary.formulas[f].length).toBeGreaterThan(0);
      for (const n of NOTATIONS) expect(catalog.glossary.notations[n].length).toBeGreaterThan(0);
      for (const u of GLOSS_UNITS) expect(catalog.glossary.units[u].length).toBeGreaterThan(0);
    }
  });
});
