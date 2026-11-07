import { describe, expect, it } from 'vitest';
import type { System } from './api/types/System';
import fixture from './systemSky.fixture.json';
import { libration, placeSystem, smallBodiesSky, systemReach, systemSky } from './systemSky';
import type { Vec3 } from './skyBodies';

// A G2V system drawn by the generator: b (terran, generated), c (a gas
// giant) and its two regular moons c-1 and c-2, locked to it.
const system = fixture as unknown as System;
const textures = (id: string) => `${id}.png`;
const length = (v: Vec3) => Math.hypot(...v);

describe('system sky', () => {
  it('places the star where the sun is, at its distance, with its radius', () => {
    const sky = systemSky(system, 'b', 10, textures);
    const b = system.bodies.find((x) => x.id === 'b')!;
    const far = Math.hypot(...placeSystem(system, 10).get('b')!.centre) / b.physical.radius_km;
    expect(length(sky.star.position)).toBeCloseTo(far, 3);
    sky.star.position.forEach((v, i) => expect(v / far).toBeCloseTo(sky.sun[i]!, 9));
    expect(sky.star.radius).toBeCloseTo((system.star.info.radius * 695_700) / b.physical.radius_km, 6);
  });

  it('reaches out to the furthest moon of the planet, from it or from a moon', () => {
    const c = system.bodies.find((x) => x.id === 'c')!;
    const moon = system.bodies.find((x) => x.id === 'c-1')!;
    const apo = (id: string) => {
      const o = system.bodies.find((x) => x.id === id)!.orbit;
      return o.semi_major_axis_km * (1 + o.eccentricity);
    };
    const furthest = Math.max(apo('c-1'), apo('c-2'));
    expect(systemReach(system, 'c')).toBeCloseTo((3 * furthest) / c.physical.radius_km, 6);
    expect(systemReach(system, 'c-1')).toBeCloseTo((3 * (apo('c-1') + apo('c-2'))) / moon.physical.radius_km, 6);
    // A planet with no moons has nothing round it to see.
    expect(systemReach(system, 'b')).toBeNull();
  });

  it('places the other bodies from the body shown, in its radii', () => {
    const sky = systemSky(system, 'b', 10, textures);
    expect(sky.ids).toEqual(['c', 'c-1', 'c-2']);
    const places = placeSystem(system, 10);
    const b = places.get('b')!;
    const c = places.get('c')!;
    const radius = system.bodies.find((x) => x.id === 'b')!.physical.radius_km;
    const far = Math.hypot(...c.centre.map((v, i) => v - b.centre[i]!)) / radius;
    expect(length(sky.bodies[0]!.position!)).toBeCloseTo(far, 3);
    expect(length(sky.sun)).toBeCloseTo(1, 9);
    // Not generated: drawn from its preview, no map.
    expect(sky.bodies[0]!.texture).toBeNull();
    expect(sky.bodies[0]!.preview).toBeDefined();
  });

  it('keeps a moon’s orbit fixed against the stars while its planet turns', () => {
    const day = system.bodies.find((x) => x.id === 'c')!.spin.period_hours / 24;
    const period = system.bodies.find((x) => x.id === 'c-1')!.orbit.period_days;
    const from = (days: number) => {
      const p = placeSystem(system, days);
      return [0, 1, 2].map((i) => p.get('c-1')!.centre[i]! - p.get('c')!.centre[i]!) as Vec3;
    };
    const a = from(0);
    const b = from(day / 4);
    const angle = Math.acos((a[0] * b[0] + a[1] * b[1] + a[2] * b[2]) / (length(a) * length(b))) / (Math.PI / 180);
    // Only its own orbit moves it: about 15.6°, not the 106° it moved when its plane turned with c
    // (within 1.5°: its orbit is slightly eccentric).
    expect(Math.abs(angle - (360 * (day / 4)) / period)).toBeLessThan(1.5);
  });

  it('lays a moon’s orbit in its planet’s tilted equator, still against the stars', () => {
    const normal = (days: number) => placeSystem(system, days).get('c-1')!.plane.normal;
    const axis = placeSystem(system, 0).get('c')!.axes[1];
    // c-1 is inclined 0.15° to c's equator, which c's 7.8° tilt leans.
    expect(normal(0)[0] * axis[0] + normal(0)[1] * axis[1] + normal(0)[2] * axis[2]).toBeGreaterThan(Math.cos((0.2 * Math.PI) / 180));
    normal(0).forEach((v, i) => expect(v).toBeCloseTo(normal(0.3)[i]!, 9));
  });

  it('brings the sun round a locked moon once per orbit', () => {
    const period = system.bodies.find((x) => x.id === 'c-1')!.orbit.period_days;
    const lon = (days: number) => {
      const s = systemSky(system, 'c-1', days, textures).sun;
      return Math.atan2(s[0], s[2]);
    };
    let turned = (lon(period / 4) - lon(0)) / (Math.PI / 180);
    turned = ((turned % 360) + 540) % 360 - 180;
    // A quarter turn in a quarter orbit (within 2°: eccentricity and the planet's year).
    expect(Math.abs(Math.abs(turned) - 90)).toBeLessThan(2);
  });

  it('keeps a locked moon facing its giant, which it sees on its own orbit', () => {
    const sky = systemSky(system, 'c-1', 3, textures);
    const giant = sky.bodies[sky.ids.indexOf('c')]!;
    expect(Math.acos(-giant.direction[2]!)).toBeLessThan(2.5 * 0.0085);
    // The planets' orbits and the giant's moons'; its own passes through it.
    expect(sky.orbits).toHaveLength(4);
    const own = sky.orbits[2]!;
    expect(Math.min(...own.map(length))).toBeLessThan(1);
  });

  it('turns a locked body steadily, its host swinging by the libration', () => {
    const eccentric = structuredClone(system);
    const moon = eccentric.bodies.find((x) => x.id === 'c-1')!;
    moon.orbit.eccentricity = 0.2;
    const period = moon.orbit.period_days;
    for (const days of [0, period / 8, period / 3]) {
      const sky = systemSky(eccentric, 'c-1', days, textures);
      const d = sky.bodies[sky.ids.indexOf('c')]!.direction;
      // The giant's longitude from the face turned to it (-z), in the sense of the orbit.
      const swing = Math.atan2(-d[0], -d[2]);
      expect(Math.abs(swing)).toBeCloseTo(Math.abs(libration(moon.orbit, days)), 6);
      expect(Math.abs(swing)).toBeLessThan(2.5 * 0.2);
    }
  });

  it('faces its host exactly on a circular orbit, and at periapsis', () => {
    const giantFrom = (s: System, days: number) => { const sky = systemSky(s, 'c-1', days, textures); return sky.bodies[sky.ids.indexOf('c')]!.direction; };
    const circular = structuredClone(system);
    circular.bodies.find((x) => x.id === 'c-1')!.orbit.eccentricity = 0;
    giantFrom(circular, 1.3).forEach((v, i) => expect(v).toBeCloseTo([0, 0, -1][i]!, 6));
    const eccentric = structuredClone(system);
    const orbit = eccentric.bodies.find((x) => x.id === 'c-1')!.orbit;
    orbit.eccentricity = 0.2;
    // The first periapsis after the epoch: mean anomaly 0.
    const periapsis = (((360 - orbit.mean_anomaly_deg) % 360) / 360) * orbit.period_days;
    expect(libration(orbit, periapsis)).toBeCloseTo(0, 6);
    giantFrom(eccentric, periapsis).forEach((v, i) => expect(v).toBeCloseTo([0, 0, -1][i]!, 4));
  });

  it('gives the libration its sign: ahead after periapsis, behind before it', () => {
    const eccentric = structuredClone(system);
    const orbit = eccentric.bodies.find((x) => x.id === 'c-1')!.orbit;
    orbit.eccentricity = 0.2;
    const periapsis = (((360 - orbit.mean_anomaly_deg) % 360) / 360) * orbit.period_days;
    expect(libration(orbit, periapsis + orbit.period_days / 8)).toBeGreaterThan(0);
    expect(libration(orbit, periapsis + (5 * orbit.period_days) / 8)).toBeLessThan(0);
  });

  it('librates a retrograde moon and a planet locked to its star too', () => {
    const odd = structuredClone(system);
    odd.bodies.find((x) => x.id === 'c-2')!.orbit.inclination_deg = 150;
    odd.bodies.find((x) => x.id === 'c-2')!.orbit.eccentricity = 0.1;
    const b = odd.bodies.find((x) => x.id === 'b')!;
    b.spin.locked = true;
    // Untilted, so that the host's longitude is its angle in the orbit's plane.
    b.spin.axial_tilt_deg = 0;
    for (const [id, host] of [['c-2', 'c'], ['b', null]] as const) {
      const body = odd.bodies.find((x) => x.id === id)!;
      const days = body.orbit.period_days / 5;
      const sky = systemSky(odd, id, days, textures);
      const d = host ? sky.bodies[sky.ids.indexOf(host)]!.direction : sky.sun;
      expect(Math.abs(Math.atan2(-d[0], -d[2]))).toBeCloseTo(Math.abs(libration(body.orbit, days)), 6);
    }
  });

  it('turns a body on itself at its own pace', () => {
    const day = system.bodies.find((x) => x.id === 'b')!.spin.period_hours / 24;
    const at = (days: number) => placeSystem(system, days).get('b')!.axes[2];
    const quarter = at(day / 4);
    const start = at(0);
    // A quarter of a day: its z axis a quarter turn round its axis.
    expect(Math.abs(start[0] * quarter[0] + start[1] * quarter[1] + start[2] * quarter[2])).toBeLessThan(0.05);
    // A whole day (and its orbit a little further): back nearly where it was.
    const whole = at(day);
    expect(start[0] * whole[0] + start[1] * whole[1] + start[2] * whole[2]).toBeGreaterThan(0.99);
    // Its spin axis stays put.
    const axis = (days: number) => placeSystem(system, days).get('b')!.axes[1];
    axis(0).forEach((v, i) => expect(v).toBeCloseTo(axis(3)[i]!, 9));
  });

  it('moves the planets round the star at their periods', () => {
    const period = system.bodies.find((x) => x.id === 'b')!.orbit.period_days;
    const at = (days: number) => placeSystem(system, days).get('b')!.centre;
    const a = at(0);
    const half = at(period / 2);
    const back = at(period);
    back.forEach((v, i) => expect(v / length(a)).toBeCloseTo(a[i]! / length(a), 6));
    expect((a[0] * half[0] + a[2] * half[2]) / (length(a) * length(half))).toBeLessThan(-0.9);
  });

  // How far a point is from a closed line through `loop`'s points.
  const fromLine = (point: Vec3, loop: readonly Vec3[]) =>
    Math.min(
      ...loop.map((a, i) => {
        const b = loop[(i + 1) % loop.length]!;
        const ab: Vec3 = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
        const ap: Vec3 = [point[0] - a[0], point[1] - a[1], point[2] - a[2]];
        const t = Math.max(0, Math.min(1, (ab[0] * ap[0] + ab[1] * ap[1] + ab[2] * ap[2]) / Math.max(length(ab) ** 2, 1e-30)));
        return length([ap[0] - t * ab[0], ap[1] - t * ab[1], ap[2] - t * ab[2]]);
      }),
    );

  it('runs every orbit line through its body’s centre, the giant’s own round the star too', () => {
    const sky = systemSky(system, 'c-1', 3, textures);
    const orbiting = system.bodies.filter((b) => b.parent === null || b.parent === 'c').map((b) => b.id);
    for (const id of orbiting.filter((x) => x !== 'c-1')) {
      const body = sky.bodies[sky.ids.indexOf(id)]!;
      // Within a millionth of the body's radius (the giant's line missed
      // it by over a tenth of its radius when sampled evenly only).
      expect(fromLine(body.position!, sky.orbits[orbiting.indexOf(id)]!)).toBeLessThan(1e-6 * body.radius!);
    }
  });

  it('keeps the giant’s own orbit line on its curve near the giant', () => {
    const sky = systemSky(system, 'c-1', 3, textures);
    const giant = sky.bodies[sky.ids.indexOf('c')]!;
    const at = system.bodies.filter((b) => b.parent === null || b.parent === 'c').findIndex((b) => b.id === 'c');
    // Its curve, sampled finely: within 30 of the giant's radii of it, no
    // point of the curve is further from the line than a thousandth of
    // its distance from the giant (what a camera there would see off it).
    const fine = systemSky(system, 'c-1', 3, textures, 0, 200_000).orbits[at]!;
    const off = (p: Vec3) => length([p[0] - giant.position![0], p[1] - giant.position![1], p[2] - giant.position![2]]);
    const near = fine.filter((p) => off(p) < 30 * giant.radius!);
    expect(near.length).toBeGreaterThan(10);
    for (const p of near) expect(fromLine(p, sky.orbits[at]!)).toBeLessThan(1e-3 * Math.max(off(p), giant.radius!));
  });

  it('samples the orbit lines evenly round what they orbit, as finely as asked at least', () => {
    const orbiting = system.bodies.filter((b) => b.parent === null || b.parent === 'c');
    for (const samples of [160, 720]) {
      const sky = systemSky(system, 'c-1', 3, textures, 0, samples);
      sky.orbits.forEach((loop, k) => {
        expect(loop.length).toBeGreaterThanOrEqual(samples);
        const parent = orbiting[k]!.parent ? sky.bodies[sky.ids.indexOf('c')]!.position! : sky.star.position;
        const from = (p: Vec3): Vec3 => [p[0] - parent[0], p[1] - parent[1], p[2] - parent[2]];
        loop.forEach((p, i) => {
          const [u, v] = [from(p), from(loop[(i + 1) % loop.length]!)];
          const angle = Math.acos(Math.min(1, (u[0] * v[0] + u[1] * v[1] + u[2] * v[2]) / (length(u) * length(v))));
          expect(angle).toBeLessThan(((2 * Math.PI) / samples) * (1 + 1e-6));
        });
      });
    }
  });
});

