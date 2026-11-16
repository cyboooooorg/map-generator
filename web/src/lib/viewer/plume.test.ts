import { describe, expect, it } from 'vitest';
import { PLUME_ALPHA, plumeAlpha, plumeFrame } from './plume';

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
    const north = plumeFrame(AXES, false, 2);
    expect(north.up[1]).toBeCloseTo(1);
    expect(north.height).toBeCloseTo(3);
  });

  it('is brightest back-lit and still seen side-lit', () => {
    // phaseCos -1 is a new moon: the light passes through the plume to the eye.
    expect(plumeAlpha(-1)).toBe(1);
    expect(plumeAlpha(0)).toBeCloseTo(PLUME_ALPHA);
    expect(plumeAlpha(1)).toBeGreaterThan(0.02);
    expect(plumeAlpha(-0.5)).toBeGreaterThan(plumeAlpha(0.5));
  });
});
