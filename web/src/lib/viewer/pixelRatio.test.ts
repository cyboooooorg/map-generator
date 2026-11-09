import { describe, expect, it } from 'vitest';
import { animatedPixelRatio, fullPixelRatio, MAX_PIXEL_RATIO, MOVING_PIXEL_RATIO, pixelRatio, SETTLE_MS } from './pixelRatio';

describe('pixelRatio', () => {
  it('caps the settled ratio', () => {
    expect(fullPixelRatio(3)).toBe(MAX_PIXEL_RATIO);
    expect(fullPixelRatio(1.5)).toBe(1.5);
    expect(pixelRatio(3, Infinity)).toBe(MAX_PIXEL_RATIO);
    expect(pixelRatio(1, SETTLE_MS)).toBe(1);
  });

  it('lowers the ratio while the view moves and shortly after', () => {
    expect(pixelRatio(2, 0)).toBe(MOVING_PIXEL_RATIO);
    expect(pixelRatio(3, SETTLE_MS - 1)).toBe(MOVING_PIXEL_RATIO);
    expect(pixelRatio(2, SETTLE_MS)).toBe(2);
  });

  it('never raises a low device ratio', () => {
    expect(pixelRatio(1, 0)).toBe(1);
    expect(pixelRatio(0.8, 0)).toBe(0.8);
  });

  it('draws an animated star at the moving ratio', () => {
    expect(animatedPixelRatio(2, true, { maxPixelRatio: 2, movingPixelRatio: 1.25 })).toBe(1.25);
    expect(animatedPixelRatio(2, false, { maxPixelRatio: 2, movingPixelRatio: 1.25 })).toBe(2);
    expect(animatedPixelRatio(1, true, { maxPixelRatio: 2, movingPixelRatio: 1.25 })).toBe(1);
  });

  it('draws an animated close-up at ratio 1 at most', () => {
    const caps = { maxPixelRatio: 2, movingPixelRatio: 1.25 };
    expect(animatedPixelRatio(2, true, caps, true)).toBe(1);
    expect(animatedPixelRatio(2, true, caps, false)).toBe(1.25);
    // Paused, the close-up is drawn sharp; a lower screen or cap is never raised.
    expect(animatedPixelRatio(2, false, caps, true)).toBe(2);
    expect(animatedPixelRatio(0.8, true, caps, true)).toBe(0.8);
    expect(animatedPixelRatio(2, true, { maxPixelRatio: 2, movingPixelRatio: 0.9 }, true)).toBe(0.9);
  });
});
