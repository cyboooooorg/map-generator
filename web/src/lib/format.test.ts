import { describe, expect, it } from 'vitest';
import type { System } from './api/types/System';
import type { WorldMetadata } from './api/types/WorldMetadata';
import { driftRequest } from './drift';
import { setLocale } from './i18n/index.svelte';
import { keepAlone, cliCommand, formatInt, formatNumber, formatUpTo, formatAgo, formatAtmosphere, formatArea, formatGeology, formatAge, formatSurfaceAge, formatDuration, formatShare, MAX_SEED, randomSeed, requestFromWorld, rerollRequest, surfaceColour } from './format';

const S = '\u202f';

const meta = {
  class: 'rocky',
  archetype: 'earth-like',
  seed: 2590618090,
  axial_tilt_deg: 18.7234,
  pressure_bar: 0.41237,
  surface_age_ga: 1.23456,
  star: { type: 'M4.5V', orbit_au: 0.0812345, eccentricity: 0.123456, metallicity: 0.1, age_gyr: 8.123456 },
  moons: { planet_day_hours: 24 },
  giants: { count: 0 },
  model: {
    inputs: { mass_earth: 1.02, core_mass_fraction: 0.33, water_fraction: 2.5e-4, envelope_fraction: 0, radiogenic: 1.1 },
    surface: { life: { arose: true } },
  },
  width: 1920,
  height: 1080,
} as unknown as WorldMetadata;

describe('format', () => {
  it('rebuilds the request and the command line of a world', () => {
    expect(requestFromWorld(meta)).toEqual({
      planet: 'earth-like',
      seed: 2590618090,
      mass_earth: 1.02,
      core_mass_fraction: 0.33,
      water_oceans: 1.08776,
      envelope_fraction: 0,
      radiogenic: 1.1,
      life: true,
      axial_tilt_deg: 18.72,
      star: 'M4.5V',
      orbit_au: 0.08123,
      eccentricity: 0.1235,
      age_gyr: 8.123456,
      metallicity: 0.1,
      width: 1920,
      height: 1080,
    });
    expect(cliCommand(meta)).toBe(
      'mise run gen -- --planet earth-like --seed 2590618090 --mass 1.02 --core-fraction 0.33 --water 1.08776 ' +
        '--envelope 0 --radiogenic 1.1 --life yes --axial-tilt 18.72 ' +
        '--star M4.5V --orbit-au 0.08123 --eccentricity 0.1235 --age 8.123456 --metallicity=0.1 --width 1920 --height 1080',
    );
    // An epoch of the plates, and moons turned off, are reproduced too.
    const drifted = { ...meta, drift_ma: -120, moons: null } as unknown as WorldMetadata;
    expect(requestFromWorld(drifted)).toMatchObject({ drift_ma: -120, moons: 'none' });
    expect(cliCommand(drifted)).toContain('--drift=-120 --moons none --metallicity=0.1 --width');
    expect(requestFromWorld({ ...drifted, moons: { planet_day_hours: 24 } } as unknown as WorldMetadata)).not.toHaveProperty('moons');
    const counted = { ...drifted, moons: { planet_day_hours: 24, count_asked: 3 } } as unknown as WorldMetadata;
    expect(requestFromWorld(counted).moons).toEqual({ count: 3 });
    expect(cliCommand(counted)).toContain('--moons 3');
    // A moon has no model: nothing of the model is pinned, and no archetype is sent.
    const moon = { ...meta, archetype: null, model: null } as unknown as WorldMetadata;
    expect(requestFromWorld(moon)).not.toHaveProperty('planet');
    expect(requestFromWorld(moon)).not.toHaveProperty('mass_earth');
    expect(cliCommand(moon)).not.toMatch(/--(planet|mass|water|life) /);
    // Life that did not arise is left to the draw.
    const dead = { ...meta, model: { ...meta.model, surface: { life: { arose: false } } } } as unknown as WorldMetadata;
    expect(requestFromWorld(dead)).not.toHaveProperty('life');
  });

  it('requestFromWorld_pins_the_model_inputs', () => {
    const request = requestFromWorld(meta);
    expect(request).not.toHaveProperty('sea_level');
    expect(request.water_oceans).toBeCloseTo((2.5e-4 * 1.02 * 5.972e24) / 1.4e21, 4);
    // A giant has no surface: its mass and envelope are pinned, no life.
    const giant = { ...meta, model: { inputs: { ...meta.model!.inputs, mass_earth: 318, envelope_fraction: 0.95 }, surface: null } } as unknown as WorldMetadata;
    expect(requestFromWorld(giant)).toMatchObject({ mass_earth: 318, envelope_fraction: 0.95 });
    expect(requestFromWorld(giant)).not.toHaveProperty('life');
  });

  it('rerolls a world with a new seed and every other parameter kept', () => {
    expect(rerollRequest(meta, 7)).toEqual({ ...requestFromWorld(meta), seed: 7 });
    // A planet of a system keeps the age of its star under a new seed.
    expect(rerollRequest(meta, 7).age_gyr).toBe(8.123456);
    const seed = rerollRequest(meta).seed ?? -1;
    expect(Number.isInteger(seed) && seed >= 0 && seed <= MAX_SEED).toBe(true);
    expect(Number.isInteger(randomSeed())).toBe(true);
  });

  it('describes the atmosphere', () => {
    const look = { atmosphere: { clouds: 'dust' } };
    expect(formatAtmosphere({ ...meta, pressure_bar: 0.006, look } as unknown as WorldMetadata)).toBe(
      '0.0060 bar, dust clouds',
    );
    expect(formatAtmosphere({ ...meta, pressure_bar: 0, look } as unknown as WorldMetadata)).toBe('none');
  });

  it("describes the geology", () => {
    const look = { geology: { regime: 'stagnant', wet_until: 3.5 } };
    expect(formatGeology({ ...meta, surface_age_ga: 3.8, look } as unknown as WorldMetadata)).toBe(
      'stagnant lid, surface 3.8 billion years (3.8 Ga)',
    );
    expect(formatGeology({ ...meta, surface_age_ga: 0.02, look } as unknown as WorldMetadata)).toBe(
      'stagnant lid, surface 20 million years (20 Ma)',
    );
  });

  it('formats shares, areas, durations and ages', () => {
    expect(formatShare(0.25)).toBe('25 %');
    expect(formatShare(0.0123)).toBe('1.2 %');
    expect(formatShare(0.0004)).toBe('<0.1 %');
    expect(formatArea(510_000_000)).toBe('510 M km²');
    expect(formatArea(12_345)).toBe('12,300 km²');
    expect(formatArea(1_130_000_000)).toBe('1,130 M km²');
    expect(formatArea(304.4)).toBe('304 km²');
    expect(formatDuration(42.4)).toBe('42 s');
    expect(formatDuration(185)).toBe('3 min 05 s');
    expect(formatAgo(1_000_000, 1_000_000 + 2 * 3_600_000)).toBe('2 hours ago');
    expect(formatAgo(1_000_000, 1_000_000 + 5_000)).toBe('just now');
  });
});

