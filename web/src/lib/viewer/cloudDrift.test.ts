import { describe, expect, it } from 'vitest';
import { advanceDrift, NO_DRIFT } from './cloudDrift';

describe('advanceDrift', () => {
  it('turns the cloud map and ages the bands', () => {
    const d = advanceDrift(NO_DRIFT, 30, 300);
    expect(d.shift).toBeCloseTo(0.1);
    expect(d.bandTime).toBe(30);
  });

  it('keeps the cloud map within one turn and lets the bands age on', () => {
    const d = advanceDrift({ shift: 0.95, bandTime: 100 }, 30, 300);
    expect(d.shift).toBeCloseTo(0.05);
    expect(d.bandTime).toBe(130);
  });

  it('starts again from the generated look', () => {
    // What resetDrift puts back: storms and great spot included, as they
    // drift with bandTime.
    expect(NO_DRIFT).toEqual({ shift: 0, bandTime: 0 });
  });
});
