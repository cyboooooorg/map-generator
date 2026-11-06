import { describe, expect, it } from 'vitest';
import { SKY_ORBIT_DASH, skyOrbitDash } from './shaders';

describe('skyOrbitDash', () => {
  it('keeps the still period while the orbits stand still or crawl', () => {
    expect(skyOrbitDash(1400, 0)).toBe(SKY_ORBIT_DASH);
    expect(skyOrbitDash(10, 0.1)).toBe(SKY_ORBIT_DASH);
  });

  it('lets the dashes go past at most a quarter of a period a frame at 60 fps', () => {
    // A moon 222 of its radii from its planet, round once every 10 s.
    const length = 2 * Math.PI * 222;
    const dash = skyOrbitDash(length, 0.1);
    const perFrame = (length * 0.1) / 60;
    expect(perFrame / dash).toBeCloseTo(0.25, 6);
    // Twice the pace, twice the period.
    expect(skyOrbitDash(length, 0.2)).toBeCloseTo(2 * dash, 6);
  });
});
