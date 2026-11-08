import { describe, expect, it } from 'vitest';
import type { Body } from './api/types/Body';
import { dustFedBy, dustOf, innerRingOf, ringPart, ringsOf } from './ringMoons';

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
    body({ params: { given_rings: { rings: { inner: 1.24, outer: 2.27, gaps: [], opacity: 0.8, colour: [1, 1, 1], bright: true, d_ring } } } });

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
