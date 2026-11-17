import { describe, expect, it } from 'vitest';
import { PLUME_MIN_PX, plumeAcross, plumeAlpha, plumeFrame } from './plume';

const AXES = [
  [1, 0, 0],
  [0, 1, 0],
  [0, 0, 1],
] as const;

describe('plume', () => {
  it('stands on the venting pole at the drawn size', () => {
    const south = plumeFrame(AXES, true, 2).up;
    expect(south[0]).toBeCloseTo(0);
    expect(south[1]).toBeCloseTo(-1);
    expect(south[2]).toBeCloseTo(0);
    const north = plumeFrame(AXES, false, 20);
    expect(north.up[1]).toBeCloseTo(1);
    expect(north.height).toBeCloseTo(30);
  });

  it('keeps a floor of its own at the moon\'s size floor and 1.5 radii on a large body', () => {
    expect(plumeFrame(AXES, true, 2.5).height).toBeGreaterThanOrEqual(PLUME_MIN_PX);
    expect(plumeFrame(AXES, true, 100).height).toBeCloseTo(150);
  });

  it('is brightest back-lit and still seen side-lit', () => {
    // phaseCos -1 is a new moon: the light passes through the plume to the eye.
    expect(plumeAlpha(-1)).toBe(1);
    expect(plumeAlpha(0)).toBeCloseTo(0.35);
    expect(plumeAlpha(1)).toBeLessThan(plumeAlpha(0));
    expect(plumeAlpha(1)).toBeGreaterThan(0.02);
    expect(plumeAlpha(-0.5)).toBeGreaterThan(plumeAlpha(0.5));
  });
  it('keeps the quad front-facing: (across, axis) is positively handed like (right, up)', () => {
    for (const deg of [0, 37, 90, 145, 200, 300]) {
      const a: [number, number] = [Math.cos((deg * Math.PI) / 180), Math.sin((deg * Math.PI) / 180)];
      const [px, py] = plumeAcross(a);
      expect(px * a[1] - py * a[0]).toBeCloseTo(1);
    }
  });
});
