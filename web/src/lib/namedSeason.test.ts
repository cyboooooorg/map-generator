import { describe, expect, it } from 'vitest';
import { namedSeason, sunDeclinationDeg } from './sky';

describe('namedSeason', () => {
  it('names a solstice only when the sun is over the tilt', () => {
    expect(namedSeason(0.25, sunDeclinationDeg(23.4, 0.25), 23.4)).toBe(1);
    expect(namedSeason(0.75, sunDeclinationDeg(23.4, 0.75), 23.4)).toBe(3);
    // 22.8° on a world tilted 23.4° is near the solstice, not the solstice.
    expect(namedSeason(0.25, 22.8, 23.4)).toBeNull();
  });
  it('names an equinox only when the sun is over the equator', () => {
    expect(namedSeason(0, 0, 23.4)).toBe(0);
    expect(namedSeason(1, 0, 23.4)).toBe(0);
    expect(namedSeason(0.5, 0.05, 23.4)).toBe(2);
    expect(namedSeason(0.5, 2, 23.4)).toBeNull();
  });
  it('names nothing between the dates', () => {
    expect(namedSeason(0.13, sunDeclinationDeg(23.4, 0.13), 23.4)).toBeNull();
  });
});
