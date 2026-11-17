import { describe, expect, it } from 'vitest';
import type { Body } from './api/types/Body';
import { dustRingSeed } from './skyBodies';
import {
  dustFedBy,
  dustOf,
  innerRingOf,
  ringPart,
  ringsOf,
  DUST_RING_MAX,
  dustColumn,
  dustRingUniforms,
} from './ringMoons';

const body = (over: Record<string, unknown>): Body =>
  ({
    id: 'c',
    physical: { radius_km: 60_000 },
    params: {},
    ...over,
  }) as unknown as Body;

describe('ringPart', () => {
  const giant = body({
    params: {
      given_rings: {
        rings: {
          inner: 1.2,
          outer: 2.27,
          gaps: [
            [2.2, 0.005],
            [1.95, 0.08],
          ],
          opacity: 0.8,
          colour: [0.86, 0.8, 0.68],
          bright: true,
          gap_moons: ['c-1', 'c-4'],
        },
      },
    },
  });

  it('gives the gap a moon opens, in km', () => {
    const pan = body({ id: 'c-1', ring_role: 'embedded' });
    expect(ringPart(pan, giant)).toEqual({ role: 'embedded', gapKm: 300 });
    const mimas = body({ id: 'c-4', ring_role: 'resonant' });
    expect(ringPart(mimas, giant)?.gapKm).toBeCloseTo(4800);
  });

  it('has no gap for an edge moon, and nothing for another moon', () => {
    expect(ringPart(body({ id: 'c-2', ring_role: 'edge' }), giant)).toEqual({ role: 'edge', gapKm: null });
    expect(ringPart(body({ id: 'c-5' }), giant)).toBeNull();
    expect(ringsOf(giant)?.bright).toBe(true);
    expect(ringsOf(body({}))).toBeNull();
  });
});

describe('dustFedBy', () => {
  it('finds the dust ring a moon feeds', () => {
    const plume = { kind: 'plume', inner: 3, outer: 8, peak: 4, opacity: 0.02, colour: [0.8, 0.9, 1], source: 'c-5' };
    const giant = body({ params: { given_rings: { rings: null, dust: [plume] } } });
    expect(dustOf(giant)).toHaveLength(1);
    expect(dustFedBy(body({ id: 'c-5' }), giant)?.kind).toBe('plume');
    expect(dustFedBy(body({ id: 'c-6' }), giant)).toBeNull();
    expect(dustOf(body({}))).toEqual([]);
  });
});

describe('innerRingOf', () => {
  const rings = (d_ring?: unknown) =>
    body({
      params: {
        given_rings: {
          rings: { inner: 1.24, outer: 2.27, gaps: [], opacity: 0.8, colour: [1, 1, 1], bright: true, d_ring },
        },
      },
    });

  it('draws a D-like ring as a smooth dust ring peaking in its middle', () => {
    const [d] = innerRingOf(rings({ inner: 1.12, outer: 1.24, opacity: 0.03 }));
    expect(d).toMatchObject({ inner: 1.12, outer: 1.24, opacity: 0.03 });
    expect(d?.peak).toBeCloseTo(1.18);
  });

  it('has none for rings without one, or a body without rings', () => {
    expect(innerRingOf(rings())).toEqual([]);
    expect(innerRingOf(body({}))).toEqual([]);
  });
});

describe('dustRingUniforms', () => {
  it('keeps the rings that matter most and pads the rest', () => {
    const six = Array.from({ length: 6 }, (_, i) => ({
      inner: i,
      peak: i + 0.5,
      outer: i + 1,
      opacity: 0.01 * (i + 1),
      colour: [1, 1, 1] as [number, number, number],
    }));
    const u = dustRingUniforms(six);
    expect(u.count).toBe(DUST_RING_MAX);
    expect(Array.from(u.opacity)).toEqual(expect.arrayContaining([0.06, 0.05, 0.04, 0.03].map(Math.fround)));
    expect(u.radii.length).toBe(3 * DUST_RING_MAX);
    expect(dustRingUniforms([]).count).toBe(0);
  });

  it('packs edges and peak, and converts the colour to linear', () => {
    const u = dustRingUniforms([{ inner: 3, peak: 4, outer: 6, opacity: 0.2, colour: [1, 0.5, 0] }]);
    expect(Array.from(u.radii.slice(0, 3))).toEqual([3, 4, 6]);
    expect(u.colour[0]).toBeCloseTo(1, 5);
    expect(u.colour[1]).toBeCloseTo(0.214, 3);
    expect(u.colour[2]).toBe(0);
    expect(Array.from(u.radii.slice(3))).toEqual(new Array(9).fill(0));
  });

  it("seeds each ring's ringlets from its peak", () => {
    const u = dustRingUniforms([{ inner: 3, peak: 4, outer: 6, opacity: 0.2, colour: [1, 1, 1] }]);
    expect(u.seed[0]).toBeCloseTo(dustRingSeed(4), 5);
    expect(u.seed[1]).toBe(0);
  });
});

describe('dustColumn', () => {
  const ring = { inner: 3, peak: 5, outer: 8, opacity: 0.1, colour: [1, 1, 1] as [number, number, number] };
  it('gives the face-on opacity to a vertical ray at the peak', () => {
    expect(dustColumn([5, -1, 0], [0, 1, 0], ring)).toBeCloseTo(0.1, 2);
    expect(Math.abs(dustColumn([5, -1, 0], [0, 1, 0], ring) / 0.1 - 1)).toBeLessThan(0.05);
  });
  it('is finite and thicker than face-on along the plane from inside', () => {
    const inPlane = dustColumn([4, 0, 0], [1, 0, 0], ring);
    expect(Number.isFinite(inPlane)).toBe(true);
    expect(inPlane).toBeGreaterThan(0.1);
    const up = dustColumn([4, 0, 0], [Math.cos(Math.PI / 6), Math.sin(Math.PI / 6), 0], ring);
    expect(up).toBeLessThan(inPlane);
    expect(up).toBeGreaterThan(0);
  });
  it('resolves the vertical Gaussian seen slant from afar, as from the giant', () => {
    // From 10 radii at 8 degrees over the plane, through the peak: the
    // column is the face-on one over the sine of the elevation.
    const e = (8 * Math.PI) / 180;
    const dir: [number, number, number] = [-Math.cos(e), -Math.sin(e), 0];
    const eye: [number, number, number] = [5 + 10 * Math.cos(e), 10 * Math.sin(e), 0];
    expect(dustColumn(eye, dir, ring) * Math.sin(e)).toBeCloseTo(0.1, 1);
  });
  it('is zero for a ray that misses the ring radially', () => {
    expect(dustColumn([10, 0, 0], [0, 0, 1], ring)).toBe(0);
  });
});
