import { describe, expect, it } from 'vitest';
import { PREVIEW_EDGES, previewColourAt, previewEdges, previewEdgeUniform, previewTexture } from './previewTexture';

describe('previewColourAt', () => {
  it('is the base colour on a plain body', () => {
    expect(previewColourAt({ colours: [[0.2, 0.3, 0.4]], bands: 0, edges: [], glow: false }, 1, 0, 0)).toEqual([0.2, 0.3, 0.4]);
  });

  it('draws a giant between the edges of the bands it will be generated with', () => {
    const giant = {
      colours: [
        [1, 0, 0],
        [0, 0, 1],
        [0, 1, 0],
      ] as [number, number, number][],
      bands: 3,
      edges: [-60, -20, 20, 60],
      glow: false,
    };
    const at = (deg: number) => {
      const lat = (deg * Math.PI) / 180;
      // Longitude 0 puts the wavy edge's phase at its centre line.
      return previewColourAt(giant, 0, Math.sin(lat), Math.cos(lat));
    };
    // The equatorial zone in the zones' colour, the bands beside it belts.
    expect(at(0)[0]).toBeGreaterThan(0.9);
    expect(at(40)[2]).toBeGreaterThan(0.9);
    expect(at(-40)[2]).toBeGreaterThan(0.9);
    // The polar regions beyond the outer edges.
    expect(at(80)[1]).toBeGreaterThan(0.9);
    expect(previewColourAt(giant, 0, 1, 0)).toEqual([0, 1, 0]);
  });

  it('spaces the bands of a preview with no edges evenly', () => {
    expect(previewEdges({ colours: [], bands: 4, edges: [], glow: false })).toEqual([-64, -32, 0, 32, 64]);
    expect(previewEdges({ colours: [], bands: 0, edges: [], glow: false })).toEqual([]);
    const padded = previewEdgeUniform({ colours: [], bands: 2, edges: [-50, 0, 50], glow: false });
    expect(padded).toHaveLength(PREVIEW_EDGES);
    expect(padded.slice(0, 4)).toEqual([-50, 0, 50, 50]);
  });

  it('caps a rocky body with its third colour', () => {
    const rocky = {
      colours: [
        [0.1, 0.2, 0.5],
        [0.3, 0.3, 0.2],
        [0.9, 0.9, 0.9],
      ] as [number, number, number][],
      bands: 0,
      edges: [],
      glow: false,
    };
    const pole = previewColourAt(rocky, 0, -1, 0);
    for (const c of pole) expect(c).toBeCloseTo(0.9);
  });
});

describe('previewTexture', () => {
  it('fills an RGBA image of the size asked, opaque, in sRGB', () => {
    const data = previewTexture({ colours: [[1, 1, 1]], bands: 0, edges: [], glow: false }, 8, 4);
    expect(data.length).toBe(8 * 4 * 4);
    expect([...data.slice(0, 4)]).toEqual([255, 255, 255, 255]);
    const grey = previewTexture({ colours: [[0.214, 0.214, 0.214]], bands: 0, edges: [], glow: false }, 2, 2);
    expect(grey[0]).toBeGreaterThan(125);
    expect(grey[0]).toBeLessThan(130);
  });
});
