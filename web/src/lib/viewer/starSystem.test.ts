import { describe, expect, it } from 'vitest';
import { orbitGaps, markerSizes, MARKER_MIN, systemLogScale, compressedFrom, giantMarkerSize, systemCompressedFrom, meanOrbitalSpeed, moonOrbits, orbitalSpeed, withOrbitDefaults, keplerPosition, orbitDistance, periapsisAngle, STAR_RADIUS, SUN_SYSTEM, systemScale, velocityScale, VELOCITY_MAX_LENGTH, defaultSystemSpeed, gravityScale, starGravity, GRAVITY_MAX_LENGTH, SYSTEM_SPEED_MAX, frostLineAu, hillRadiusAu, hillShownRadius, realVectorBound, REAL_VECTOR_FLOOR, sphereGrid, PLANET_MARKER, RING_MOON_SIZE, ringBands } from './starSystem';

describe('star view', () => {
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

  it('compresses distances beyond the world only when giants lie there', () => {
    const star = { apoapsis_au: 1.1, hz_outer_au: 1.7 };
    const plain = systemScale(star);
    const far = systemScale(star, [{ orbit_au: 30, eccentricity: 0.1 }]);
    // The world's system keeps its scale.
    for (const au of [0.1, 1, 1.9]) expect(far(au)).toBeCloseTo(plain(au), 10);
    // Beyond, the farthest giant lands 3.5 units further, and order is kept.
    const from = compressedFrom(star);
    expect(far(33) - far(from)).toBeCloseTo(3.5, 5);
    expect(far(5)).toBeLessThan(far(10));
    expect(far(10)).toBeLessThan(plain(10));
    // A close giant leaves the scale linear.
    expect(systemScale(star, [{ orbit_au: 0.05, eccentricity: 0 }])(1)).toBeCloseTo(plain(1), 10);
    expect(giantMarkerSize(11.2)).toBeGreaterThan(giantMarkerSize(3.9));
  });

  it('keeps one scale for a planetary system, whichever planet is selected', () => {
    const planets = [
      { orbit_au: 0.4, eccentricity: 0.1 },
      { orbit_au: 1.2, eccentricity: 0.02 },
      { orbit_au: 30, eccentricity: 0.05 },
    ];
    const hz = { hz_outer_au: 1.7 };
    const from = systemCompressedFrom(hz);
    const inner = systemScale({ ...hz, apoapsis_au: 0.44 }, planets, 6, from);
    const outer = systemScale({ ...hz, apoapsis_au: 31.5 }, planets, 6, from);
    for (const au of [0.1, 0.4, 1, 1.7, 5, 30]) expect(outer(au)).toBeCloseTo(inner(au), 10);
    // Planets all within the zone: linear, with no compressed part.
    const close = systemScale({ ...hz, apoapsis_au: 1 }, planets.slice(0, 2), 6, from);
    expect(close(1) - close(0.5)).toBeCloseTo(close(1.5) - close(1), 10);
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

  it('draws moons around their planet at their pace, binaries swinging', () => {
    const moon = (period_days: number, inclination_deg: number, barycentre_km = 0) => ({
      radius_km: 1737,
      inclination_deg,
      period_days,
      barycentre_km,
      semi_major_axis_km: 384_400,
    });
    const [near, far, back] = moonOrbits([moon(27.3, 5), moon(0.3, 1), moon(10, 150)], 6371, 365.25, 60, false);
    // Nearest first, farther out one after the other.
    expect(near!.radius).toBeLessThan(far!.radius);
    expect(far!.radius).toBeLessThan(back!.radius);
    // The Moon goes round in 27.3 / 365.25 of the animated year; a close
    // moonlet no faster than once in two seconds; a retrograde moon backwards.
    expect(near!.speed).toBeCloseTo((2 * Math.PI) / ((27.3 / 365.25) * 60), 6);
    expect(far!.speed).toBeCloseTo(Math.PI, 6);
    expect(back!.speed).toBeLessThan(0);
    expect(back!.inclination).toBeCloseTo((30 * Math.PI) / 180, 6);
    expect(near!.swing).toBe(0);
    // A binary's planet swings around the barycentre outside it.
    const [charon] = moonOrbits([{ ...moon(6.4, 0, 2100), semi_major_axis_km: 19_600 }], 1188, 90_000, 60, true);
    expect(charon!.swing).toBeCloseTo(2100 / 19_600, 6);
  });

  it('draws a ring moon to scale in the rings, in its gap, and the other moons beyond', () => {
    const R = 60_268;
    const moon = (a_radii: number, ring: boolean) => ({
      radius_km: ring ? 10 : 200,
      inclination_deg: 0,
      period_days: 1,
      barycentre_km: 0,
      semi_major_axis_km: a_radii * R,
      ring,
    });
    const rings = { inner: 1.25, outer: 2.3, gaps: [[2.05, 0.0005] as [number, number], [1.9, 0.06] as [number, number]] };
    const [moonlet, resonant, other] = moonOrbits([moon(2.05, true), moon(3.0, true), moon(4, false)], R, 365, 60, false);
    expect(moonlet!.radius).toBeCloseTo(PLANET_MARKER * 2.05, 9);
    // In the rings' plane (see giantRings' moonSet).
    expect(moonlet!.ring).toBe(true);
    expect(moonlet!.inclination).toBe(0);
    expect(resonant!.radius).toBeCloseTo(PLANET_MARKER * 3.0, 9);
    expect(other!.radius).toBeGreaterThan(resonant!.radius);
    // The moonlet's gap is cut wide enough for its marker.
    const least = RING_MOON_SIZE * 1.6;
    const bands = ringBands(rings, least);
    expect(bands.some(([a, b]) => a < 2.05 + RING_MOON_SIZE && b > 2.05 - RING_MOON_SIZE)).toBe(false);
    expect(bands[0]![0]).toBe(1.25);
    expect(bands.at(-1)![1]).toBe(2.3);
    // Without ring moons the moons keep their places.
    const plain = moonOrbits([moon(4, false)], R, 365, 60, false);
    expect(plain[0]!.radius).toBeCloseTo(PLANET_MARKER * 1.6, 9);
  });

  it('scales every velocity vector alike, the fastest planet at its periapsis the longest', () => {
    const orbits = [
      { orbit_au: 0.4, eccentricity: 0.2 },
      { orbit_au: 1, eccentricity: 0 },
    ];
    const k = velocityScale(1, orbits);
    expect(orbitalSpeed(1, 0.4, 0.4 * 0.8) * k).toBeCloseTo(VELOCITY_MAX_LENGTH, 10);
    expect(orbitalSpeed(1, 1, 1) * k).toBeLessThan(VELOCITY_MAX_LENGTH);
    expect(velocityScale(1, [])).toBe(0);
  });

  it('gives the star\'s pull by Newton, the closest planet\'s vector the longest', () => {
    expect(starGravity(1, 1)).toBeCloseTo(5.93e-3, 6);
    expect(starGravity(1, 2)).toBeCloseTo(5.93e-3 / 4, 8);
    const orbits = [
      { orbit_au: 0.4, eccentricity: 0.2 },
      { orbit_au: 1, eccentricity: 0 },
    ];
    expect(starGravity(1, 0.32) * gravityScale(1, orbits)).toBeCloseTo(GRAVITY_MAX_LENGTH, 10);
  });

  it('opens a system at a speed of a day a second at least, its innermost year in about ten seconds', () => {
    expect(defaultSystemSpeed(1.5)).toBe(0);
    expect(defaultSystemSpeed(88)).toBe(3);
    expect(defaultSystemSpeed(1e9)).toBe(SYSTEM_SPEED_MAX);
  });

  it('puts the frost line at 2.7 AU for the Sun, and Jupiter\'s Hill sphere at about 0.34 AU', () => {
    expect(frostLineAu(1)).toBeCloseTo(2.7, 10);
    expect(frostLineAu(4)).toBeCloseTo(5.4, 10);
    expect(hillRadiusAu(5.2, 0.049, 317.8, 1)).toBeCloseTo(0.338, 2);
    expect(hillRadiusAu(1, 0.0167, 1, 1)).toBeCloseTo(0.0098, 3);
  });

  it('draws a Hill sphere outside the moons it holds, dashed when that is not to scale', () => {
    expect(hillShownRadius(2, 0.1, 0.5)).toEqual({ radius: 2, toScale: true });
    // Smaller than the outermost moon's ring (drawn larger than to scale).
    const shown = hillShownRadius(0.3, 0.1, 0.5);
    expect(shown.toScale).toBe(false);
    expect(shown.radius).toBeGreaterThan(0.5);
    // No moons: just outside the marker.
    const bare = hillShownRadius(0.01, 0.1, 0);
    expect(bare.toScale).toBe(false);
    expect(bare.radius).toBeCloseTo(0.16, 10);
  });

  it('draws a Hill sphere to scale and solid in the true-scale view, whatever the marker and the moons', () => {
    // Under the schematic marker and the moons' reach: still its true radius.
    expect(hillShownRadius(0.001, 0.1, 0.5, true)).toEqual({ radius: 0.001, toScale: true });
    expect(hillShownRadius(0.001, 0.1, 0, true)).toEqual({ radius: 0.001, toScale: true });
    // The schematic default keeps its floor.
    expect(hillShownRadius(0.001, 0.1, 0.5).toScale).toBe(false);
  });

  it('bounds the true-scale vectors by half the narrowest gap between the star and the drawn orbits', () => {
    // Orbits drawn at 1, 1.5 and 4 view units: the narrowest gap is 0.5.
    expect(realVectorBound([4, 1, 1.5])).toBeCloseTo(0.25, 10);
    // One planet: the gap to the star.
    expect(realVectorBound([0.8])).toBeCloseTo(0.4, 10);
    // Never longer than the schematic's, never invisible.
    expect(realVectorBound([50])).toBe(VELOCITY_MAX_LENGTH);
    expect(realVectorBound([1, 1.0001])).toBe(REAL_VECTOR_FLOOR);
    expect(realVectorBound([])).toBe(VELOCITY_MAX_LENGTH);
  });

  it('makes true-scale velocity vectors longer the faster the planet goes, the longest at the bound', () => {
    const orbits = [
      { orbit_au: 0.4, eccentricity: 0 },
      { orbit_au: 1, eccentricity: 0.0167 },
      { orbit_au: 5.2, eccentricity: 0.05 },
      { orbit_au: 30, eccentricity: 0.01 },
    ];
    const bound = realVectorBound(orbits.map((o) => (9 * o.orbit_au) / 30));
    const k = velocityScale(1, orbits, bound);
    const lengths = orbits.map((o) => orbitalSpeed(1, o.orbit_au, o.orbit_au * (1 - o.eccentricity)) * k);
    for (let i = 1; i < lengths.length; i++) expect(lengths[i]!).toBeLessThan(lengths[i - 1]!);
    expect(Math.max(...lengths)).toBeCloseTo(bound, 10);
    // Same for the star's pull: the closest is the strongest and longest.
    const g = gravityScale(1, orbits, bound);
    const pulls = orbits.map((o) => starGravity(1, o.orbit_au * (1 - o.eccentricity)) * g);
    for (let i = 1; i < pulls.length; i++) expect(pulls[i]!).toBeLessThan(pulls[i - 1]!);
    expect(Math.max(...pulls)).toBeCloseTo(bound, 10);
  });

  it('builds a sphere grid whose points all lie on the sphere', () => {
    const grid = sphereGrid(2, 16);
    expect(grid.length).toBe(11 * 16 * 2);
    for (const [x, y, z] of grid) expect(Math.hypot(x, y, z)).toBeCloseTo(2, 10);
  });
});

const gapsOf = (at: (au: number) => number, planets: { orbit_au: number; eccentricity: number }[]) =>
  orbitGaps(planets.map((p) => ({ near: at(p.orbit_au * (1 - p.eccentricity)), far: at(p.orbit_au * (1 + p.eccentricity)) })));

describe('planetary system scale', () => {
  // Semi-major axes (AU) and eccentricities of two crowded systems.
  const compact = {
    star: { hz_inner_au: 1.065, hz_outer_au: 1.873 },
    planets: [
      { orbit_au: 0.06957, eccentricity: 0.0638 },
      { orbit_au: 0.08808, eccentricity: 0.0579 },
      { orbit_au: 0.15003, eccentricity: 0.052 },
    ],
  };
  const giants = {
    star: { hz_inner_au: 0.271, hz_outer_au: 0.517 },
    planets: [
      [0.0383, 0],
      [0.0552, 0.0072],
      [2.266, 0.1068],
      [3.832, 0.1218],
      [6.142, 0.0637],
      [9.263, 0.0154],
      [13.831, 0.0337],
    ].map(([orbit_au, eccentricity]) => ({ orbit_au: orbit_au!, eccentricity: eccentricity! })),
  };

  it('is continuous and increasing, from the star to beyond the farthest orbit', () => {
    for (const { star, planets } of [compact, giants]) {
      const at = systemLogScale(star, planets);
      let last = at(0);
      expect(last).toBe(0);
      for (let au = 0.001; au < 40; au *= 1.01) {
        const r = at(au);
        expect(r).toBeGreaterThan(last);
        // No jump: a 1% step in distance moves at most a few hundredths.
        expect(r - last).toBeLessThan(0.1);
        last = r;
      }
    }
  });

  it('keeps the innermost orbit off the star and the whole system within reach of the camera', () => {
    for (const { star, planets } of [compact, giants]) {
      const at = systemLogScale(star, planets);
      const inner = Math.min(...planets.map((p) => p.orbit_au * (1 - p.eccentricity)));
      expect(at(inner)).toBeGreaterThan(STAR_RADIUS * 1.6);
      expect(at(Math.max(...planets.map((p) => p.orbit_au * (1 + p.eccentricity))))).toBeLessThan(18);
    }
  });

  it('spreads a compact inner trio and gives outer giants room', () => {
    const at = systemLogScale(compact.star, compact.planets);
    const [b, c, d] = compact.planets.map((p) => at(p.orbit_au));
    expect(c! - b!).toBeGreaterThan(0.4);
    expect(d! - c!).toBeGreaterThan(c! - b!);
    const far = systemLogScale(giants.star, giants.planets);
    const places = giants.planets.map((p) => far(p.orbit_au));
    const sizes = markerSizes(
      giants.planets.map((p) => giantMarkerSize(p.orbit_au > 1 ? 4 : 1)),
      gapsOf(far, giants.planets),
    );
    for (let k = 3; k < places.length; k++) {
      // At least three marker diameters between neighbouring giants.
      expect(places[k]! - places[k - 1]!).toBeGreaterThan(3 * 2 * sizes[k]!);
    }
  });

  it('keeps the habitable zone in order and in its place', () => {
    const at = systemLogScale(giants.star, giants.planets);
    expect(at(giants.star.hz_inner_au)).toBeLessThan(at(giants.star.hz_outer_au));
    expect(at(giants.star.hz_outer_au)).toBeLessThan(at(giants.planets[2]!.orbit_au));
  });

  it('draws markers that never overlap, nor the star', () => {
    for (const { star, planets } of [compact, giants]) {
      const at = systemLogScale(star, planets);
      const places = planets.map((p) => at(p.orbit_au));
      const gaps = gapsOf(at, planets);
      const sizes = markerSizes(
        planets.map(() => giantMarkerSize(11)),
        gaps,
        planets.map(() => 2.75),
      );
      sizes.forEach((size, k) => {
        // Everything drawn round the planet (rings and halo, 2.75 radii) stays between the orbits,
        // unless the minimum size forces it (never so in these systems).
        expect(size * 2.75).toBeLessThanOrEqual(0.5 * gaps[k]! + 1e-9);
        expect(size).toBeGreaterThanOrEqual(MARKER_MIN);
        expect(size).toBeLessThanOrEqual(giantMarkerSize(11));
        expect(places[k]! - size).toBeGreaterThan(STAR_RADIUS);
        if (k > 0) expect(places[k]! - places[k - 1]!).toBeGreaterThan(size + sizes[k - 1]!);
      });
    }
  });

  it('measures the room between orbit curves, eccentricity included', () => {
    // Two orbits 1 apart as circles; the inner one's apoapsis reaches 0.3 further out.
    expect(orbitGaps([{ near: 3, far: 3 }, { near: 4, far: 4 }])[0]).toBeCloseTo(1);
    expect(orbitGaps([{ near: 3, far: 3.3 }, { near: 4, far: 4 }])[1]).toBeCloseTo(0.7);
    // The innermost is limited by the star's surface (radius 1).
    expect(orbitGaps([{ near: 1.6, far: 1.6 }, { near: 5, far: 5 }])[0]).toBeCloseTo(0.6);
    // Crossing orbits leave no room.
    expect(orbitGaps([{ near: 3, far: 4 }, { near: 3.5, far: 5 }])).toEqual([0, 0]);
  });

  it('shrinks a marker to 0.4 of the room, what it draws round it to half, and no further than the minimum', () => {
    expect(markerSizes([0.2, 0.2], [0.25, 0.25])).toEqual([0.1, 0.1]);
    expect(markerSizes([0.2], [1], [4])[0]).toBeCloseTo(0.125);
    expect(markerSizes([0.2, 0.2], [0.001, 0.001])).toEqual([MARKER_MIN, MARKER_MIN]);
    expect(markerSizes([0.11, 0.11], [3, 6])).toEqual([0.11, 0.11]);
  });
});
