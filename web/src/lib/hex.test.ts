import { describe, expect, it } from 'vitest';
import type { HexMap } from './api/types/HexMap';
import { hexAt, locateHex } from './hex';

// 8 columns of hexes 100 px wide (size 100 / √3) over an 800 × 400 map.
const size = 100 / Math.sqrt(3);
const map = {
  columns: 8,
  rows: 5,
  hex_width_px: 100,
  hex_size_px: size,
  source: { width: 800, height: 400, seed: 1, class: 'rocky' },
} as unknown as HexMap;

describe('locateHex', () => {
  it('finds the hex whose centre is at a point', () => {
    // Centre of hex (col, row): x = (col + 0.5 + 0.5 × (row odd)) × width, y = size + 1.5 × size × row.
    for (const [col, row] of [
      [0, 0],
      [3, 1],
      [7, 2],
      [5, 3],
    ] as const) {
      const x = (col + 0.5 + 0.5 * (row % 2)) * 100;
      const y = size + 1.5 * size * row;
      expect(locateHex(map, x, y)).toEqual({ col, row, index: row * 8 + col });
    }
  });

  it('wraps columns around the planet', () => {
    // Odd row: the last hex straddles the right edge, its other half is at x < 50.
    const { col } = locateHex(map, 10, size + 1.5 * size);
    expect(col).toBe(7);
  });

  it('locates longitude and latitude through the map projection', () => {
    // Longitude 0 at the left edge, the north pole at the top.
    expect(hexAt(map, { lon: 0.06, lat: Math.PI / 2 - 0.2 })).toMatchObject({ col: 0, row: 0 });
  });
});
