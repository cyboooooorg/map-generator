import { describe, expect, it } from 'vitest';
import { ELEVATION_KM, reliefScale } from './relief';

describe('reliefScale', () => {
  it('gives an elevation unit in radii of the body', () => {
    // Earth: 25 km over a 6,378 km radius.
    expect(reliefScale(40075)).toBeCloseTo(ELEVATION_KM / 6378, 5);
    // A small moon of 300 km: the same unit is a twelfth of its radius.
    expect(reliefScale(2 * Math.PI * 300)).toBeCloseTo(1 / 12, 5);
  });

  it('is flat for a body without a size', () => {
    expect(reliefScale(0)).toBe(0);
  });
});