describe('surfaceColour', () => {
  it('prefers the surface colour', () => {
    expect(surfaceColour({ color: '#FF0000', surface_color: '#102030' })).toBe('#102030');
  });
  it('falls back to the classification colour without a surface colour', () => {
    expect(surfaceColour({ color: '#FF0000', surface_color: '' })).toBe('#FF0000');
  });
});

describe('signed zero', () => {
  it('prints a negative number rounded to zero as 0', () => {
    expect(formatInt(-0)).toBe('0');
    expect(formatInt(-0.2)).toBe('0');
    expect(formatNumber(-0.04, 1)).toBe('0.0');
    expect(formatUpTo(-0.001, 1)).toBe('0');
    expect(formatInt(-3)).toBe('-3');
  });
});

describe('keepAlone', () => {
  const stored = (request: unknown) => ({ request }) as unknown as System;
  it('carries the lone flag of the system into the request of Vary, a reroll or an epoch', () => {
    const lone = stored({ around: { planet: 'terran', alone: true } });
    expect(keepAlone(requestFromWorld(meta), lone).alone).toBe(true);
    expect(keepAlone(rerollRequest(meta, 5), lone).alone).toBe(true);
    expect(keepAlone(driftRequest(meta, 250), lone)).toMatchObject({ alone: true });
  });
  it('leaves other systems alone', () => {
    const request = requestFromWorld(meta);
    expect(keepAlone(request, stored({ around: { planet: 'terran' } }))).toBe(request);
    expect(keepAlone(request, stored({ around: { alone: false } }))).toBe(request);
    expect(keepAlone(request, stored({ system: {} }))).toBe(request);
    expect(keepAlone(request, null)).toBe(request);
  });
});

describe('ages on the cards', () => {
  it('spells the system age out in billions or millions of years', () => {
    expect(formatAge(8.123456)).toBe('8.12 billion years (8.12 Ga)');
    expect(formatAge(1.5)).toBe('1.5 billion years (1.5 Ga)');
    expect(formatAge(0.6)).toBe('600 million years (600 Ma)');
    expect(formatAge(0.001)).toBe('1 million years (1 Ma)');
    expect(formatAge(meta.star!.age_gyr)).toContain('8.12');
  });

  it('spells a surface age out the same way', () => {
    expect(formatSurfaceAge(3.84)).toBe('3.84 billion years (3.84 Ga)');
    expect(formatSurfaceAge(0.4234)).toBe('423 million years (423 Ma)');
    expect(formatSurfaceAge(0.02)).toBe('20 million years (20 Ma)');
  });

  it('spells ages out in French', () => {
    setLocale('fr');
    try {
      expect(formatAge(8.123456)).toBe(`8,12${S}milliards d’années (8,12${S}Ga)`);
      expect(formatAge(1.5)).toBe(`1,5${S}milliard d’années (1,5${S}Ga)`);
      expect(formatAge(0.6)).toBe(`600${S}millions d’années (600${S}Ma)`);
      expect(formatAge(0.001)).toBe(`1${S}million d’années (1${S}Ma)`);
    } finally {
      setLocale('en');
    }
  });
});
