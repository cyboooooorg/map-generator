import { describe, expect, it } from 'vitest';
import { ancestor, children, cropOf, firstDetailLevel, grid, tileRect } from './tiles';

describe('tiles', () => {
  it('cover the planet at every level', () => {
    expect(grid(0)).toEqual({ cols: 2, rows: 1 });
    expect(tileRect({ z: 0, x: 1, y: 0 })).toEqual([0.5, 0, 0.5, 1]);
    // Row 0 is the north pole: the top of texture space.
    const [, v0, , dv] = tileRect({ z: 2, x: 0, y: 0 });
    expect(v0 + dv).toBe(1);
  });

  it('nest children inside their parent', () => {
    const parent = { z: 3, x: 5, y: 2 };
    const [pu, pv, pdu, pdv] = tileRect(parent);
    for (const child of children(parent)) {
      const [u, v, du, dv] = tileRect(child);
      expect(u).toBeGreaterThanOrEqual(pu);
      expect(v).toBeGreaterThanOrEqual(pv);
      expect(u + du).toBeLessThanOrEqual(pu + pdu + 1e-12);
      expect(v + dv).toBeLessThanOrEqual(pv + pdv + 1e-12);
      expect(ancestor(child, 3)).toEqual(parent);
    }
  });

  it('crop the right quarter of a parent image', () => {
    expect(cropOf({ z: 4, x: 11, y: 5 }, { z: 3, x: 5, y: 2 })).toEqual([0.5, 0.5, 0.5, 0.5]);
    expect(cropOf({ z: 5, x: 20, y: 8 }, { z: 3, x: 5, y: 2 })).toEqual([0, 0, 0.25, 0.25]);
  });

  it('start detail where tiles beat the base map', () => {
    // 1920 px: level 2 is 2048 px wide, level 3 is 4096.
    expect(firstDetailLevel(1920)).toBe(2);
    expect(firstDetailLevel(4096)).toBe(4);
  });
});
