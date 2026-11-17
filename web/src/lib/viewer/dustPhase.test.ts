import { describe, expect, it } from 'vitest';
import {
  cosScatter,
  DUST_ALPHA_MAX,
  DUST_G,
  DUST_RING_FLOOR,
  DUST_RING_GAIN_MAX,
  dustAlpha,
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

describe('dust ring opacity', () => {
  it('follows a thin column and tends to its cap along a long one', () => {
    expect(dustAlpha(0)).toBe(0);
    expect(dustAlpha(0.02)).toBeCloseTo(0.02, 3);
    expect(dustAlpha(100)).toBeCloseTo(DUST_ALPHA_MAX, 6);
    expect(dustAlpha(1)).toBeLessThan(DUST_ALPHA_MAX);
    expect(dustAlpha(0.2)).toBeGreaterThan(dustAlpha(0.1));
  });
  it('has the same cap in its GLSL twin', () => {
    expect(dustPhaseGlsl).toContain(DUST_ALPHA_MAX.toFixed(3));
  });
});
