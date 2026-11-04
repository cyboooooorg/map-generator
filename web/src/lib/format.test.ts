import { describe, expect, it } from 'vitest';
import type { WorldMetadata } from './api/types/WorldMetadata';
import { cliCommand, formatAgo, formatAtmosphere, formatArea, formatGeology, formatDuration, formatShare, MAX_SEED, randomSeed, requestFromWorld, rerollRequest, surfaceColour } from './format';

const meta = {
  planet_type: 'frozen',
  seed: 2590618090,
  sea_level: 0.121345,
  volcanic_intensity: 0.61071,
  circumference_km: 51823.43,
  axial_tilt_deg: 18.7234,
  erosion: 1,
  pressure_bar: 0.41237,
  surface_age_ga: 1.23456,
  star: { type: 'M4.5V', orbit_au: 0.0812345, eccentricity: 0.123456 },
  width: 1920,
  height: 1080,
} as WorldMetadata;

describe('format', () => {
  it('rebuilds the request and the command line of a world', () => {
    expect(requestFromWorld(meta)).toEqual({
      planet: 'frozen',
      seed: 2590618090,
      sea_level: 0.1213,
      volcanic: 0.6107,
      circumference_km: 51823.4,
      axial_tilt_deg: 18.72,
      erosion: 1,
      pressure_bar: 0.4124,
      surface_age_ga: 1.235,
      star: 'M4.5V',
      orbit_au: 0.08123,
      eccentricity: 0.1235,
      width: 1920,
      height: 1080,
    });
    expect(cliCommand(meta)).toBe(
      'mise run gen -- --planet frozen --seed 2590618090 --sea-level 0.1213 --volcanic 0.6107 ' +
        '--circumference 51823.4 --axial-tilt 18.72 --erosion 1 --pressure 0.4124 --surface-age 1.235 ' +
        '--star M4.5V --orbit-au 0.08123 --eccentricity 0.1235 --width 1920 --height 1080',
    );
    // Worlds made before the surface age leave it to the preset.
    const old = { ...meta, surface_age_ga: undefined } as unknown as WorldMetadata;
    expect(requestFromWorld(old)).not.toHaveProperty('surface_age_ga');
    expect(cliCommand(old)).not.toContain('--surface-age');
    // Worlds made before stars orbit the Sun at 1 AU.
    const sunlit = { ...meta, star: undefined } as unknown as WorldMetadata;
    expect(cliCommand(sunlit)).toContain('--star G2V --orbit-au 1 --eccentricity 0 ');
  });

  it('rerolls a world with a new seed and every other parameter kept', () => {
    expect(rerollRequest(meta, 7)).toEqual({ ...requestFromWorld(meta), seed: 7 });
    const seed = rerollRequest(meta).seed ?? -1;
    expect(Number.isInteger(seed) && seed >= 0 && seed <= MAX_SEED).toBe(true);
    expect(Number.isInteger(randomSeed())).toBe(true);
  });

  it('describes the atmosphere, with Earth for worlds made before it', () => {
    const preset = { atmosphere: { clouds: 'dust' } };
    expect(formatAtmosphere({ ...meta, pressure_bar: 0.006, preset } as unknown as WorldMetadata)).toBe(
      '0.0060 bar, dust clouds',
    );
    expect(formatAtmosphere({ ...meta, pressure_bar: 0, preset } as unknown as WorldMetadata)).toBe('none');
    const old = { ...meta, preset: {} } as unknown as WorldMetadata;
    delete (old as { pressure_bar?: number }).pressure_bar;
    expect(formatAtmosphere(old)).toBe('1.00 bar, water clouds');
  });

  it("describes the geology, with Earth's regime for worlds made before it", () => {
    const preset = { geology: { regime: 'stagnant', wet_until: 3.5 } };
    expect(formatGeology({ ...meta, surface_age_ga: 3.8, preset } as unknown as WorldMetadata)).toBe(
      'stagnant lid, surface 3.8 Ga',
    );
    expect(formatGeology({ ...meta, surface_age_ga: 0.02, preset } as unknown as WorldMetadata)).toBe(
      'stagnant lid, surface 20 Ma',
    );
    const old = { ...meta, surface_age_ga: undefined, preset: {} } as unknown as WorldMetadata;
    expect(formatGeology(old)).toBe('plate tectonics');
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
  it('falls back to the classification colour for older worlds', () => {
    expect(surfaceColour({ color: '#FF0000', surface_color: '' })).toBe('#FF0000');
  });
});
