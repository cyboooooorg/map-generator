import { describe, expect, it } from 'vitest';
import { parseFormMemory } from './formMemory';

const FIELDS = ['axial_tilt_deg'];
const ARCHETYPES = ['earth-like', 'venus-like', 'mars-like'];

describe('parseFormMemory', () => {
  it('keeps a well-formed form', () => {
    const stored = {
      planet: 'mars-like',
      randomPlanet: false,
      size: '1024x512',
      fields: { axial_tilt_deg: { random: false, value: 25 } },
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
      moons: 3,
      giant: { metallicityRandom: false, metallicity: 0.3, giants: 'none', alone: false, host: 'giant', massRandom: false, mass: 95 },
    };
    expect(parseFormMemory(stored, FIELDS, ARCHETYPES)).toEqual(stored);
    expect(parseFormMemory({ ...stored, moons: 'none' }, FIELDS, ARCHETYPES)?.moons).toBe('none');
    // An unknown or out-of-range number of moons falls back to the seed's.
    expect(parseFormMemory({ ...stored, moons: 9 }, FIELDS, ARCHETYPES)?.moons).toBe('auto');
  });

  it('drops malformed or unknown parameters and defaults the rest', () => {
    const parsed = parseFormMemory(
      {
        planet: 'terran',
        size: '2048x1024',
        fields: { axial_tilt_deg: { random: 'no', value: 1 }, sea_level: { random: false, value: 2 }, gone: {} },
        star: { klass: 'Q' },
        moons: 'lots',
      },
      FIELDS,
      ARCHETYPES,
    );
    expect(parsed).toEqual({
      planet: 'earth-like',
      randomPlanet: false,
      size: '2048x1024',
      fields: {},
      star: null,
      moons: 'auto',
      // A form saved before the giant fields: all drawn.
      giant: { metallicityRandom: true, metallicity: 0, giants: 'auto', alone: false, host: 'auto', massRandom: true, mass: 318 },
    });
  });

  it('refuses a form without its planet or size', () => {
    for (const stored of [null, 'terran', { planet: 'terran' }, { planet: 'terran', size: 'big' }, { size: '1x1' }]) {
      expect(parseFormMemory(stored, FIELDS, ARCHETYPES)).toBeNull();
    }
  });

  it('old_form_memory_falls_back_to_earth_like', () => {
    const parsed = parseFormMemory(
      { planet: 'terran', size: '1024x512', fields: { sea_level: { random: false, value: 0.2 }, axial_tilt_deg: { random: false, value: 10 } } },
      FIELDS,
      ARCHETYPES,
    );
    expect(parsed?.planet).toBe('earth-like');
    expect(parsed?.fields).toEqual({ axial_tilt_deg: { random: false, value: 10 } });
    expect(parsed).not.toHaveProperty('seaBy');
  });
});
