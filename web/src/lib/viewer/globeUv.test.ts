import { describe, expect, it } from 'vitest';
import { globeUv, seamSafeStep, unwrapNear } from './globeUv';

// The surface vertex shader's sphere point of map position (u, v).
const sphere = (u: number, v: number): [number, number, number] => {
  const lon = u * 2 * Math.PI;
  const lat = (v - 0.5) * Math.PI;
  return [Math.cos(lat) * Math.sin(lon), Math.sin(lat), Math.cos(lat) * Math.cos(lon)];
};

describe('globeUv', () => {
  it('inverts the vertex shader across the map', () => {
    for (let i = 0; i < 40; i++) {
      for (let j = 1; j < 20; j++) {
        const u = i / 40;
        const v = j / 20;
        const at = globeUv(...sphere(u, v));
        expect(at.u).toBeCloseTo(u, 9);
        expect(at.v).toBeCloseTo(v, 9);
        expect(fractDiff(at.uOpposite, u)).toBeCloseTo(0, 9);
      }
    }
  });

  it('reads the poles at v = 0 and 1 whatever the length', () => {
    expect(globeUv(0, 3, 0).v).toBe(1);
    expect(globeUv(0, -0.2, 0).v).toBe(0);
  });

  it('gives a seam-safe step across the seam and near the pole', () => {
    for (const v of [0.5, 0.99, 0.9999]) {
      const a = globeUv(...sphere(-1e-4, v));
      const b = globeUv(...sphere(1e-4, v));
      // u wraps by a whole turn, the opposite longitude does not.
      expect(Math.abs(b.u - a.u)).toBeGreaterThan(0.99);
      expect(seamSafeStep(b.u - a.u, b.uOpposite - a.uOpposite)).toBeCloseTo(2e-4, 9);
    }
    // Opposite the seam, u is the continuous one.
    const a = globeUv(...sphere(0.5 - 1e-4, 0.999));
    const b = globeUv(...sphere(0.5 + 1e-4, 0.999));
    expect(seamSafeStep(b.u - a.u, b.uOpposite - a.uOpposite)).toBeCloseTo(2e-4, 9);
  });

  it('turns all the way round a pole in a small circle', () => {
    // Round a tiny circle about the north pole, u runs through a full turn
    // and stays continuous step by step with the seam-safe derivative.
    let total = 0;
    let previous = globeUv(...sphere(0, 0.99999));
    for (let k = 1; k <= 64; k++) {
      const next = globeUv(...sphere(k / 64, 0.99999));
      total += seamSafeStep(next.u - previous.u, next.uOpposite - previous.uOpposite);
      expect(next.v).toBeCloseTo(0.99999, 9);
      previous = next;
    }
    expect(total).toBeCloseTo(1, 9);
  });
});

describe('unwrapNear', () => {
  it('moves u to the side of the seam the mesh is on', () => {
    expect(unwrapNear(0.001, 0.999)).toBeCloseTo(1.001, 12);
    expect(unwrapNear(0.999, 0.002)).toBeCloseTo(-0.001, 12);
    expect(unwrapNear(0.4, 0.45)).toBe(0.4);
  });
});

// The difference of two longitudes in turns, wrapped to [−0.5, 0.5).
function fractDiff(a: number, b: number): number {
  const d = a - b;
  return d - Math.floor(d + 0.5);
}
