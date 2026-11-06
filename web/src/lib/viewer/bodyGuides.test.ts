import { describe, expect, it } from 'vitest';
import { hostLineRadii, labelIndex } from './bodyGuides';

describe('hostLineRadii', () => {
  it('runs from the surface to the host, geometrically', () => {
    const radii = hostLineRadii(100, 5);
    expect(radii).toHaveLength(5);
    expect(radii[0]).toBe(1);
    expect(radii[4]).toBeCloseTo(100);
    expect(radii[2]).toBeCloseTo(10);
  });

  it('stays at the surface for a host closer than it', () => {
    expect(hostLineRadii(0.5, 3)).toEqual([1, 1, 1]);
  });
});

describe('labelIndex', () => {
  it('picks the middle of the longest run in view', () => {
    expect(labelIndex([false, true, true, true, false, true])).toBe(2);
    expect(labelIndex([true, false, true, true, true, true])).toBe(3);
  });

  it('is -1 when none is in view', () => {
    expect(labelIndex([false, false])).toBe(-1);
    expect(labelIndex([])).toBe(-1);
  });
});
