import { describe, expect, it } from 'vitest';
import { en } from './i18n/en';
import { fr } from './i18n/fr';
import { className, classLabel } from './i18n/names.svelte';
import { GIANT_CLASSES, hasFrozenSurface, isGiantType, surfaceTemperatureK } from './planetClass';

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
  'hot-neptune',
  'lava-world',
  'volcanic-world',
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

  it('words no old type', () => {
    for (const key of ['terran', 'barren', 'caustic', 'frozen', 'volcanic']) {
      expect(en.viewer.bodyTypes, key).not.toHaveProperty(key);
      expect(fr.viewer.bodyTypes, key).not.toHaveProperty(key);
    }
  });

  it('draws the giants and the mini-Neptune as giants', () => {
    expect([...GIANT_CLASSES].sort()).toEqual(['gas-giant', 'hot-jupiter', 'hot-neptune', 'ice-giant', 'mini-neptune']);
    expect(isGiantType('super-earth')).toBe(false);
    expect(isGiantType('terran')).toBe(false);
  });

  const model = (oceans: number, liquid: string, k: number) => ({ surface: { water: { oceans }, climate: { temperature_k: k, liquid } } });

  it('tells a rocky world whose water is frozen at the surface', () => {
    expect(hasFrozenSurface('rocky', model(0.25, 'none', 215))).toBe(true);
    expect(hasFrozenSurface('super-earth', model(0.01, 'none', 272))).toBe(true);
    expect(hasFrozenSurface('rocky', model(0.005, 'none', 215))).toBe(false);
    expect(hasFrozenSurface('rocky', model(0.25, 'water', 285))).toBe(false);
    expect(hasFrozenSurface('rocky', model(0.25, 'none', 280))).toBe(false);
    expect(hasFrozenSurface('ice-world', model(5, 'none', 100))).toBe(false);
    expect(hasFrozenSurface('ocean-world', model(20, 'none', 200))).toBe(false);
    expect(hasFrozenSurface('ice-giant', model(1, 'none', 100))).toBe(false);
    expect(hasFrozenSurface('rocky', { surface: null })).toBe(false);
    expect(hasFrozenSurface('rocky', undefined)).toBe(false);
  });

  it('reads the surface temperature of a model that has a surface', () => {
    expect(surfaceTemperatureK(model(0, 'water', 288))).toBe(288);
    expect(surfaceTemperatureK({ surface: null })).toBeNull();
    expect(surfaceTemperatureK(undefined)).toBeNull();
  });

  it('words a round moon as a moon wherever the planet label says planet', () => {
    expect(className('rocky')).toBe('Rocky planet');
    expect(className('rocky', true)).toBe('Rocky moon');
    expect(className('volcanic-world')).toBe('Volcanic world');
    expect(className('volcanic-world', true)).toBe('Volcanic moon');
    expect(className('ice-world', true)).toBe('Icy moon');
    expect(className('icy-moon', true)).toBe('icy moon');
    expect(classLabel('rocky', undefined, true)).toBe('Rocky moon');
    expect(classLabel('rocky', model(0.25, 'none', 215), true)).toBe('Rocky moon · frozen surface');
    expect(classLabel('rocky')).toBe('Rocky planet');
  });

  it('keeps the moon wordings in both languages and none that say planet', () => {
    const en_ = en.viewer.moonBodyTypes as Record<string, string>;
    const fr_ = fr.viewer.moonBodyTypes as Record<string, string>;
    expect(Object.keys(fr_).sort()).toEqual(Object.keys(en_).sort());
    for (const key of Object.keys(en_)) {
      expect(PLANET_CLASSES, key).toContain(key);
      expect(en_[key], key).not.toMatch(/planet/i);
      expect(fr_[key], key).not.toMatch(/plan[èe]te/i);
    }
    // Every class has a moon label, none reading as a world.
    for (const key of PLANET_CLASSES) {
      expect(en_[key], key).toBeTruthy();
      expect(fr_[key], key).toBeTruthy();
      expect(en_[key], key).not.toMatch(/world/i);
      expect(fr_[key], key).not.toMatch(/monde/i);
    }
  });
});
