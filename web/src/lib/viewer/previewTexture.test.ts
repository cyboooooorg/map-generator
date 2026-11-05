import { describe, expect, it } from 'vitest';
import { previewColourAt, previewTexture } from './previewTexture';

describe('previewColourAt', () => {
  it('is the base colour on a plain body', () => {
    expect(previewColourAt({ colours: [[0.2, 0.3, 0.4]], bands: 0, glow: false }, 1, 0, 0)).toEqual([0.2, 0.3, 0.4]);
  });

  it('gives a giant belts and zones, and its poles their own colour', () => {
    const giant = {
      colours: [
        [1, 0, 0],
        [0, 0, 1],
        [0, 1, 0],
      ] as [number, number, number][],
      bands: 12,
      glow: false,
    };
    const along = [0, 0.1, 0.2, 0.3, 0.4, 0.5].map((lat) => previewColourAt(giant, 0, Math.sin(lat), Math.cos(lat)));
    // Some latitudes lean to the belts' colour, others to the zones'.
    expect(along.some((c) => c[0] > 0.7)).toBe(true);
    expect(along.some((c) => c[2] > 0.7)).toBe(true);
    expect(previewColourAt(giant, 0, 1, 0)).toEqual([0, 1, 0]);
  });

  it('caps a rocky body with its third colour', () => {
    const rocky = {
      colours: [
        [0.1, 0.2, 0.5],
        [0.3, 0.3, 0.2],
        [0.9, 0.9, 0.9],
      ] as [number, number, number][],
      bands: 0,
      glow: false,
    };
    const pole = previewColourAt(rocky, 0, -1, 0);
    for (const c of pole) expect(c).toBeCloseTo(0.9);
  });
});

describe('previewTexture', () => {
  it('fills an RGBA image of the size asked, opaque, in sRGB', () => {
    const data = previewTexture({ colours: [[1, 1, 1]], bands: 0, glow: false }, 8, 4);
    expect(data.length).toBe(8 * 4 * 4);
    expect([...data.slice(0, 4)]).toEqual([255, 255, 255, 255]);
    const grey = previewTexture({ colours: [[0.214, 0.214, 0.214]], bands: 0, glow: false }, 2, 2);
    expect(grey[0]).toBeGreaterThan(125);
    expect(grey[0]).toBeLessThan(130);
  });
});
