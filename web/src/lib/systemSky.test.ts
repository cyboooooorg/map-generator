import { describe, expect, it } from 'vitest';
import type { System } from './api/types/System';
import fixture from './systemSky.fixture.json';
import { placeSystem, systemReach, systemSky } from './systemSky';
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

  it('keeps a locked moon facing its giant, which it sees on its own orbit', () => {
    const sky = systemSky(system, 'c-1', 3, textures);
    const giant = sky.bodies[sky.ids.indexOf('c')]!;
    giant.direction.forEach((v, i) => expect(v).toBeCloseTo([0, 0, -1][i]!, 6));
    // The planets' orbits and the giant's moons'; its own passes through it.
    expect(sky.orbits).toHaveLength(4);
    const own = sky.orbits[2]!;
    expect(Math.min(...own.map(length))).toBeLessThan(1);
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
});
