import { describe, expect, it } from 'vitest';
import type { System } from './api/types/System';
import fixture from './systemSky.fixture.json';
import { ringsKind, systemStarView } from './systemStar';
import { anomalies } from './systemSky';

const system = fixture as unknown as System;

describe('system star view', () => {
  it('puts the planet shown on the orbit and every planet round it, as giants', () => {
    const b = system.bodies.find((x) => x.id === 'b')!;
    const view = systemStarView(system, b);
    expect(view.star.orbit_au).toBe(b.orbit.semi_major_axis_au);
    expect(view.star.year_days).toBe(b.orbit.period_days);
    // Every planet, the one shown included, not the moons, in the system's order.
    expect(view.ids).toEqual(['b', 'c']);
    expect(view.planets.map((p) => p.id)).toEqual(['b', 'c']);
    expect(view.selected).toBe(0);
    expect(systemStarView(system, system.bodies.find((x) => x.id === 'c')!).selected).toBe(1);
    const giant = view.giants.giants[1]!;
    expect(giant.orbit_au).toBe(system.bodies.find((x) => x.id === 'c')!.orbit.semi_major_axis_au);
    // Its preview's colour, in sRGB as the markers take it.
    const linear = system.bodies.find((x) => x.id === 'c')!.preview.colours[0]![0];
    expect(giant.colour[0]).toBeGreaterThan(linear);
    expect(giant.name).toBe(system.bodies.find((x) => x.id === 'c')!.designation);
  });

  it('labels the other planets with the names given to them', () => {
    const b = system.bodies.find((x) => x.id === 'b')!;
    expect(systemStarView(system, b, { c: 'Tethys' }).giants.giants[1]!.name).toBe('Tethys');
  });

  it('gives each planet the facts its card shows', () => {
    const b = system.bodies.find((x) => x.id === 'b')!;
    const [pb, pc] = systemStarView(system, b, { c: 'Tethys' }, { [b.type]: 'Worded' }).planets;
    expect(pb!.typeLabel).toBe('Worded');
    expect(pb!.moons).toBe(0);
    expect(pc!.moons).toBe(2);
    expect(pc!.name).toBe('Tethys');
    expect(pc!.orbit_au).toBe(system.bodies.find((x) => x.id === 'c')!.orbit.semi_major_axis_au);
  });

  it('gives each planet its tilt and its moons, nearest first', () => {
    const c = system.bodies.find((x) => x.id === 'c')!;
    const view = systemStarView(system, c, { 'c-1': 'Io' });
    const giant = view.giants.giants[1]!;
    expect(giant.inclination).toBeCloseTo((c.orbit.inclination_deg * Math.PI) / 180, 10);
    expect(giant.node).toBeCloseTo((c.orbit.node_deg * Math.PI) / 180, 10);
    expect(giant.moons).toHaveLength(2);
    const list = view.planets[1]!.moonList;
    expect(list.map((m) => m.id)).toEqual(['c-1', 'c-2']);
    expect(list[0]!.name).toBe('Io');
    expect(list[0]!.semi_major_axis_km).toBeLessThan(list[1]!.semi_major_axis_km);
  });
});

describe('system star view epoch', () => {
  it('runs the system view on the planets epoch', () => {
    const b = system.bodies.find((x) => x.id === 'b')!;
    const view = systemStarView(system, b);
    const planets = system.bodies.filter((x) => x.kind === 'planet');
    const TWO_PI = 2 * Math.PI;
    const wrap = (x: number) => ((x % TWO_PI) + TWO_PI) % TWO_PI;
    for (const days of [0, 1234]) {
      planets.forEach((planet, k) => {
        const giant = view.giants.giants[k]!;
        // StarView's mean anomaly at its systemDay (giantAnomaly).
        const shown = giant.phase * TWO_PI + (TWO_PI * days) / giant.year_days;
        const sky = anomalies(planet.orbit, days).mean;
        const diff = wrap(shown - sky);
        expect(Math.min(diff, TWO_PI - diff)).toBeLessThan(1e-9);
      });
    }
  });
});

describe('ringsKind', () => {
  it('sums up a ring system for the star view', () => {
    const ring = { inner: 1.2, outer: 2.2, gaps: [], opacity: 0.7, colour: [0.8, 0.8, 0.7] as [number, number, number] };
    expect(ringsKind({ ...ring, bright: true })).toBe('bright');
    expect(ringsKind({ ...ring, bright: false })).toBe('faint');
    expect(ringsKind(null)).toBe('none');
  });

  it('draws a mini-Neptune as an ice giant', () => {
    const c = system.bodies.find((x) => x.id === 'c')!;
    const mini = { ...system, bodies: system.bodies.map((x) => (x.id === 'c' ? { ...x, type: 'mini-neptune' } : x)) };
    expect(systemStarView(mini, c).giants.giants[1]!.kind).toBe('ice');
  });
});
