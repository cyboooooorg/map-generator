import { describe, expect, it } from 'vitest';
import { surfaceDetail } from './starSurface';
import { surfaceRadiusPx } from './StarBodyView';

/** The Sun fixture's granule size, star radii (starActivity.test.ts). */
const GRANULE = 0.0018903897;

describe('star view detail', () => {
  it('sizes the surface from its nearest point, so granules show up close', () => {
    // 1.02 radii from the centre, 0.02 above the surface: a granule spans
    // about 140 px of a 1080 px tall view (40° field of view).
    const close = surfaceRadiusPx(1.02, 40, 1080);
    expect(close * GRANULE).toBeGreaterThan(100);
    expect(surfaceDetail(close, GRANULE).granulation).toBe(1);
    // From the centre's distance, as the system view measures, it would be under 3 px.
    expect((540 / (1.02 * Math.tan((20 * Math.PI) / 180))) * GRANULE).toBeLessThan(3);
  });

  it('keeps granules sub-pixel with the whole disc in view', () => {
    const far = surfaceRadiusPx(3.24, 40, 1080);
    expect(far * GRANULE).toBeLessThan(1.5);
    expect(surfaceDetail(far, GRANULE).granulation).toBe(0);
    expect(surfaceDetail(far, GRANULE).regions).toBe(1);
  });

  it('draws nothing without a size', () => {
    expect(surfaceRadiusPx(2, 40, 0)).toBe(0);
    expect(surfaceRadiusPx(1, 40, 1080)).toBeGreaterThan(0);
  });
});
