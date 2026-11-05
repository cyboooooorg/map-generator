import { describe, expect, it } from 'vitest';
import type { System } from './api/types/System';
import fixture from './systemSky.fixture.json';
import { systemStarView } from './systemStar';

const system = fixture as unknown as System;

describe('system star view', () => {
  it('puts the planet shown on the orbit and the others round it, as giants', () => {
    const b = system.bodies.find((x) => x.id === 'b')!;
    const view = systemStarView(system, b);
    expect(view.star.orbit_au).toBe(b.orbit.semi_major_axis_au);
    expect(view.star.year_days).toBe(b.orbit.period_days);
    // The other planets only, not the moons, in the system's order.
    expect(view.ids).toEqual(['c']);
    const giant = view.giants.giants[0]!;
    expect(giant.orbit_au).toBe(system.bodies.find((x) => x.id === 'c')!.orbit.semi_major_axis_au);
    // Its preview's colour, in sRGB as the markers take it.
    const linear = system.bodies.find((x) => x.id === 'c')!.preview.colours[0]![0];
    expect(giant.colour[0]).toBeGreaterThan(linear);
  });
});
