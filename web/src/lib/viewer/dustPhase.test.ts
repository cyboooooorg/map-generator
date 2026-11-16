import { describe, expect, it } from 'vitest';
import { cosScatter, DUST_G, dustGain, dustPhaseGlsl } from './dustPhase';

describe('dust phase function', () => {
  it('is 1 at 90° and grows tens of times back-lit', () => {
    expect(dustGain(0)).toBeCloseTo(1, 6);
    expect(dustGain(1)).toBeGreaterThan(30);
    expect(dustGain(-1)).toBeLessThan(0.5);
  });
  it('is back-lit when the sun is behind the dust', () => {
    expect(cosScatter([0, 0, 1], [0, 0, -1])).toBe(1);
  });
  it('the GLSL carries the same constants', () => {
    expect(dustPhaseGlsl).toContain(String(DUST_G));
  });
});
