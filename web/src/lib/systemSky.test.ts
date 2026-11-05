import { describe, expect, it } from 'vitest';
import type { System } from './api/types/System';
import fixture from './systemSky.fixture.json';
import { placeSystem, systemSky } from './systemSky';
import type { Vec3 } from './skyBodies';

// A G2V system drawn by the generator: b (terran, generated), c (a gas
// giant) and its two regular moons c-1 and c-2, locked to it.
const system = fixture as unknown as System;
const textures = (id: string) => `${id}.png`;
const length = (v: Vec3) => Math.hypot(...v);

describe('system sky', () => {
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
