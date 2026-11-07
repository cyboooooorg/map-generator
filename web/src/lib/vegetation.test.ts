import { describe, expect, it } from 'vitest';
import { vegetationShift } from './viewer/PlanetView';
import { starLitColour, vegetationShares } from './vegetation';

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

  it('colours a swatch as the globe shows it under Star light', () => {
    // No shift and white light: the colour itself.
    expect(starLitColour('#3a7d2c', 1, [0, 0], [1, 1, 1])).toBe('#3a7d2c');
    // A forest under a red dwarf: darker, and redder than green.
    const red = starLitColour('#3a7d2c', 1, [1, 0], [1, 0.6, 0.3]);
    const [r, g] = [parseInt(red.slice(1, 3), 16), parseInt(red.slice(3, 5), 16)];
    expect(r).toBeGreaterThan(g);
    expect(r + g).toBeLessThan(0x3a + 0x7d);
    // Rock, with no plants, only takes the light's colour.
    expect(starLitColour('#808080', 0, [1, 0], [1, 1, 1])).toBe('#808080');
  });
});
