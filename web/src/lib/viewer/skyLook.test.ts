import { describe, expect, it } from 'vitest';
import type { GalacticPlace } from '../api/types/GalacticPlace';
import { FIXED_SKY, skyLook } from './skyLook';

function place(position: [number, number, number], seed = 5): GalacticPlace {
  return {
    source: 'drawn',
    position_kpc: position,
    axes: [
      [0, 0, 1],
      [1, 0, 0],
      [0, 1, 0],
    ],
    sky_seed: seed,
  };
}

const norm = (v: readonly number[]) => Math.hypot(v[0]!, v[1]!, v[2]!);

describe('skyLook', () => {
  it('is the fixed sky without a place', () => {
    expect(skyLook(null)).toBe(FIXED_SKY);
    expect(FIXED_SKY.seed).toBe(7);
    expect(FIXED_SKY.brightness).toBe(1);
  });

  it("is unit brightness and a 7 degree band at the Sun's place", () => {
    const look = skyLook(place([8.2, 0, 0.02]));
    expect(look.brightness).toBeCloseTo(1, 9);
    expect(look.bandWidth).toBeCloseTo((7 * Math.PI) / 180, 9);
    expect(look.seed).toBe(5);
  });

  it('is brighter and more bulged inwards', () => {
    const inner = skyLook(place([2, 0, 0]));
    const outer = skyLook(place([16, 0, 0]));
    expect(inner.brightness).toBeGreaterThan(outer.brightness);
    expect(inner.bulge).toBeGreaterThan(outer.bulge);
  });

  it('narrows the band away from the plane', () => {
    expect(skyLook(place([8.2, 0, 1.5])).bandWidth).toBeLessThan(skyLook(place([8.2, 0, 0])).bandWidth);
  });

  it('keeps unit axes, finite numbers and a bulge in [0, 1]', () => {
    for (const p of [place([2, 0, 0]), place([16, 0, 0]), place([8.2, 0, 1.5]), place([0, 8, -1.5])]) {
      const look = skyLook(p);
      expect(norm(look.north)).toBeCloseTo(1, 9);
      expect(norm(look.centre)).toBeCloseTo(1, 9);
      for (const v of [look.brightness, look.bandWidth, look.bulge, ...look.north, ...look.centre]) {
        expect(Number.isFinite(v)).toBe(true);
      }
      expect(look.bulge).toBeGreaterThanOrEqual(0);
      expect(look.bulge).toBeLessThanOrEqual(1);
    }
  });

  it('maps galactic directions through the axes', () => {
    const look = skyLook(place([8, 0, 0]));
    expect(look.north).toEqual([0, 1, 0]);
    // Centre is galactic -x; axes[0] = system z.
    expect(look.centre[2]).toBeCloseTo(-1, 9);
  });
});
