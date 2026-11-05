import { describe, expect, it } from 'vitest';
import type { WorldMetadata } from './api/types/WorldMetadata';
import { skyFromWorld } from './sky';
import { eclipse, halo } from './skyBodies';

describe('eclipses', () => {
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
