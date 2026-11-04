import { describe, expect, it } from 'vitest';
import { STAR_RADIUS, SUN_SYSTEM, starSurface, systemScale } from './starSystem';

describe('star view', () => {
  it('gives cool stars spots, flares and darker limbs, hot stars a smooth surface', () => {
    const red = starSurface(3100, 'M');
    const sun = starSurface(5772, 'G');
    const hot = starSurface(9700, 'A');
    expect(red.spots).toBeGreaterThan(sun.spots);
    expect(red.flares).toBe(1);
    expect(red.limb).toBeGreaterThan(sun.limb);
    expect(hot.granulation).toBe(0);
    expect(hot.spots).toBe(0);
  });

  it('lays the system out outside the star, in proportion', () => {
    const at = systemScale(SUN_SYSTEM);
    expect(at(0)).toBeGreaterThan(STAR_RADIUS);
    const inner = at(SUN_SYSTEM.hz_inner_au);
    const orbit = at(SUN_SYSTEM.orbit_au);
    const outer = at(SUN_SYSTEM.hz_outer_au);
    expect(inner).toBeLessThan(orbit);
    expect(orbit).toBeLessThan(outer);
    // Linear in AU beyond the offset.
    expect((orbit - at(0)) / (outer - at(0))).toBeCloseTo(SUN_SYSTEM.orbit_au / SUN_SYSTEM.hz_outer_au, 5);
  });
});
