import { DataUtils } from 'three';
import { describe, expect, it } from 'vitest';
import {
  grazingMu,
  SUN_DEPTH_HEIGHT,
  SUN_DEPTH_WIDTH,
  sunColumnFactor,
  sunDepthAt,
  sunDepthTable,
} from './sunDepth';

/** Column of exp(-(r - 1) / H) to the sphere `top`, by fine numerical integration, over the density at r. */
function marched(r: number, mu: number, H: number, top: number): number {
  const b = r * mu;
  const length = -b + Math.sqrt(b * b - (r * r - top * top));
  const steps = 4000;
  const ds = length / steps;
  let sum = 0;
  for (let i = 0; i < steps; i += 1) {
    const t = (i + 0.5) * ds;
    const radius = Math.sqrt(r * r + 2 * r * mu * t + t * t);
    sum += Math.exp(-(radius - r) / H) * ds;
  }
  return sum;
}

/** Shells of sky.ts: thin (Mars), Earth, dense (Venus), and Earth lifted by the relief. */
const SHELLS = [0.03, 0.036, 0.069, 0.096].map((thickness) => ({
  top: 1 + thickness,
  heights: [0.25 * thickness, 0.12 * thickness],
}));

describe('sunColumnFactor', () => {
  it('follows a numerical integration within 2.5 %', () => {
    for (const { top, heights } of SHELLS) {
      for (const H of heights) {
        for (const share of [0, 0.2, 0.5]) {
          const r = 1 + share * (top - 1);
          for (const mu of [1, 0.6, 0.3, 0.1, 0.03, 0, -0.03, -0.1, -0.2]) {
            if (mu < grazingMu(r)) continue;
            const expected = marched(r, mu, H, top);
            expect(Math.abs(sunColumnFactor(r, mu, H, top) / expected - 1)).toBeLessThan(0.025);
          }
        }
      }
    }
  });

  it('is about one scale height straight up from the ground', () => {
    expect(sunColumnFactor(1, 1, 0.01, 1.04)).toBeCloseTo(0.01, 3);
  });
});

describe('sunDepthTable', () => {
  it('maps the corners to the grazing ray and the zenith, from the ground to the top', () => {
    expect(sunDepthAt(0, 0, 1.04).r).toBe(1);
    expect(sunDepthAt(0, 0, 1.04).mu).toBeCloseTo(0);
    expect(sunDepthAt(1, 1, 1.04).r).toBeCloseTo(1.04);
    expect(sunDepthAt(1, 1, 1.04).mu).toBeCloseTo(1);
    expect(sunDepthAt(0, 1, 1.04).mu).toBeCloseTo(grazingMu(1.04));
  });

  it('interpolates within 3 % of the direct value between texels', () => {
    const { top, heights } = SHELLS[1]!;
    const [heightR, heightM] = heights as [number, number];
    const table = sunDepthTable(top, heightR, heightM);
    expect(table.length).toBe(SUN_DEPTH_WIDTH * SUN_DEPTH_HEIGHT * 2);
    const texel = (i: number, j: number, c: number) => DataUtils.fromHalfFloat(table[(j * SUN_DEPTH_WIDTH + i) * 2 + c]!);
    for (const [u, v] of [
      [0.3, 0.3],
      [0.05, 0.1],
      [0.5, 0.02],
      [0.8, 0.6],
    ] as const) {
      const x = u * (SUN_DEPTH_WIDTH - 1);
      const y = v * (SUN_DEPTH_HEIGHT - 1);
      const i = Math.floor(x);
      const j = Math.floor(y);
      const fx = x - i;
      const fy = y - j;
      for (const [c, H] of [
        [0, heightR],
        [1, heightM],
      ] as const) {
        const value =
          (texel(i, j, c) * (1 - fx) + texel(i + 1, j, c) * fx) * (1 - fy) +
          (texel(i, j + 1, c) * (1 - fx) + texel(i + 1, j + 1, c) * fx) * fy;
        const { r, mu } = sunDepthAt(u, v, top);
        expect(Math.abs(value / sunColumnFactor(r, mu, H, top) - 1)).toBeLessThan(0.03);
      }
    }
  });
});
