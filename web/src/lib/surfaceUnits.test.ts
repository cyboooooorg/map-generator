import { describe, expect, it } from 'vitest';
import type { Terrain } from './api/types/Terrain';
import { unitsFor } from './surfaceUnits';

describe('unitsFor', () => {
  it('gives no rows without terrain', () => {
    expect(unitsFor(undefined)).toEqual([]);
    expect(unitsFor({ units: [] })).toEqual([]);
  });

  it('gives one row for a body of one unit', () => {
    const terrain: Terrain = { units: [{ age_gyr: 0.4, share: 1, renewal: 'plates' }] };
    expect(unitsFor(terrain)).toEqual([{ age_gyr: 0.4, share: 1, renewal: 'plates' }]);
  });

  it('gives two rows for a Ganymede, the larger share first', () => {
    const terrain: Terrain = {
      units: [
        { age_gyr: 4.5, share: 0.35, renewal: 'ice-lid' },
        { age_gyr: 2.0, share: 0.65, renewal: 'tidal-episode' },
      ],
    };
    const rows = unitsFor(terrain);
    expect(rows.map((r) => r.renewal)).toEqual(['tidal-episode', 'ice-lid']);
    expect(rows.reduce((sum, r) => sum + r.share, 0)).toBeCloseTo(1, 12);
  });

  it('merges two units of the same renewal and age, without changing the terrain', () => {
    const terrain: Terrain = {
      units: [
        { age_gyr: 1, share: 0.3, renewal: 'plates' },
        { age_gyr: 1, share: 0.2, renewal: 'plates' },
        { age_gyr: 0.1, share: 0.5, renewal: 'volatile-ice' },
      ],
    };
    const rows = unitsFor(terrain);
    expect(rows).toHaveLength(2);
    expect(rows.find((r) => r.renewal === 'plates')?.share).toBeCloseTo(0.5, 12);
    expect(terrain.units[0]?.share).toBe(0.3);
  });
});
