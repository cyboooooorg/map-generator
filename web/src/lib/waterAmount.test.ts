import { describe, expect, it } from 'vitest';
import { waterDepthKm, waterShare } from './waterAmount';

describe('waterAmount', () => {
  it('gives the share of the mass', () => {
    expect(waterShare(2390, 4.78)).toBeCloseTo(0.117, 2);
    expect(waterShare(1, 1)).toBeCloseTo(2.3e-4, 5);
  });
  it('gives the depth over the whole planet', () => {
    expect(waterDepthKm(1, 1)).toBeCloseTo(2.7, 1);
    expect(waterDepthKm(0, 1)).toBe(0);
  });
});
