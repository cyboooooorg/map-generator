import { afterEach, describe, expect, it } from 'vitest';
import type { ArchetypeInfo } from '../api/types/ArchetypeInfo';
import { setLocale } from './index.svelte';
import { setArchetypeLocales, similarLine } from './names.svelte';

const archetype = (name: string, reference: ArchetypeInfo['reference'], locales: ArchetypeInfo['locales'] = {}) =>
  ({ name, description: '', reference, locales }) as unknown as ArchetypeInfo;

setArchetypeLocales([
  archetype('earth-like', { name: 'Earth', theoretical: false }, { fr: { name: 'Terrestre', reference: 'la Terre' } }),
  archetype('hot-jupiter', { name: '51 Pegasi b', discovered: 1995, theoretical: false }, { fr: { name: 'Jupiter chaud' } }),
  archetype('ocean-world', { theoretical: true }, { fr: { name: 'Monde océan' } }),
  archetype('my-own', null),
]);

describe('similarLine', () => {
  afterEach(() => setLocale('en'));

  it('names the Solar System body of the archetype, with its French article', () => {
    expect(similarLine('earth-like')).toEqual({ text: 'Similar to Earth', hint: undefined });
    setLocale('fr');
    expect(similarLine('earth-like').text).toBe('Semblable à la Terre');
  });

  it('names the first exoplanet of its kind, with its year in the hint', () => {
    expect(similarLine('hot-jupiter')).toEqual({ text: 'Similar to 51 Pegasi b', hint: 'first known of its kind, 1995' });
    setLocale('fr');
    expect(similarLine('hot-jupiter')).toEqual({ text: 'Semblable à 51 Pegasi b', hint: 'première connue de ce type, 1995' });
  });

  it('calls a type without a known planet theoretical', () => {
    expect(similarLine('ocean-world')).toEqual({ text: 'Theoretical type: ocean-world', hint: 'no such planet is known yet' });
    setLocale('fr');
    expect(similarLine('ocean-world')).toEqual({ text: 'Type théorique : Monde océan', hint: 'aucune planète de ce type n’est encore connue' });
  });

  it('keeps naming the archetype when its file has no reference', () => {
    expect(similarLine('my-own').text).toBe('Similar to my-own');
  });
});
