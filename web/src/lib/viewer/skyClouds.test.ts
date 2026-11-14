import { describe, expect, it } from 'vitest';
import type { Vec3 } from '../skyBodies';
import { bakeSkyClouds, CLOUD_CEILING, dustAt, faceDirection } from './skyClouds';
import { FIXED_SKY, type SkyLook } from './skyLook';

const decode = (byte: number) => {
  const v = byte / 255;
  return v <= 0.04045 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4;
};
const dot = (a: Vec3, b: Vec3) => a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
const SIZE = 64;

function meanOf(faces: Uint8Array[], pick: (face: number, col: number, row: number) => boolean, size: number) {
  let sum = 0;
  let count = 0;
  faces.forEach((pixels, face) => {
    for (let row = 0; row < size; row += 1) {
      for (let col = 0; col < size; col += 1) {
        if (!pick(face, col, row)) continue;
        const at = (row * size + col) * 4;
        sum += decode(pixels[at]!) + decode(pixels[at + 1]!) + decode(pixels[at + 2]!);
        count += 1;
      }
    }
  });
  return sum / count;
}

describe('faceDirection', () => {
  it('points the centre of each face along its axis', () => {
    const axes: Vec3[] = [
      [1, 0, 0],
      [-1, 0, 0],
      [0, 1, 0],
      [0, -1, 0],
      [0, 0, 1],
      [0, 0, -1],
    ];
    axes.forEach((axis, face) => {
      expect(dot(faceDirection(face, 0, 0), axis)).toBeCloseTo(1, 9);
      // The texel next to the centre of a 256 face is within a degree.
      expect(dot(faceDirection(face, 1 / 256, 1 / 256), axis)).toBeGreaterThan(0.9999);
    });
  });

  it('puts the face top up and its right edge on the cube map convention', () => {
    // +z face: right is +x, top is +y; +y face: top is -z... (t down = +z).
    expect(faceDirection(4, 1, 0)[0]).toBeGreaterThan(0.5);
    expect(faceDirection(4, 0, -1)[1]).toBeGreaterThan(0.5);
    expect(faceDirection(2, 0, 1)[2]).toBeGreaterThan(0.5);
  });
});

describe('bakeSkyClouds', () => {
  const look: SkyLook = { ...FIXED_SKY, bulge: 0.3 };
  const baked = bakeSkyClouds(look, SIZE);

  it('is deterministic and memoised per look', () => {
    expect(bakeSkyClouds({ ...look }, SIZE)).toBe(baked);
    expect(baked.faces).toHaveLength(6);
    expect(baked.faces[0]!.length).toBe(SIZE * SIZE * 4);
  });

  it('differs between seeds', () => {
    const other = bakeSkyClouds({ ...look, seed: 99 }, SIZE);
    let diff = 0;
    for (let f = 0; f < 6; f += 1) {
      for (let i = 0; i < baked.faces[f]!.length; i += 1) diff += Math.abs(baked.faces[f]![i]! - other.faces[f]![i]!);
    }
    expect(diff / (6 * SIZE * SIZE * 4)).toBeGreaterThan(0.3);
  });

  it('stays faint', () => {
    for (const pixels of baked.faces) {
      for (let i = 0; i < pixels.length; i += 4) {
        for (let c = 0; c < 3; c += 1) expect(decode(pixels[i + c]!)).toBeLessThan(0.08);
      }
    }
    expect(CLOUD_CEILING).toBeLessThan(0.08);
  });

  it('is brighter along the band than at the poles', () => {
    const latOf = (face: number, col: number, row: number) =>
      Math.asin(dot(faceDirection(face, (2 * (col + 0.5)) / SIZE - 1, (2 * (row + 0.5)) / SIZE - 1), look.north));
    const band = meanOf(baked.faces, (f, c, r) => Math.abs(latOf(f, c, r)) < 0.1, SIZE);
    const poles = meanOf(baked.faces, (f, c, r) => Math.abs(latOf(f, c, r)) > 1.1, SIZE);
    expect(band).toBeGreaterThan(poles);
  });

  it('bakes a 256 cube quickly', () => {
    const start = performance.now();
    bakeSkyClouds({ ...look, seed: 12345 }, 256);
    const ms = performance.now() - start;
    console.log(`bakeSkyClouds(256): ${ms.toFixed(0)} ms`);
  });
});

describe('dustAt', () => {
  it('is in [0, 1] and higher in the band than at the poles', () => {
    const north = FIXED_SKY.north;
    // A band direction orthogonal to north, and sample around it.
    const e1: Vec3 = [1, 0, 0];
    const e3: Vec3 = [0, -north[2], north[1]];
    let band = 0;
    let poles = 0;
    const n = 200;
    for (let i = 0; i < n; i += 1) {
      const a = (2 * Math.PI * i) / n;
      const inBand = dustAt(FIXED_SKY, [
        Math.cos(a) * e1[0] + Math.sin(a) * e3[0],
        Math.cos(a) * e1[1] + Math.sin(a) * e3[1],
        Math.cos(a) * e1[2] + Math.sin(a) * e3[2],
      ]);
      const polar = dustAt(FIXED_SKY, [north[0] * 0.99 + 0.1 * Math.cos(a), north[1] * 0.99, north[2] * 0.99]);
      expect(inBand).toBeGreaterThanOrEqual(0);
      expect(inBand).toBeLessThanOrEqual(1);
      band += inBand;
      poles += polar;
    }
    expect(band).toBeGreaterThan(poles);
    expect(band).toBeGreaterThan(0);
  });
});
