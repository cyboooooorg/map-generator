import { afterEach, describe, expect, it } from 'vitest';
import type { WorldMetadata } from './api/types/WorldMetadata';
import { driftPreviewUrl, driftRange, driftRequest, formatDrift, snapDrift } from './drift';
import { setLocale } from './i18n/index.svelte';

const meta = (overrides: Record<string, unknown> = {}) =>
  ({
    format_version: 14,
    planet_type: 'terran',
    seed: 7,
    sea_level: 0.3,
    volcanic_intensity: 0.2,
    circumference_km: 40000,
    erosion: 1,
    axial_tilt_deg: 23,
    pressure_bar: 1,
    surface_age_ga: 4,
    star: { type: 'G2V', orbit_au: 1, eccentricity: 0, metallicity: 0 },
    width: 1024,
    height: 512,
    preset: { geology: { regime: 'plates', wet_until: 0 } },
    ...overrides,
  }) as unknown as WorldMetadata;

describe('drift', () => {
  afterEach(() => setLocale('en'));

  it('is offered by worlds with plates only', () => {
    expect(driftRange(meta())).toEqual([-300, 300]);
    expect(driftRange(meta({ preset: { geology: { regime: 'stagnant' } } }))).toBeNull();
    expect(driftRange(meta({ giant: { mass_earth: 300 } }))).toBeNull();
  });

  it('snaps to the slider stops', () => {
    expect(snapDrift(14)).toBe(10);
    expect(snapDrift(-96)).toBe(-100);
    expect(snapDrift(500)).toBe(300);
    expect(snapDrift(-500)).toBe(-300);
  });

  it('reads negative as the past', () => {
    expect(formatDrift(0)).toBe('present');
    expect(formatDrift(-150)).toMatch(/150.*Ma ago/);
    expect(formatDrift(50)).toMatch(/in 50.*Ma/);
  });

  it('names the preview in the body routes', () => {
    expect(driftPreviewUrl('systems/mgs-1-42/bodies/c', '2048x1024', -120, 256, 9)).toBe(
      '/api/systems/mgs-1-42/bodies/c/drift/-120/surface.png?rev=9&size=2048x1024&width=256',
    );
    expect(driftPreviewUrl('systems/mgs-1-42/bodies/c', '1024x512', 0, 512, null)).toBe(
      '/api/systems/mgs-1-42/bodies/c/drift/0/surface.png?size=1024x512&width=512',
    );
  });

  it('asks for the same world at another epoch', () => {
    expect(driftRequest(meta(), -150)).toMatchObject({ planet: 'terran', seed: 7, drift_ma: -150 });
    // The present leaves the epoch out, even from a drifted world.
    expect(driftRequest(meta({ drift_ma: -80 }), 0)).not.toHaveProperty('drift_ma');
  });

  it('generates at the resolution displayed', () => {
    // world.json is the displayed size's: the form opens at that size.
    expect(driftRequest(meta({ width: 4096, height: 2048 }), 100)).toMatchObject({ width: 4096, height: 2048 });
  });
});
