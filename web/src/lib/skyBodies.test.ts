import { describe, expect, it } from 'vitest';
import type { Moon } from './api/types/Moon';
import type { WorldMetadata } from './api/types/WorldMetadata';
import { skyFromWorld } from './sky';
import {
  eclipse,
  halo,
  joinSkies,
  MAX_CASTERS,
  moonDirection,
  moonPlace,
  moonSky,
  moonSpinHours,
  moonSystemSky,
  orbitLoops,
  planetSky,
  siblingOrbitLoops,
  siblingSky,
  sunLongitudeOnMoon,
  sunOnMoon,
  type SkyBody,
  type Vec3,
} from './skyBodies';

const dot = (a: Vec3, b: Vec3) => a[0] * b[0] + a[1] * b[1] + a[2] * b[2];

/** Longitude (degrees) of a sky body's map facing the body shown. */
const face = (body: SkyBody): number => {
  const [x, , z] = body.axes ?? [
    [1, 0, 0],
    [0, 1, 0],
    [0, 0, 1],
  ];
  const back = body.direction.map((c) => -c) as Vec3;
  return (Math.atan2(dot(back, x), dot(back, z)) * 180) / Math.PI;
};

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
    // Circular unless a test says otherwise (see the Kepler test).
    eccentricity: 0,
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

  it('moves an eccentric moon on its ellipse, nearer and faster at periapsis', () => {
    const eccentric = moon({ eccentricity: 0.3, seed: 0 });
    // Seed 0: the node and the periapsis at longitude 0, where the mean
    // longitude is half an orbit after the season starts (the sun at
    // longitude 0); over so long a year that the planet's frame does not
    // turn against the stars meanwhile.
    const year = 1e9;
    const at = (days: number) => moonPlace(eccentric, 0, days / year, year, 0);
    const peri = at(27.3 / 2);
    const apo = at(0);
    expect(peri.distanceKm).toBeCloseTo(384_400 * 0.7, 3);
    expect(apo.distanceKm).toBeCloseTo(384_400 * 1.3, 3);
    expect(peri.direction[2]).toBeCloseTo(1, 9);
    // A tenth of a day either side, it sweeps a wider angle near periapsis (Kepler's second law).
    const angle = (a: number[], b: number[]) => Math.acos(Math.min(1, a.reduce((sum, c, i) => sum + c * b[i]!, 0)));
    const nearPeri = angle(at(27.3 / 2 - 0.1).direction, at(27.3 / 2 + 0.1).direction);
    const nearApo = angle(at(-0.1).direction, at(0.1).direction);
    expect(nearPeri / nearApo).toBeCloseTo((1.3 / 0.7) ** 2, 1);
    // Larger in the sky and its orbit line nearer there.
    const sky = (days: number) => planetSky([eccentric], 6371, days / year, year, 0).bodies[0]!;
    expect(sky(27.3 / 2).angularDiameterDeg / sky(0).angularDiameterDeg).toBeCloseTo(1.3 / 0.7, 6);
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

  it('maps each moon in its planet sky, its far side away', () => {
    const moons = [moon(), moon({ name: 'II', composition: 'ice', semi_major_axis_km: 600_000, period_days: 52 })];
    const { bodies } = planetSky(moons, 6371, 0, 365.25, 0, (k) => `moons/${k + 1}.png`);
    expect(bodies.map((b) => b.texture)).toEqual(['moons/1.png', 'moons/2.png']);
    expect(bodies.every((b) => Math.abs(Math.abs(face(b)) - 180) < 1e-9)).toBe(true);
  });

  it('shows the planet and the other moons from a moon, where they are', () => {
    const inner = moon({ name: 'I', semi_major_axis_km: 200_000, period_days: 10, radius_km: 1000 });
    const outer = moon({ name: 'II', semi_major_axis_km: 400_000, period_days: 28, radius_km: 2000 });
    const textures = (k: number) => `moons/${k + 1}.png`;
    const sky = moonSystemSky([inner, outer], 0, 6371, 'planet.png', textures, 0, 365.25, 0);
    expect(sky.bodies).toHaveLength(2);
    const [planet, other] = sky.bodies;
    planet!.direction.forEach((c, i) => expect(c).toBeCloseTo([0, 0, -1][i]!, 9));
    expect(planet!.texture).toBe('planet.png');
    expect(other!.texture).toBe('moons/2.png');
    // The outer moon where planetSky puts it, seen from the inner one.
    const d0 = moonDirection(inner, 0, 0, 365.25, 0);
    const d1 = moonDirection(outer, 1, 0, 365.25, 0);
    const v = d1.map((c, i) => 400_000 * c - 200_000 * d0[i]!);
    const distance = Math.hypot(...v);
    expect(other!.direction[2]).toBeCloseTo(v.reduce((sum, c, i) => sum + c * d0[i]!, 0) / distance, 6);
    expect(other!.angularDiameterDeg).toBeCloseTo((2 * Math.atan(2000 / distance) * 180) / Math.PI, 6);
    // The planet's shadow first, then the other moon's.
    expect(sky.casters[0]!.radius).toBeCloseTo(6371 / 1000, 6);
    expect(Math.hypot(...sky.casters[1]!.position)).toBeCloseTo(distance / 1000, 6);
  });

  it('shows each other the faces two lined-up moons turn to each other', () => {
    // Two moons hanging over the same point of their planet (as a binary's).
    const inner = moon({ migration: 'locked', semi_major_axis_km: 200_000 });
    const outer = moon({ migration: 'locked', semi_major_axis_km: 400_000 });
    const none = () => null;
    // From the inner one, the outer is straight up, its planet side down.
    const up = moonSystemSky([inner, outer], 0, 6371, null, none, 0, 365.25, 0).bodies[1]!;
    expect(up.direction[2]).toBeCloseTo(1, 9);
    expect(Math.abs(face(up))).toBeCloseTo(180, 9);
    // From the outer one, the inner is in front of the planet, its far side out.
    const down = moonSystemSky([inner, outer], 1, 6371, null, none, 0, 365.25, 0).bodies[1]!;
    expect(down.direction[2]).toBeCloseTo(-1, 9);
    expect(face(down)).toBeCloseTo(0, 9);
  });

  it('turns the sun round a moon once an orbit, over its planet side when full', () => {
    // Full at the season's start: the sun over the face towards the planet.
    expect(sunLongitudeOnMoon(moon(), 0, 0, 365.25, 0)).toBeCloseTo(180, 6);
    // A quarter of an orbit on, a quarter of a turn of the sun.
    const quarter = sunLongitudeOnMoon(moon(), 0, 27.3 / 4 / 365.25, 365.25, 0);
    expect(Math.abs(quarter - 180)).toBeCloseTo(90, 4);
    // New moon: the sun over the far side.
    const fresh = sunLongitudeOnMoon(moon(), 0, 27.3 / 2 / 365.25, 365.25, 0);
    expect(Math.cos((fresh * Math.PI) / 180)).toBeCloseTo(1, 6);
  });

  it('draws each moving moon orbit, from the planet or from another moon', () => {
    const inner = moon({ semi_major_axis_km: 200_000, inclination_deg: 0 });
    const outer = moon({ semi_major_axis_km: 400_000, inclination_deg: 30 });
    const binary = moon({ migration: 'locked' });
    // From the planet: great circles, in their planes; none for the binary.
    const fromPlanet = orbitLoops([inner, outer, binary], null, 0, 365.25, 0);
    expect(fromPlanet).toHaveLength(2);
    expect(fromPlanet[0]!.every((d) => Math.abs(d[1]) < 1e-9 && Math.abs(Math.hypot(...d) - 1) < 1e-9)).toBe(true);
    expect(Math.max(...fromPlanet[1]!.map((d) => d[1]))).toBeCloseTo(Math.sin(Math.PI / 6), 2);
    // From the inner moon: its own orbit, a great circle through its
    // planet; the outer one's goes round its sky, through the zenith of its
    // far side and its planet side.
    const level = moon({ semi_major_axis_km: 400_000 });
    const fromMoon = orbitLoops([inner, level], 0, 0, 365.25, 0);
    expect(fromMoon).toHaveLength(2);
    const [own, other] = fromMoon;
    expect(own!.every((d) => Math.abs(d[1]) < 1e-9 && Math.abs(Math.hypot(...d) - 1) < 1e-9)).toBe(true);
    expect(Math.min(...own!.map((d) => d[2]))).toBeCloseTo(-1, 9);
    expect(other!.every((d) => Math.abs(Math.hypot(...d) - 1) < 1e-9)).toBe(true);
    // A loop of the outer orbit seen from inside it spans the whole sky.
    expect(Math.max(...other!.map((d) => d[2]))).toBeGreaterThan(0.9);
    expect(Math.min(...other!.map((d) => d[2]))).toBeLessThan(-0.9);
    // A binary's moon sees its own orbit too.
    expect(orbitLoops([binary], 0, 0, 365.25, 0)).toHaveLength(1);
  });

  it("puts the sun over a moon's latitude from its planet's season and its orbit", () => {
    // In the planet's equator: the planet's tropic at the solstice.
    const level = sunOnMoon(moon(), 0, 0.25, 365.25, 0, 23.44);
    expect(level.latitudeDeg).toBeCloseTo(23.44, 6);
    // Round its planet's year, an inclined moon of an upright planet sees
    // the sun climb and fall by its inclination over its equator, its
    // orbit's node fixed among the stars.
    const tilted = moon({ inclination_deg: 10 });
    const latitudes = Array.from({ length: 48 }, (_, i) => sunOnMoon(tilted, 0, i / 48, 365.25, 0, 0).latitudeDeg);
    expect(Math.max(...latitudes)).toBeCloseTo(10, 0);
    expect(Math.min(...latitudes)).toBeCloseTo(-10, 0);
  });

  it('turns the planet on itself in the sky of its moon', () => {
    const inner = moon({ semi_major_axis_km: 200_000, period_days: 10 });
    const none = () => null;
    const at = (turn: number) =>
      moonSystemSky([inner], 0, 6371, 'planet.png', none, 0, 365.25, 0, {}, turn).bodies[0]!;
    // Full at the season's start: the planet's longitude 180° under it.
    expect(Math.abs(face(at(0)))).toBeCloseTo(180, 9);
    // A quarter of a turn of the planet later, 90° further west.
    expect(face(at(Math.PI / 2))).toBeCloseTo(90, 9);
  });

  it("tilts the planet's axis in the sky of an inclined moon", () => {
    const tilted = moon({ semi_major_axis_km: 200_000, inclination_deg: 30 });
    const planet = moonSystemSky([tilted], 0, 6371, null, () => null, 0, 365.25, 0).bodies[0]!;
    const axis = planet.axes![1];
    // The planet's spin axis, 30° from the moon's (its orbit's normal).
    expect(axis[1]).toBeCloseTo(Math.cos(Math.PI / 6), 9);
  });

  it('spins a captured moon on itself, its planet rising and setting', () => {
    expect(moonSpinHours(moon())).toBeNull();
    expect(moonSpinHours(moon({ migration: 'locked', tidally_locked: false }))).toBeNull();
    const captured = moon({ origin: 'captured', tidally_locked: false, seed: 4242 });
    const spin = moonSpinHours(captured)!;
    expect(spin).toBeGreaterThanOrEqual(6);
    expect(spin).toBeLessThanOrEqual(24);
    // Half a day of its own later, the sun over the opposite longitude
    // (the orbit moving it a little).
    const noon = sunLongitudeOnMoon(captured, 0, 0, 365.25, 0);
    const later = sunLongitudeOnMoon(captured, 0, spin / 2 / 24 / 365.25, 365.25, 0);
    const turned = (((later - noon) % 360) + 360) % 360;
    expect(Math.abs(turned - 180)).toBeLessThan(15);
    // Its face towards its planet changes with it.
    const faceAt = (season: number) => face(planetSky([captured], 6371, season, 365.25, 0).bodies[0]!);
    expect(Math.abs(faceAt(0) - faceAt(spin / 4 / 24 / 365.25))).toBeGreaterThan(45);
  });

  it("circles a giant's other moons round it", () => {
    const [loop] = siblingOrbitLoops([{ distance_km: 100 }], 1000);
    // Seen from 1000 km off, a 100 km circle spans about ±5.7° round the giant.
    const spread = Math.max(...loop!.map((d) => Math.atan2(d[0], -d[2])));
    expect(spread).toBeCloseTo(Math.asin(0.1), 3);
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


describe('halo', () => {
  const world = (pressure: number, atmosphere?: object) =>
    ({ pressure_bar: pressure, preset: { atmosphere } }) as unknown as WorldMetadata;

  it('gives an airless planet no halo', () => {
    expect(halo(skyFromWorld(world(0, { haze: 0 })))).toBeNull();
  });

  it("gives Earth's air a blue halo, a thin one", () => {
    const earth = halo(skyFromWorld(world(1)))!;
    expect(earth.colour[2]).toBe(1);
    expect(earth.colour[0]).toBeLessThan(0.7);
    expect(earth.thickness).toBeGreaterThan(0.02);
    expect(earth.thickness).toBeLessThan(0.06);
    expect(earth.strength).toBeGreaterThan(0.1);
    expect(earth.strength).toBeLessThanOrEqual(1);
  });

  it('brightens the halo of a denser air', () => {
    const thin = halo(skyFromWorld(world(0.006)))!;
    const dense = halo(skyFromWorld(world(92)))!;
    expect(dense.strength).toBeGreaterThan(thin.strength);
  });
});
