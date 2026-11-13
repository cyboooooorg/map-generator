import { describe, expect, it } from 'vitest';
import type { BodySummary } from './api/types/BodySummary';
import type { SystemSummary } from './api/types/SystemSummary';
import { cssColour, orbitRadius, orrery } from './orrery';

const body = (id: string, orbit_au: number, extra: Partial<BodySummary> = {}): BodySummary => ({
  id,
  designation: `MGS 1 ${id}`,
  kind: 'planet',
  body_type: 'terran',
  colour: [0.2, 0.3, 0.5],
  generated: false,
  orbit_au,
  eccentricity: 0,
  periapsis_deg: 0,
  ...extra,
});

const system = (bodies: BodySummary[], hz: [number, number] = [0.9, 1.5]): SystemSummary => ({
  id: 'mgs-1-7',
  seed: 7,
  width: 1024,
  height: 512,
  star: 'G2V',
  designation: 'MGS 1',
  star_light: [1, 1, 1],
  hz_inner_au: hz[0],
  hz_outer_au: hz[1],
  planets: {},
  moons: 0,
  bodies,
  names: {},
  modified_ms: 0,
  pinned: false,
  outdated: false,
});

describe('orbitRadius', () => {
  it('runs from the star to the edge on a square-root scale', () => {
    expect(orbitRadius(0, 10, 4, 20)).toBe(4);
    expect(orbitRadius(10, 10, 4, 20)).toBe(20);
    expect(orbitRadius(2.5, 10, 4, 20)).toBe(12);
    expect(orbitRadius(1, 0, 4, 20)).toBe(4);
  });
});

describe('orrery', () => {
  it('puts the outermost orbit at the edge of the box and every dot inside it', () => {
    const o = orrery(system([body('b', 0.4), body('c', 1), body('d', 5.2, { body_type: 'gas-giant' })]), 72, 40);
    expect(o.planets.map((p) => p.id)).toEqual(['b', 'c', 'd']);
    const outer = o.planets[2]!;
    expect(outer.rx).toBeCloseTo(36 - 3);
    expect(outer.ry).toBeCloseTo(20 - 3);
    for (const p of o.planets) {
      expect(p.x - p.r).toBeGreaterThanOrEqual(0);
      expect(p.x + p.r).toBeLessThanOrEqual(72);
      expect(p.y - p.r).toBeGreaterThanOrEqual(0);
      expect(p.y + p.r).toBeLessThanOrEqual(40);
    }
    // Giants larger.
    expect(outer.r).toBeGreaterThan(o.planets[0]!.r);
  });

  it('leaves out moons and orbits it does not know, and the zone when unknown', () => {
    const o = orrery(system([body('b', 0), body('c', 1), body('c-1', 0.002, { kind: 'moon' })], [0, 0]), 72, 40);
    expect(o.planets.map((p) => p.id)).toEqual(['c']);
    expect(o.zone).toBeNull();
  });

  it('draws the habitable zone inside the outermost orbit', () => {
    const o = orrery(system([body('b', 1)], [0.9, 1.5]), 72, 40);
    expect(o.zone).not.toBeNull();
    expect(o.zone!.outer[0]).toBeCloseTo(o.planets[0]!.rx);
    expect(o.zone!.inner[0]).toBeLessThan(o.zone!.outer[0]);
  });

  it('draws an eccentric orbit with the star at a focus, its far point at the edge', () => {
    const o = orrery(system([body('b', 1, { eccentricity: 0.5, periapsis_deg: 0 })]), 72, 40);
    const points = o.planets[0]!.orbit
      .replace(/[MLZ]/g, ' ')
      .trim()
      .split(/\s+/)
      .map(Number);
    const xs = points.filter((_, i) => i % 2 === 0);
    // Periapsis to the right of the star, apoapsis to the left at the edge.
    expect(Math.min(...xs)).toBeCloseTo(3, 1);
    expect(Math.max(...xs) - 36).toBeLessThan(36 - Math.min(...xs));
  });

  it('draws a system the same way every time', () => {
    const s = system([body('b', 0.4), body('c', 1)]);
    expect(orrery(s, 72, 40)).toEqual(orrery(s, 72, 40));
  });
});

describe('cssColour', () => {
  it('converts linear RGB to CSS', () => {
    expect(cssColour([1, 1, 1])).toBe('rgb(255 255 255)');
    expect(cssColour([0, 0, 0])).toBe('rgb(0 0 0)');
  });

  it('spreads the inner orbits of a crowded system on the log scale, inside the box', () => {
    const bodies = [0.02, 0.03, 0.05, 0.08, 0.5, 3].map((au, k) => body(`p${k}`, au));
    const sqrt = orrery(system(bodies, [0.04, 0.09]), 124, 124);
    const log = orrery(system(bodies, [0.04, 0.09]), 124, 124, 'log');
    const gap = (o: ReturnType<typeof orrery>) => (o.planets[1]?.rx ?? 0) - (o.planets[0]?.rx ?? 0);
    expect(gap(log)).toBeGreaterThan(gap(sqrt));
    for (const p of log.planets) {
      expect(p.rx).toBeLessThanOrEqual(124 / 2);
      expect(p.rx).toBeGreaterThan(log.starR);
    }
    expect(log.starR).toBeLessThan(sqrt.starR);
  });
});
