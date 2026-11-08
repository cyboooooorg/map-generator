import { describe, expect, it } from 'vitest';
import type { System } from './api/types/System';
import { clockDays, seasonOfYearDay, yearDayAt, yearDayOfSeason } from './seasonClock';
import fixture from './systemSky.fixture.json';
import { systemSky } from './systemSky';

const base = fixture as unknown as System;
const TILT = 23.4;

/** The fixture with planet b on an orbit of eccentricity `e` and a periapsis and epoch phase of its own. */
function withOrbit(e: number): System {
  return {
    ...base,
    bodies: base.bodies.map((b) =>
      b.id === 'b'
        ? {
            ...b,
            orbit: { ...b.orbit, eccentricity: e, periapsis_deg: 70, mean_anomaly_deg: 200, inclination_deg: 3, node_deg: 40 },
            spin: { ...b.spin, axial_tilt_deg: TILT, locked: false },
          }
        : b,
    ),
  };
}

/** The latitude under the star, degrees, the body's sky gives at season `season`. */
function latitude(system: System, season: number): number {
  const orbit = system.bodies.find((b) => b.id === 'b')!.orbit;
  const sky = systemSky(system, 'b', clockDays(orbit, season), () => null);
  return (Math.asin(sky.sun[1]) * 180) / Math.PI;
}

describe.each([0, 0.3])('season slider of a system planet, e = %s', (e) => {
  const system = withOrbit(e);
  const orbit = system.bodies.find((b) => b.id === 'b')!.orbit;

  it('puts the equinoxes and the solstices at the quarters', () => {
    expect(latitude(system, 0)).toBeCloseTo(0, 5);
    expect(latitude(system, 0.25)).toBeCloseTo(TILT, 5);
    expect(latitude(system, 0.5)).toBeCloseTo(0, 5);
    expect(latitude(system, 0.75)).toBeCloseTo(-TILT, 5);
  });

  it('counts the days of the year from the spring equinox, round trip with the season', () => {
    expect(yearDayOfSeason(orbit, 0)).toBeCloseTo(0, 9);
    for (const s of [0.1, 0.25, 0.6, 0.9]) {
      expect(seasonOfYearDay(orbit, yearDayOfSeason(orbit, s))).toBeCloseTo(s, 8);
    }
    expect(yearDayAt(orbit, clockDays(orbit, 0.25, 0))).toBeCloseTo(yearDayOfSeason(orbit, 0.25), 9);
  });

  it('keeps the seasons equal on a circular orbit and unequal on an eccentric one', () => {
    const gaps = [0, 0.25, 0.5, 0.75].map((s, i, all) => {
      const next = all[i + 1] ?? 1;
      return (next === 1 ? orbit.period_days : yearDayOfSeason(orbit, next)) - yearDayOfSeason(orbit, s);
    });
    expect(gaps.reduce((a, b) => a + b, 0)).toBeCloseTo(orbit.period_days, 6);
    if (e === 0) {
      gaps.forEach((g) => expect(g).toBeCloseTo(orbit.period_days / 4, 6));
    } else {
      expect(Math.max(...gaps) - Math.min(...gaps)).toBeGreaterThan(orbit.period_days * 0.05);
    }
  });
});
