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

  it('darkens under red dwarfs, yellows under hot stars, leaves the Sun and early K dwarfs alone', () => {
    expect(vegetationShift(5772)).toEqual([0, 0]);
    // K2V: "very similar to the Earth" (Kiang et al. 2007).
    expect(vegetationShift(5040)).toEqual([0, 0]);
    expect(vegetationShift(4440)[0]).toBeCloseTo(0.31, 2);
    expect(vegetationShift(3400)[0]).toBe(1);
    expect(vegetationShift(3000)[0]).toBe(1);
    expect(vegetationShift(7300)[1]).toBeCloseTo(1, 2);
  });
});
