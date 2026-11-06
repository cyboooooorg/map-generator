import { describe, expect, it } from 'vitest';
import { SKY_ORBIT_DASH, SKY_ORBIT_DASHES_PER_SECOND, skyOrbitDash } from './shaders';

describe('skyOrbitDash', () => {
  it('keeps the still period while the orbits stand still or crawl', () => {
    expect(skyOrbitDash(1400, 0)).toBe(SKY_ORBIT_DASH);
    expect(skyOrbitDash(10, 0.1)).toBe(SKY_ORBIT_DASH);
  });

  it('lets a fixed number of dashes a second go past, whatever the speed', () => {
    // A moonlet 33,900 of its radii from its giant, round once every 43 min.
    const length = 2 * Math.PI * 33_900;
    const pace = 1 / 2560;
    const dash = skyOrbitDash(length, pace);
    const perSecond = (length * pace) / dash;
    expect(perSecond).toBeCloseTo(SKY_ORBIT_DASHES_PER_SECOND, 6);
    expect(perSecond).toBeLessThanOrEqual(2);
    // Twice the pace, twice the period: as many dashes a second.
    expect(skyOrbitDash(length, 2 * pace)).toBeCloseTo(2 * dash, 6);
  });
});
