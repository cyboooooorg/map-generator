import { describe, expect, it } from 'vitest';
import { vegetationShift } from './viewer/PlanetView';
import { vegetationShares } from './vegetation';

describe('vegetation under other stars', () => {
  it('covers the vegetated biomes only, by id', () => {
    const shares = vegetationShares([
      { id: 0, key: 'DeepOcean' },
      { id: 9, key: 'Forest' },
      { id: 5, key: 'Tundra' },
    ]);
    expect(shares[9]).toBe(1);
    expect(shares[5]).toBe(0.35);
    expect(shares[0]).toBe(0);
    expect(shares[3]).toBe(0);
  });

  it('darkens under cool stars, yellows under hot ones, leaves the Sun alone', () => {
    expect(vegetationShift(5772)).toEqual([0, 0]);
    expect(vegetationShift(3000)[0]).toBe(1);
    expect(vegetationShift(4400)[0]).toBeCloseTo(0.55, 2);
    expect(vegetationShift(7300)[1]).toBeCloseTo(1, 2);
  });
});
