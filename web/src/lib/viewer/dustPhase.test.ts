import { describe, expect, it } from 'vitest';
import {
  cosScatter,
  DUST_G,
  DUST_RING_FLOOR,
  DUST_RING_GAIN_MAX,
  dustGain,
  dustPhaseGlsl,
  dustRingGain,
} from './dustPhase';

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
    expect(dustPhaseGlsl).toContain(DUST_RING_GAIN_MAX.toFixed(1));
  });
});

describe('dust ring gain', () => {
  it('stays seen front-lit and only brightens a little back-lit', () => {
    expect(dustRingGain(-1)).toBe(DUST_RING_FLOOR);
    expect(dustRingGain(0)).toBeCloseTo(1, 6);
    expect(dustRingGain(1)).toBe(DUST_RING_GAIN_MAX);
    expect(dustRingGain(1) / dustRingGain(0)).toBeLessThanOrEqual(2);
  });
  it('never falls as the light comes round behind', () => {
    let last = 0;
    for (let c = -1; c <= 1; c += 0.05) {
      expect(dustRingGain(c)).toBeGreaterThanOrEqual(last);
      last = dustRingGain(c);
    }
  });
});
