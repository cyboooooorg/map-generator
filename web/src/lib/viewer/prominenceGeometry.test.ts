import { describe, expect, it } from 'vitest';
import { prominenceGeometry, STRANDS } from './prominenceGeometry';
import { prominences } from './starSystem';

describe('prominence geometry', () => {
  const arc = prominences('M', 7)[0]!;
  const geometry = prominenceGeometry(arc, 1, 3);

  it('makes a bundle of strands, each with its own look and fraying direction', () => {
    const strands = new Set(Array.from(geometry.getAttribute('aStrand').array));
    expect(strands.size).toBe(STRANDS);
    const drift = geometry.getAttribute('aDrift');
    for (let i = 0; i < drift.count; i += 997) {
      expect(Math.hypot(drift.getX(i), drift.getY(i), drift.getZ(i))).toBeCloseTo(1, 5);
    }
  });

  it('rises from the surface without dipping far into the star', () => {
    const position = geometry.getAttribute('position');
    let lowest = Infinity;
    let highest = 0;
    for (let i = 0; i < position.count; i += 1) {
      const r = Math.hypot(position.getX(i), position.getY(i), position.getZ(i));
      lowest = Math.min(lowest, r);
      highest = Math.max(highest, r);
    }
    expect(lowest).toBeGreaterThan(0.95);
    expect(highest).toBeGreaterThan(1 + arc.height * 0.7);
  });

  it('is the same for the same seed', () => {
    const again = prominenceGeometry(arc, 1, 3).getAttribute('position').array;
    expect(Array.from(again.slice(0, 30))).toEqual(Array.from(geometry.getAttribute('position').array.slice(0, 30)));
  });
});
