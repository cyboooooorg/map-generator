import { describe, expect, it } from 'vitest';
import type { Vec3 } from '../skyBodies';
import { bakeSkyClouds, CLOUD_CEILING, cloudSampler, dustAt, faceDirection, type SkyExposure, type SkyFaces } from './skyClouds';
import { FIXED_SKY, skyLook, type SkyLook } from './skyLook';

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
  const baked = bakeSkyClouds(look, 'eye', SIZE);

  it('is deterministic and memoised per look', () => {
    expect(bakeSkyClouds({ ...look }, 'eye', SIZE)).toBe(baked);
    expect(baked.faces).toHaveLength(6);
    expect(baked.faces[0]!.length).toBe(SIZE * SIZE * 4);
  });

  it('differs between seeds', () => {
    const other = bakeSkyClouds({ ...look, seed: 99 }, 'eye', SIZE);
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
    bakeSkyClouds({ ...look, seed: 12345 }, 'eye', 256);
    const ms = performance.now() - start;
    console.log(`bakeSkyClouds(256): ${ms.toFixed(0)} ms`);
  });
});

describe('clouds of an outer system', () => {
  it('stay visible far out in the disc', () => {
    const outer = skyLook({ source: 'Derived', position_kpc: [16, 0, 0], axes: [[1, 0, 0], [0, 1, 0], [0, 0, 1]], sky_seed: 5 } as never);
    const faces = bakeSkyClouds(outer, 'eye', SIZE);
    let brightest = 0;
    for (const pixels of faces.faces) {
      for (let i = 0; i < pixels.length; i += 4) brightest = Math.max(brightest, decode(pixels[i]!), decode(pixels[i + 1]!), decode(pixels[i + 2]!));
    }
    expect(brightest).toBeGreaterThanOrEqual(0.02);
    expect(brightest).toBeLessThan(0.08);
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

describe('the galactic core and the two exposures', () => {
  const SIZE_CORE = 128;
  const sunPlace = (position: [number, number, number], seed = 5) =>
    skyLook({
      source: 'Derived',
      position_kpc: position,
      axes: [[1, 0, 0], [0, 1, 0], [0, 0, 1]],
      sky_seed: seed,
    } as never);
  const sun = sunPlace([8.2, 0, 0.02]);
  const eye = bakeSkyClouds(sun, 'eye', SIZE_CORE);
  const photo = bakeSkyClouds(sun, 'photo', SIZE_CORE);

  const dirOf = (face: number, col: number, row: number) =>
    faceDirection(face, (2 * (col + 0.5)) / SIZE_CORE - 1, (2 * (row + 0.5)) / SIZE_CORE - 1);
  const frameOf = (look: SkyLook) => {
    const along = [...look.centre] as Vec3;
    const d = dot(along, look.north);
    const a = along.map((v, i) => v - d * look.north[i]!) as Vec3;
    const l = Math.hypot(...a);
    const u: Vec3 = [a[0] / l, a[1] / l, a[2] / l];
    const n = look.north;
    const across: Vec3 = [n[1] * u[2] - n[2] * u[1], n[2] * u[0] - n[0] * u[2], n[0] * u[1] - n[1] * u[0]];
    return { along: u, across };
  };
  const meanNear = (faces: SkyFaces, target: Vec3, radius: number) =>
    meanOf(faces.faces, (f, c, r) => dot(dirOf(f, c, r), target) > Math.cos(radius), SIZE_CORE);

  it('lights the core at least twice as bright as the band 90 degrees away, in both exposures', () => {
    const { across } = frameOf(sun);
    for (const baked of [eye, photo]) {
      const core = meanNear(baked, sun.centre, sun.coreSize);
      const far = meanNear(baked, across, sun.coreSize);
      console.log(`core/band ratio ${(core / far).toFixed(2)}`);
      expect(core).toBeGreaterThanOrEqual(2 * far);
    }
  });

  it('is brighter and larger close in than far out', () => {
    const near = sunPlace([3, 0, 0.02]);
    const far = sunPlace([14, 0, 0.02]);
    expect(near.coreSize).toBeGreaterThan(far.coreSize);
    expect(near.coreBrightness).toBeGreaterThan(far.coreBrightness);
    expect(near.coreBrightness).toBeCloseTo(4, 9);
    const peak = (look: SkyLook) => meanNear(bakeSkyClouds(look, 'photo', SIZE_CORE), look.centre, 0.1);
    expect(peak(near)).toBeGreaterThan(peak(far));
  });

  it('crosses the core with a dark lane along the plane', () => {
    for (const baked of [eye, photo]) {
      const lat = (f: number, c: number, r: number) => Math.asin(dot(dirOf(f, c, r), sun.north));
      const inside = (f: number, c: number, r: number) => dot(dirOf(f, c, r), sun.centre) > Math.cos(0.4 * sun.coreSize);
      const deg = Math.PI / 180;
      const lane = meanOf(baked.faces, (f, c, r) => inside(f, c, r) && Math.abs(lat(f, c, r)) < 0.8 * deg, SIZE_CORE);
      const above = meanOf(baked.faces, (f, c, r) => inside(f, c, r) && Math.abs(lat(f, c, r) - 3.2 * deg) < 0.8 * deg, SIZE_CORE);
      const below = meanOf(baked.faces, (f, c, r) => inside(f, c, r) && Math.abs(lat(f, c, r) + 3.2 * deg) < 0.8 * deg, SIZE_CORE);
      expect(lane).toBeLessThan(above);
      expect(lane).toBeLessThan(below);
    }
  });

  it('keeps the eye below 0.08 and the photo below 0.35, the eye less saturated', () => {
    const stats = (baked: SkyFaces) => {
      let brightest = 0;
      let saturation = 0;
      let count = 0;
      for (const pixels of baked.faces) {
        for (let i = 0; i < pixels.length; i += 4) {
          const c = [decode(pixels[i]!), decode(pixels[i + 1]!), decode(pixels[i + 2]!)];
          const hi = Math.max(...c);
          brightest = Math.max(brightest, hi);
          if (hi > 0.004) {
            saturation += (hi - Math.min(...c)) / hi;
            count += 1;
          }
        }
      }
      return { brightest, saturation: saturation / count };
    };
    const e = stats(eye);
    const p = stats(photo);
    expect(e.brightest).toBeLessThanOrEqual(0.08);
    expect(p.brightest).toBeLessThanOrEqual(0.35);
    expect(p.brightest).toBeGreaterThan(e.brightest);
    expect(e.saturation).toBeLessThan(p.saturation);
  });

  it('is deterministic per look and exposure, and keeps both exposures apart', () => {
    // Memoised per look and exposure (the cache keeps only a few bakes, so compare fresh ones).
    const again = bakeSkyClouds({ ...sun }, 'photo', SIZE_CORE);
    expect(bakeSkyClouds({ ...sun }, 'photo', SIZE_CORE)).toBe(again);
    expect(again.faces).toEqual(photo.faces);
    expect(bakeSkyClouds({ ...sun }, 'eye', SIZE_CORE).faces).toEqual(eye.faces);
    expect(photo.faces).not.toEqual(eye.faces);
  });
});

describe('the clouds along a curve', () => {
  // A straight edge across the views (a step where the glow was cut at 2.8
  // sigma) must not come back: along great circles that cross the band's
  // edge and the cube's face edges, the colour and its slope stay continuous.
  const look: SkyLook = { ...FIXED_SKY, seed: 5, bulge: 0.4, coreSize: 0.3, coreBrightness: 1 };
  const circles: [Vec3, Vec3][] = [
    [[1, 0, 0], [0, 1, 0]],
    [[0, 0, 1], [0.6, 0.8, 0]],
    [[0.577, 0.577, 0.577], [-0.707, 0.707, 0]],
    [[0, 1, 0], [0, 0, 1]],
  ];
  const STEP = 0.0004;

  for (const exposure of ['eye', 'photo'] as SkyExposure[]) {
    it(`has no step nor kink in the ${exposure} rendering`, () => {
      const sample = cloudSampler(look, exposure);
      let worstStep = 0;
      let worstKink = 0;
      for (const [a, b] of circles) {
        const steps = Math.round((2 * Math.PI) / STEP);
        let previous: Vec3 | null = null;
        let previousDelta: Vec3 | null = null;
        for (let i = 0; i <= steps; i++) {
          const angle = i * STEP;
          const dir: Vec3 = [0, 1, 2].map((k) => Math.cos(angle) * a[k]! + Math.sin(angle) * b[k]!) as Vec3;
          const value = sample(dir);
          if (previous) {
            const delta = value.map((v, k) => v - previous![k]!) as Vec3;
            worstStep = Math.max(worstStep, ...delta.map(Math.abs));
            if (previousDelta) worstKink = Math.max(worstKink, ...delta.map((d, k) => Math.abs(d - previousDelta![k]!)));
            previousDelta = delta;
          }
          previous = value;
        }
      }
      expect(worstStep).toBeLessThan(2.5e-3);
      expect(worstKink).toBeLessThan(1.5e-4);
    });
  }

  it('matches the baked cube texels', () => {
    const faces = bakeSkyClouds(look, 'photo');
    const sample = cloudSampler(look, 'photo');
    const size = faces.size;
    const [row, col] = [40, 90];
    const at = (row * size + col) * 4;
    const linear = sample(faceDirection(2, (2 * (col + 0.5)) / size - 1, (2 * (row + 0.5)) / size - 1));
    const encoded = (v: number) => Math.round(255 * (v <= 0.0031308 ? 12.92 * v : 1.055 * v ** (1 / 2.4) - 0.055));
    expect(faces.faces[2]![at]).toBe(encoded(linear[0]));
  });
});
