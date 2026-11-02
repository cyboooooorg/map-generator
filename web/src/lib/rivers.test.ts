import { describe, expect, it } from 'vitest';
import type { RiversFile } from './api/types/RiversFile';
import { RiverIndex } from './rivers';

const rad = Math.PI / 180;
const file = {
  version: 3,
  width_law: {},
  rivers: [
    // Along the equator from 10° E to 12° E, flow growing downstream.
    { id: 7, parent: null, mouth: 'sea', source: null, estuary: true, points: [[10, 0, 100], [11, 0, 200], [12, 0, 300]] },
    // Across the date line: longitudes stay continuous (359° → 361°).
    { id: 8, parent: 7, mouth: 'river', source: null, estuary: false, points: [[359, 30, 10], [361, 30, 20]] },
  ],
} as unknown as RiversFile;

describe('RiverIndex', () => {
  const index = new RiverIndex(file, 36_000);

  it('finds the nearest river and interpolates its flow', () => {
    const hit = index.nearest({ lon: 10.5 * rad, lat: 0.05 * rad }, 0.2);
    expect(hit?.river.id).toBe(7);
    expect(hit?.flowKm2).toBeCloseTo(150, 5);
    expect(hit?.distanceDeg).toBeCloseTo(0.05, 5);
  });

  it('finds nothing beyond the tolerance', () => {
    expect(index.nearest({ lon: 10.5 * rad, lat: 1 * rad }, 0.2)).toBeNull();
  });

  it('handles rivers crossing the date line', () => {
    expect(index.nearest({ lon: 0.5 * rad, lat: 30 * rad }, 0.2)?.river.id).toBe(8);
  });

  it('measures lengths along the planet', () => {
    // 2° of a 36 000 km circumference is 200 km.
    expect(index.lengthKm(file.rivers[0]!)).toBeCloseTo(200, 3);
    expect(index.mouthFlowKm2(file.rivers[0]!)).toBe(300);
  });
});
