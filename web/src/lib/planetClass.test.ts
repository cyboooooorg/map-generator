import { describe, expect, it } from 'vitest';
import type { Body } from './api/types/Body';
import { en } from './i18n/en';
import { fr } from './i18n/fr';
import { GIANT_CLASSES, isGiantType, isHabitable } from './planetClass';

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

const planet = (liquid: string | null, extra: Partial<Body> = {}): Body =>
  ({
    kind: 'planet',
    type: 'rocky',
    model: liquid === null ? null : { surface: { climate: { liquid } } },
    ...extra,
  }) as unknown as Body;

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

  it('is habitable with liquid water on the surface only', () => {
    expect(isHabitable(planet('water'))).toBe(true);
    expect(isHabitable(planet('none'))).toBe(false);
    expect(isHabitable(planet('lava'))).toBe(false);
    expect(isHabitable(planet('water', { kind: 'moon' }))).toBe(false);
    expect(isHabitable({ kind: 'planet', type: 'ocean-world', model: { surface: null } } as unknown as Body)).toBe(false);
  });

  it('falls back to the preset for a planet asked for, which has no model', () => {
    expect(isHabitable(planet(null, { type: 'terran' }))).toBe(true);
    expect(isHabitable(planet(null, { type: 'barren' }))).toBe(false);
  });
});
