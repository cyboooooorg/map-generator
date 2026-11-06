import { describe, expect, it } from 'vitest';
import type { Belt } from '../api/types/Belt';
import { beltParticles, lagrangePoints, massShare, periodDays, ROUTH, swarmParticles, swarmPoints } from './smallBodies';

const main: Belt = {
  kind: 'inner',
  inner_au: 2.1,
  outer_au: 3.3,
  mass_earth: 4e-4,
  gaps: [
    [2.5, 0.03],
    [2.82, 0.03],
  ],
  composition: 'mixed',
  eccentricity: 0.15,
  inclination_deg: 10,
  shaped_by: 'f',
};

describe('small bodies', () => {
  it('spreads a belt across its span but for its gaps, the same each time', () => {
    const points = beltParticles(main, 2000, 7);
    expect(points).toHaveLength(2000);
    for (const p of points) {
      expect(p.a).toBeGreaterThanOrEqual(2.1);
      expect(p.a).toBeLessThanOrEqual(3.3);
      expect(Math.abs(p.a - 2.5)).toBeGreaterThanOrEqual(0.015);
      expect(Math.abs(p.a - 2.82)).toBeGreaterThanOrEqual(0.015);
    }
    expect(beltParticles(main, 50, 7)).toEqual(beltParticles(main, 50, 7));
  });

  it('places the Lagrange points as the restricted three-body problem does', () => {
    const mu = massShare(317.8, 1);
    expect(mu).toBeCloseTo(9.54e-4, 6);
    const [l1, l2, l3, l4, l5] = lagrangePoints(5.2, mu);
    // Jupiter's L1 and L2 lie about 0.35 AU (its Hill radius) either side.
    expect(5.2 - l1!.au).toBeCloseTo(0.355, 2);
    expect(l2!.au - 5.2).toBeCloseTo(0.355, 2);
    expect(l3!.angle).toBe(Math.PI);
    expect(l4!.angle).toBeCloseTo(Math.PI / 3, 10);
    expect(l5!.angle).toBeCloseTo(-Math.PI / 3, 10);
    expect([l1, l2, l3].every((p) => !p!.stable)).toBe(true);
    expect(l4!.stable && l5!.stable).toBe(true);
    // Past Routh's limit, L4 and L5 are not stable either.
    expect(lagrangePoints(1, ROUTH * 1.01)[3]!.stable).toBe(false);
  });

  it('draws trojans on tadpoles round their point, within the swarm width', () => {
    const mu = massShare(317.8, 1);
    const swarm = swarmParticles('L4', 300, 5.2, mu, 3);
    const width = Math.sqrt((8 * mu) / 3) * 5.2;
    for (const t of swarm) {
      expect(t.angle).toBeGreaterThan((20 * Math.PI) / 180);
      expect(t.angle).toBeLessThan((100 * Math.PI) / 180);
      expect(Math.abs(t.au - 5.2)).toBeLessThanOrEqual(width + 1e-9);
    }
    expect(swarmParticles('L5', 10, 5.2, mu, 3).every((t) => t.angle < 0)).toBe(true);
    // A tiny planet's swarm is drawn at least as wide as asked.
    const earth = swarmParticles('L4', 200, 1, massShare(1, 1), 3, 0.05);
    expect(Math.max(...earth.map((t) => Math.abs(t.au - 1)))).toBeGreaterThan(0.02);
  });

  it('draws more points for larger swarms, a few hundred at most', () => {
    expect(swarmPoints(0)).toBe(0);
    expect(swarmPoints(10)).toBeGreaterThan(30);
    expect(swarmPoints(260_000)).toBeGreaterThan(swarmPoints(1000));
    expect(swarmPoints(1e12)).toBe(400);
  });

  it('gives Kepler periods', () => {
    expect(periodDays(1, 1)).toBeCloseTo(365.25, 6);
    expect(periodDays(5.2, 1) / 365.25).toBeCloseTo(11.86, 1);
  });
});
