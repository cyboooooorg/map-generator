import { describe, expect, it } from 'vitest';
import type { Body } from './api/types/Body';
import { ringPart, ringsOf } from './ringMoons';

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
