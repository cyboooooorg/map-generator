import { describe, expect, it } from 'vitest';
import type { System } from './api/types/System';
import fixture from './systemSky.fixture.json';
import { systemStarView } from './systemStar';

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
});
