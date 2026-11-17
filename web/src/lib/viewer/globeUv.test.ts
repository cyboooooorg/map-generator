import { describe, expect, it } from 'vitest';
import { globeUv, globeUvStep, unwrapNear } from './globeUv';

// The surface vertex shader's sphere point of map position (u, v).
const sphere = (u: number, v: number): [number, number, number] => {
  const lon = u * 2 * Math.PI;
  const lat = (v - 0.5) * Math.PI;
  return [Math.cos(lat) * Math.sin(lon), Math.sin(lat), Math.cos(lat) * Math.cos(lon)];
};

// The step of (u, v) from map position a to b, by the direction's change
// taken at their middle.
const step = (a: [number, number], b: [number, number]) => {
  const p = sphere(...a);
  const q = sphere(...b);
  const m = sphere((a[0] + b[0]) / 2, (a[1] + b[1]) / 2);
  return globeUvStep(...m, q[0] - p[0], q[1] - p[1], q[2] - p[2]);
};

describe('globeUv', () => {
  it('inverts the vertex shader across the map', () => {
    for (let i = 0; i < 40; i++) {
      for (let j = 1; j < 20; j++) {
        const at = globeUv(...sphere(i / 40, j / 20));
        expect(at.u).toBeCloseTo(i / 40, 9);
        expect(at.v).toBeCloseTo(j / 20, 9);
      }
    }
  });

  it('keeps u below 1 just west of the seam', () => {
    const at = globeUv(-1e-20, 0, 1);
    expect(at.u).toBeGreaterThanOrEqual(0);
    expect(at.u).toBeLessThan(1);
  });

  it('reads the poles at v = 0 and 1', () => {
    expect(globeUv(0, 1, 0).v).toBe(1);
    expect(globeUv(0, -1, 0).v).toBe(0);
  });
});

describe('globeUvStep', () => {
  it('matches the change of u and v, across the seam too', () => {
    for (const v of [0.5, 0.8, 0.99, 0.9999]) {
      // Across the seam u jumps by a whole turn; the step does not.
      const [du, dv] = step([-1e-5, v], [1e-5, v + 1e-7]);
      expect(du).toBeCloseTo(2e-5, 9);
      expect(dv / 1e-7).toBeCloseTo(1, 2);
    }
  });

  it('adds up to one turn round a small circle about a pole', () => {
    let total = 0;
    for (let k = 0; k < 256; k++) total += step([k / 256, 0.99999], [(k + 1) / 256, 0.99999])[0];
    expect(total).toBeCloseTo(1, 4);
  });
});

describe('unwrapNear', () => {
  it('moves u to the side of the seam the mesh is on', () => {
    expect(unwrapNear(0.001, 0.999)).toBeCloseTo(1.001, 12);
    expect(unwrapNear(0.999, 0.002)).toBeCloseTo(-0.001, 12);
    expect(unwrapNear(0.4, 0.45)).toBe(0.4);
  });
});
