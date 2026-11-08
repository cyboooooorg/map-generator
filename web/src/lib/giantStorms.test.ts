import { describe, expect, it } from 'vitest';
import { giantStorms, greatSpotIndex, MAX_STORMS } from './giantStorms';

const STORM = 7;

/** A map of `width` × `height` with ellipses of storm pixels at (x, y, rx, ry). */
function map(width: number, height: number, storms: [number, number, number, number][]): Uint8Array {
  const ids = new Uint8Array(width * height).fill(1);
  for (const [cx, cy, rx, ry] of storms) {
    for (let y = 0; y < height; y++) {
      for (let x = 0; x < width; x++) {
        let dx = x - cx;
        if (dx > width / 2) dx -= width;
        if (dx < -width / 2) dx += width;
        if ((dx / rx) ** 2 + ((y - cy) / ry) ** 2 <= 1) ids[y * width + x] = STORM;
      }
    }
  }
  return ids;
}

describe('giantStorms', () => {
  it('finds each storm with its centre and extent, the largest first', () => {
    const storms = giantStorms(map(200, 100, [[50, 70, 6, 4], [140, 30, 12, 6]]), 200, 100, STORM);
    expect(storms).toHaveLength(2);
    const [big, small] = storms;
    // Row 30 from the top is v = 0.695 from the south pole.
    expect(big!.u).toBeCloseTo(140.5 / 200, 2);
    expect(big!.v).toBeCloseTo(1 - 30.5 / 100, 2);
    expect(big!.halfU).toBeCloseTo(12.5 / 200, 2);
    expect(big!.halfV).toBeCloseTo(6.5 / 100, 2);
    expect(small!.u).toBeCloseTo(50.5 / 200, 2);
  });

  it('keeps a storm across the seam whole', () => {
    const storms = giantStorms(map(200, 100, [[0, 50, 8, 4]]), 200, 100, STORM);
    expect(storms).toHaveLength(1);
    expect(Math.min(storms[0]!.u, 1 - storms[0]!.u)).toBeLessThan(0.01);
    expect(storms[0]!.halfU).toBeCloseTo(8.5 / 200, 2);
  });

  it('leaves specks to their band and keeps the largest storms at most', () => {
    expect(giantStorms(map(200, 100, [[50, 50, 1, 1]]), 200, 100, STORM)).toHaveLength(0);
    const many: [number, number, number, number][] = Array.from({ length: 20 }, (_, k) => [5 + k * 10, 50, 3, 3]);
    expect(giantStorms(map(200, 100, many), 200, 100, STORM)).toHaveLength(MAX_STORMS);
  });
});

describe('greatSpotIndex', () => {
  const storms = [
    { u: 0.5, v: 0.5, halfU: 0.02, halfV: 0.02 },
    { u: 0.25, v: 0.5 - 22 / 180, halfU: 0.02, halfV: 0.02 },
  ];
  it('finds the storm at the great spot, whichever is the largest', () => {
    // Longitude 90°, latitude -22°: the second storm.
    expect(greatSpotIndex(storms, [90, -22])).toBe(1);
    expect(greatSpotIndex(storms, [180, 0])).toBe(0);
  });
  it('is -1 with no great spot or no storm near it', () => {
    expect(greatSpotIndex(storms, null)).toBe(-1);
    expect(greatSpotIndex(storms, [300, 40])).toBe(-1);
    expect(greatSpotIndex([], [90, -22])).toBe(-1);
  });
});
