import { describe, expect, it } from 'vitest';
import { en } from './i18n/en';
import { fr } from './i18n/fr';
import { GIANT_CLASSES, isGiantType } from './planetClass';

// Copied from `PLANET_CLASSES` (crates/mapgen/src/planet/classify.rs); a
// server test (app.rs) reads this file and checks the list against it.
const PLANET_CLASSES = [
  'rocky',
  'super-earth',
  'ice-world',
  'mini-neptune',
  'ice-giant',
  'gas-giant',
  'hot-jupiter',
  'lava-world',
  'runaway-greenhouse',
  'hydrocarbon-world',
  'ocean-world',
];

describe('planet classes', () => {
  it('words every class in English and French', () => {
    for (const key of PLANET_CLASSES) {
      expect((en.viewer.bodyTypes as Record<string, string>)[key], key).toBeTruthy();
      expect((fr.viewer.bodyTypes as Record<string, string>)[key], key).toBeTruthy();
    }
  });

  it('draws the giants and the mini-Neptune as giants', () => {
    expect([...GIANT_CLASSES].sort()).toEqual(['gas-giant', 'hot-jupiter', 'ice-giant', 'mini-neptune']);
    expect(isGiantType('super-earth')).toBe(false);
    expect(isGiantType('terran')).toBe(false);
  });

});
