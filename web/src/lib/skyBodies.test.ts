import { describe, expect, it } from 'vitest';
import type { Moon } from './api/types/Moon';
import { eclipse, joinSkies, MAX_CASTERS, moonDirection, moonSky, planetSky, siblingSky } from './skyBodies';

const moon = (overrides: Partial<Moon> = {}): Moon =>
  ({
    name: 'I',
    origin: 'giant_impact',
    composition: 'rock',
    radius_km: 1737,
    axes_km: [1737, 1737, 1737],
    round: true,
    mass_ratio: 0.0123,
    density: 3.3,
    gravity: 0.17,
    semi_major_axis_km: 384_400,
    semi_major_axis_radii: 60.3,
    eccentricity: 0.05,
    inclination_deg: 0,
    period_days: 27.3,
    tidally_locked: true,
    migration: 'receding',
    angular_diameter_deg: 0.52,
    barycentre_km: 4670,
    seed: 1,
    ...overrides,
  }) as Moon;

describe('sky bodies', () => {
  it('puts the full moon opposite the sun, then turns it with the seasons', () => {
    // Sun at longitude 0: direction (0, 0, 1).
    const full = moonDirection(moon(), 0, 0, 365.25, 0);
    expect(full[2]).toBeCloseTo(-1, 9);
    // Half a month later it is new, towards the sun.
    const fresh = moonDirection(moon(), 0, 13.65 / 365.25, 365.25, 0);
    expect(fresh[2]).toBeCloseTo(1, 6);
  });

  it('keeps the moon of a binary planet over the same point', () => {
    const charon = moon({ migration: 'locked' });
    expect(moonDirection(charon, 0, 0, 90_000, 0)).toEqual([0, 0, -1]);
    expect(moonDirection(charon, 0, 0.37, 90_000, 1.2)).toEqual([0, 0, -1]);
  });

  it('casts the moons shadows on their planet, at their size and distance', () => {
    const { bodies, casters } = planetSky([moon()], 6371, 0, 365.25, 0);
    expect(bodies[0]!.angularDiameterDeg).toBe(0.52);
    expect(Math.hypot(...casters[0]!.position)).toBeCloseTo(384_400 / 6371, 6);
    expect(casters[0]!.radius).toBeCloseTo(1737 / 6371, 6);
    const many = planetSky(Array.from({ length: 6 }, (_, i) => moon({ radius_km: 10 + i })), 6371, 0, 365.25, 0);
    expect(many.casters).toHaveLength(MAX_CASTERS);
    expect(many.casters[0]!.radius).toBeCloseTo(15 / 6371, 9);
  });

  it('shows the planet from its moon, large and towards its long axis', () => {
    const { bodies, casters } = moonSky({ radius_km: 11, semi_major_axis_km: 9376 }, 3390, 'map.png');
    // Mars from Phobos: about 40 degrees across.
    expect(bodies[0]!.angularDiameterDeg).toBeGreaterThan(38);
    expect(bodies[0]!.angularDiameterDeg).toBeLessThan(44);
    expect(bodies[0]!.direction).toEqual([0, 0, -1]);
    expect(casters[0]!.radius).toBeCloseTo(3390 / 11, 6);
  });

  it('darkens the ground in the umbra, less in the penumbra, not elsewhere', () => {
    const sun: [number, number, number] = [0, 0, 1];
    const sunRadius = (0.533 / 2) * (Math.PI / 180);
    // The Moon between the Sun and Earth, over the point (0, 0, 1), near
    // its perigee (at 60 Earth radii its umbra falls just short: annular).
    const caster = { position: [0, 0, 56] as [number, number, number], radius: 0.2727 };
    const annular = eclipse([0, 0, 1], sun, { ...caster, position: [0, 0, 63] }, sunRadius);
    expect(annular).toBeGreaterThan(0.8);
    expect(annular).toBeLessThan(1);
    expect(eclipse([0, 0, 1], sun, caster, sunRadius)).toBeCloseTo(1, 6);
    expect(eclipse([0.3, 0, Math.sqrt(1 - 0.09)], sun, caster, sunRadius)).toBeLessThan(1);
    expect(eclipse([0.9, 0, Math.sqrt(1 - 0.81)], sun, caster, sunRadius)).toBe(0);
    // Behind the planet, the moon casts no shadow on the day side.
    expect(eclipse([0, 0, 1], sun, { ...caster, position: [0, 0, -56] }, sunRadius)).toBe(0);
  });

  it('shows the other moons of the giant beside it, the largest shadows kept', () => {
    // Europa (1 561 km, 671 000 km, 3.55 d) seen from Io (1 822 km, 421 700 km, 1.77 d).
    const europa = { radius_km: 1561, distance_km: 671_000, period_days: 3.55, icy: true };
    const sky = siblingSky([europa], 1822, 421_700, 1.77, 0, 4333);
    const direction = sky.bodies[0]!.direction;
    expect(Math.hypot(...direction)).toBeCloseTo(1, 9);
    // At most the two orbits apart, at least their difference.
    const size = sky.bodies[0]!.angularDiameterDeg;
    const at = (d: number) => (2 * Math.atan(1561 / d) * 180) / Math.PI;
    expect(size).toBeGreaterThanOrEqual(at(671_000 + 421_700) - 1e-9);
    expect(size).toBeLessThanOrEqual(at(671_000 - 421_700) + 1e-9);
    expect(sky.casters[0]!.radius).toBeCloseTo(1561 / 1822, 9);
    // Joined with the giant: every body, and the giant's shadow first.
    const giant = moonSky({ radius_km: 1822, semi_major_axis_km: 421_700 }, 69_911, null);
    const joined = joinSkies(giant, sky);
    expect(joined.bodies).toHaveLength(2);
    expect(joined.casters[0]!.radius).toBeCloseTo(69_911 / 1822, 6);
  });
});

