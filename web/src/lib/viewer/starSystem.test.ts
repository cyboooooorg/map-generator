import { describe, expect, it } from 'vitest';
import { meanOrbitalSpeed, orbitalSpeed, prominences, withOrbitDefaults, keplerPosition, orbitDistance, periapsisAngle, STAR_RADIUS, SUN_SYSTEM, starSurface, systemScale } from './starSystem';

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

  it('reads the stars of worlds made before eccentric orbits as circles', () => {
    const old = { ...SUN_SYSTEM, eccentricity: undefined, periapsis_au: undefined, apoapsis_au: undefined } as unknown as typeof SUN_SYSTEM;
    const star = withOrbitDefaults({ ...old, orbit_au: 0.5 });
    expect([star.eccentricity, star.periapsis_au, star.apoapsis_au]).toEqual([0, 0.5, 0.5]);
  });

  it('gives orbital speeds by the vis-viva equation', () => {
    expect(meanOrbitalSpeed(1, 1)).toBeCloseTo(29.78, 2);
    expect(orbitalSpeed(1, 1, 1)).toBeCloseTo(29.78, 2);
    // Mercury: 59 km/s at perihelion, 39 km/s at aphelion.
    expect(orbitalSpeed(1, 0.387, 0.307)).toBeCloseTo(58.98, 0);
    expect(orbitalSpeed(1, 0.387, 0.467)).toBeCloseTo(38.86, 0);
  });

  it('moves the planet at the vis-viva speed over the year of Kepler\'s third law', () => {
    // Mercury: the year that world.json gives (365.25 × √(a³/M) days), the
    // Kepler motion the view animates, and the speed the panel shows agree.
    const [a, e, mass] = [0.387, 0.206, 1];
    const yearS = 365.25 * Math.sqrt(a ** 3 / mass) * 86_400;
    const AU_KM = 149_597_870.7;
    const at = (m: number) => {
      const { trueAnomaly, distance } = keplerPosition(e, m);
      return [a * distance * Math.cos(trueAnomaly), a * distance * Math.sin(trueAnomaly), a * distance] as const;
    };
    for (const m of [0, 1, 2, Math.PI, 4, 5]) {
      const dm = 1e-4;
      const [x0, y0] = at(m - dm / 2);
      const [x1, y1, r] = [...at(m + dm / 2).slice(0, 2), at(m)[2]] as [number, number, number];
      const kms = (Math.hypot(x1 - x0, y1 - y0) * AU_KM) / ((dm / (2 * Math.PI)) * yearS);
      expect(kms / orbitalSpeed(mass, a, r)).toBeCloseTo(1, 3);
    }
  });

  it('places the planet smoothly however long the animation runs, even on very eccentric orbits', () => {
    for (const e of [0, 0.2, 0.79, 0.85, 0.9]) {
      // A few turns early on, and after a long run (an hour at a turn a minute).
      for (const start of [0, 2000]) {
        let previous = keplerPosition(e, start);
        let largest = 0;
        for (let m = start + 0.01; m < start + 20; m += 0.01) {
          const next = keplerPosition(e, m);
          const step = Math.hypot(
            next.distance * Math.cos(next.trueAnomaly) - previous.distance * Math.cos(previous.trueAnomaly),
            next.distance * Math.sin(next.trueAnomaly) - previous.distance * Math.sin(previous.trueAnomaly),
          );
          largest = Math.max(largest, step);
          previous = next;
        }
        // A hundredth of a radian of mean anomaly never moves the planet far.
        expect(largest).toBeLessThan(0.1);
      }
    }
  });

  it('gives active stars prominences anchored on their surface', () => {
    expect(prominences('A', 1)).toHaveLength(0);
    const arcs = prominences('M', 7);
    expect(arcs).toHaveLength(7);
    for (const arc of arcs) {
      expect(Math.hypot(...arc.from)).toBeCloseTo(1, 5);
      expect(Math.hypot(...arc.to)).toBeCloseTo(1, 5);
      expect(arc.height).toBeGreaterThan(0);
    }
    expect(prominences('M', 7)).toEqual(arcs);
  });
});