describe('small bodies in the sky', () => {
  // The fixture with trojans for the giant c and a belt round b's orbit.
  const b = system.bodies.find((x) => x.id === 'b')!;
  const withSmall: System = {
    ...system,
    belts: [
      {
        kind: 'inner',
        inner_au: b.orbit.semi_major_axis_au * 0.8,
        outer_au: b.orbit.semi_major_axis_au * 1.2,
        mass_earth: 4e-4,
        gaps: [],
        composition: 'rocky',
        eccentricity: 0.1,
        inclination_deg: 8,
        shaped_by: 'c',
      },
    ],
    bodies: system.bodies.map((body) => (body.id === 'c' ? { ...body, trojans: { l4: 230_000, l5: 120_000 } } : body)),
  };
  const angle = (u: Vec3, v: Vec3) =>
    (Math.acos(Math.min(1, (u[0] * v[0] + u[1] * v[1] + u[2] * v[2]) / (length(u) * length(v)))) * 180) / Math.PI;
  const minus = (u: Vec3, v: Vec3): Vec3 => [u[0] - v[0], u[1] - v[1], u[2] - v[2]];

  it('puts a planet’s swarms 60° ahead of it and behind it on its orbit', () => {
    const days = 40;
    const { swarms } = smallBodiesSky(withSmall, 'b', days);
    expect(swarms.map((s) => [s.planet, s.point, s.count])).toEqual([
      ['c', 'L4', 230_000],
      ['c', 'L5', 120_000],
    ]);
    const sky = systemSky(withSmall, 'b', days, textures);
    const star = sky.star.position;
    const c = sky.bodies[sky.ids.indexOf('c')]!.position!;
    const later = systemSky(withSmall, 'b', days + 30, textures);
    const cLater = later.bodies[later.ids.indexOf('c')]!.position!;
    const l4 = minus(swarms[0]!.position, star);
    const l5 = minus(swarms[1]!.position, star);
    const fromStar = minus(c, star);
    // 60° from the planet, seen from the star, as far from it: an
    // equilateral triangle, though its orbit is eccentric (0.26).
    expect(angle(l4, fromStar)).toBeCloseTo(60, 6);
    expect(angle(l5, fromStar)).toBeCloseTo(60, 6);
    // L4 ahead: the planet moves towards it; L5 behind.
    expect(angle(l4, minus(cLater, star))).toBeLessThan(angle(l4, fromStar));
    expect(angle(l5, minus(cLater, star))).toBeGreaterThan(angle(l5, fromStar));
    expect(length(l4) / length(fromStar)).toBeCloseTo(1, 9);
  });

  it('draws a belt’s edges round the star and knows when the body is in it', () => {
    const { belts } = smallBodiesSky(withSmall, 'b', 10);
    expect(belts).toHaveLength(1);
    expect(belts[0]!.inside).toBe(true);
    const star = systemSky(withSmall, 'b', 10, textures).star.position;
    const radii = (b.orbit.semi_major_axis_au * 0.8 * 1.495_978_707e8) / b.physical.radius_km;
    for (const point of belts[0]!.edges[0]) expect(length(minus(point, star)) / radii).toBeCloseTo(1, 6);
    // From the giant c, far beyond it, the body is not inside.
    expect(smallBodiesSky(withSmall, 'c', 10).belts[0]!.inside).toBe(false);
    // A system without small bodies has none.
    expect(smallBodiesSky(system, 'b', 10)).toEqual({ swarms: [], belts: [] });
  });
});
