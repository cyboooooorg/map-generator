import { describe, expect, it } from 'vitest';
import { keplerPosition, orbitDistance, periapsisAngle, STAR_RADIUS, SUN_SYSTEM, starSurface, systemScale } from './starSystem';

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

  it('moves the planet at the pace of Kepler on its ellipse', () => {
    // Circular: the true anomaly is the mean anomaly, the distance constant.
    const circle = keplerPosition(0, 1);
    expect(circle.trueAnomaly).toBeCloseTo(1, 6);
    expect(circle.distance).toBeCloseTo(1, 6);
    // Mercury: closest at periapsis (M = 0), farthest at apoapsis (M = π).
    expect(keplerPosition(0.2056, 0).distance).toBeCloseTo(1 - 0.2056, 6);
    expect(keplerPosition(0.2056, Math.PI).distance).toBeCloseTo(1 + 0.2056, 6);
    // Faster near the star: a quarter of the year from periapsis covers more than 90°.
    expect(keplerPosition(0.5, Math.PI / 2).trueAnomaly).toBeGreaterThan(Math.PI / 2);
    // The ellipse and Kepler's solution agree.
    const p = keplerPosition(0.6, 2);
    expect(orbitDistance(0.6, p.trueAnomaly)).toBeCloseTo(p.distance, 5);
    const omega = periapsisAngle(1234);
    expect(omega).toBeGreaterThanOrEqual(0);
    expect(omega).toBeLessThan(2 * Math.PI);
  });
});
