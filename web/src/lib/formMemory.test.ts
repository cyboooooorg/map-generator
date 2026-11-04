import { describe, expect, it } from 'vitest';
import { parseFormMemory } from './formMemory';

const FIELDS = ['sea_level', 'pressure_bar'];

describe('parseFormMemory', () => {
  it('keeps a well-formed form', () => {
    const stored = {
      planet: 'frozen',
      randomPlanet: false,
      size: '1024x512',
      seaBy: 'land',
      landPercent: 40,
      fields: { sea_level: { random: true, value: 0 }, pressure_bar: { random: false, value: 0.006 } },
      star: {
        klass: 'M',
        subtype: 4.5,
        orbitBy: 'position',
        orbitRandom: false,
        position: 0.4,
        au: 1,
        eccentricityRandom: false,
        eccentricity: 0.05,
      },
    };
    expect(parseFormMemory(stored, FIELDS)).toEqual(stored);
  });

  it('drops malformed or unknown parameters and defaults the rest', () => {
    const parsed = parseFormMemory(
      {
        planet: 'terran',
        size: '2048x1024',
        seaBy: 'sideways',
        landPercent: 'many',
        fields: { sea_level: { random: 'no', value: 1 }, pressure_bar: { random: false, value: 2 }, gone: {} },
        star: { klass: 'Q' },
      },
      FIELDS,
    );
    expect(parsed).toEqual({
      planet: 'terran',
      randomPlanet: false,
      size: '2048x1024',
      seaBy: 'level',
      landPercent: 29,
      fields: { pressure_bar: { random: false, value: 2 } },
      star: null,
    });
  });

  it('refuses a form without its planet or size', () => {
    for (const stored of [null, 'terran', { planet: 'terran' }, { planet: 'terran', size: 'big' }, { size: '1x1' }]) {
      expect(parseFormMemory(stored, FIELDS)).toBeNull();
    }
  });
});
