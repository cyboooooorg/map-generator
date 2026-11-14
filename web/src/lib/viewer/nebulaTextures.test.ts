import { describe, expect, it } from 'vitest';
import { bakeNebula, nebulaTextureSize, type NebulaTexture } from './nebulaTextures';
import type { SkyExposure } from './skyClouds';
import { pillarAt, pillarColumns, type Nebula, type NebulaKind } from './skyNebulae';

const DEG = Math.PI / 180;
const KINDS: NebulaKind[] = ['planetary', 'emission', 'pillars', 'remnant', 'reflection', 'dark'];
const make = (kind: NebulaKind, seed = 4242, tint = 0.3, degrees = 2): Nebula => ({
  kind,
  dir: [1, 0, 0],
  radius: degrees * DEG,
  roll: 0,
  seed,
  tint,
});
const decode = (byte: number) => {
  const v = byte / 255;
  return v <= 0.04045 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4;
};

/** Texels whose normalised position (x right, y up, in [-1, 1]) satisfies `pick`. */
function region(t: NebulaTexture, pick: (x: number, y: number) => boolean) {
  const out: number[] = [];
  for (let row = 0; row < t.size; row += 1) {
    for (let col = 0; col < t.size; col += 1) {
      if (pick((2 * (col + 0.5)) / t.size - 1, 1 - (2 * (row + 0.5)) / t.size)) out.push((row * t.size + col) * 4);
    }
  }
  return out;
}
const mean = (_t: NebulaTexture, at: number[], channel: (i: number) => number) =>
  at.reduce((s, i) => s + channel(i), 0) / at.length;
const light = (t: NebulaTexture) => (i: number) => decode(t.rgba[i]!) + decode(t.rgba[i + 1]!) + decode(t.rgba[i + 2]!);
const redShare = (t: NebulaTexture, at: number[]) => {
  let r = 0;
  let all = 0;
  for (const i of at) {
    r += decode(t.rgba[i]!);
    all += decode(t.rgba[i]!) + decode(t.rgba[i + 1]!) + decode(t.rgba[i + 2]!);
  }
  return r / all;
};

describe('bakeNebula', () => {
  it('is deterministic and memoised', () => {
    const a = bakeNebula(make('emission'), 'photo');
    expect(bakeNebula(make('emission'), 'photo')).toBe(a);
    expect(bakeNebula(make('emission', 4243), 'photo').rgba).not.toEqual(a.rgba);
    expect(bakeNebula(make('emission'), 'eye').rgba).not.toEqual(a.rgba);
  });

  it('sizes the texture so that a texel is no wider than a screen pixel in the planet view', () => {
    // The planet view's field of view is 40 degrees; the buffer we plan for is 1800 px high.
    const pixelDeg = 40 / 1800;
    for (const deg of [0.3, 0.5, 1, 2, 3, 5, 7]) {
      const size = nebulaTextureSize(deg * DEG);
      expect((2 * deg) / size).toBeLessThanOrEqual(pixelDeg + 1e-9);
      expect(size).toBeGreaterThanOrEqual(128);
      expect(size).toBeLessThanOrEqual(1024);
    }
    expect(nebulaTextureSize(0.3 * DEG)).toBe(128);
    expect(nebulaTextureSize(2 * DEG)).toBe(256);
    // Capped: the largest nebulae are slightly magnified rather than costing 4 MB per texel row.
    expect(nebulaTextureSize(12 * DEG)).toBe(1024);
    expect(bakeNebula(make('emission', 1, 0.3, 8), 'eye').size).toBe(1024);
    const t = bakeNebula(make('planetary', 1, 0.3, 0.5), 'eye');
    expect(t.rgba.length).toBe(t.size * t.size * 4);
  });

  it('uses darken for dark nebulae and add for the others', () => {
    expect(bakeNebula(make('dark'), 'photo').blend).toBe('darken');
    for (const k of KINDS.filter((x) => x !== 'dark')) expect(bakeNebula(make(k), 'photo').blend).toBe('add');
  });

  it('draws a planetary ring: a dim centre, a red rim against a blue-green core', () => {
    const t = bakeNebula(make('planetary', 4242, 0.2), 'photo');
    const inner = region(t, (x, y) => Math.hypot(x, y) < 0.2);
    const rim = region(t, (x, y) => Math.abs(Math.hypot(x, y * 1.1) - 0.5) < 0.1);
    expect(mean(t, inner, light(t))).toBeLessThan(mean(t, rim, light(t)));
    const core = region(t, (x, y) => Math.hypot(x, y) > 0.08 && Math.hypot(x, y) < 0.3);
    const outer = region(t, (x, y) => Math.abs(Math.hypot(x, y * 1.1) - 0.58) < 0.06);
    expect(redShare(t, outer)).toBeGreaterThan(redShare(t, core));
  });

  it('draws two lobes for a high tint', () => {
    const t = bakeNebula(make('planetary', 4242, 0.9), 'photo');
    const lobes = region(t, (x, y) => Math.abs(x) < 0.4 && Math.abs(y) > 0.2 && Math.abs(y) < 0.7);
    const sides = region(t, (x, y) => Math.abs(x) > 0.55 && Math.abs(y) < 0.3);
    expect(mean(t, lobes, light(t))).toBeGreaterThan(2 * mean(t, sides, light(t)));
  });

  it('draws pillars as dark columns against a lit background', () => {
    for (const seed of [1, 2, 3, 4242]) {
      const n = make('pillars', seed, 0.3, 3);
      const t = bakeNebula(n, 'photo');
      for (const p of pillarColumns(n)) {
        const at = pillarAt(p, -0.45)!;
        const strip = region(t, (x, y) => Math.abs(x - at.cx) < 0.4 * at.half && Math.abs(y + 0.45) < 0.1);
        const around = region(
          t,
          (x, y) =>
            Math.abs(Math.abs(x - at.cx) - 2.2 * at.half) < 0.8 * at.half &&
            Math.abs(y + 0.45) < 0.1 &&
            Math.hypot(x, y) < 0.75 &&
            pillarColumns(n).every((q) => {
              const a = pillarAt(q, y);
              return !a || Math.abs(x - a.cx) > a.half * 1.5;
            }),
        );
        if (around.length === 0) continue;
        expect(mean(t, strip, light(t))).toBeLessThan(0.25 * mean(t, around, light(t)));
      }
    }
  });

  it('draws an emission complex with clear dark lanes and a bright glow', () => {
    for (const seed of [1, 2, 3, 4242]) {
      const t = bakeNebula(make('emission', seed, 0.3, 6), 'photo');
      const disc = region(t, (x, y) => Math.hypot(x, y) < 0.55).map(light(t));
      const sorted = [...disc].sort((a, b) => a - b);
      const bright = sorted[Math.floor(0.9 * sorted.length)]!;
      const dark = disc.filter((v) => v < 0.15 * bright).length / disc.length;
      // A real share of the complex is lane, and the glow is much brighter than that.
      expect(dark).toBeGreaterThan(0.05);
      expect(dark).toBeLessThan(0.6);
      expect(bright).toBeGreaterThan(0.1);
    }
  });

  it('draws a remnant as a thin shell, far brighter than its inside and its surroundings', () => {
    const t = bakeNebula(make('remnant', 4242, 0.3, 5), 'photo');
    const shell = region(t, (x, y) => Math.abs(Math.hypot(x, y) - 0.68) < 0.1);
    const inside = region(t, (x, y) => Math.hypot(x, y) < 0.3);
    const outside = region(t, (x, y) => Math.hypot(x, y) > 0.82 && Math.hypot(x, y) < 0.88);
    expect(mean(t, shell, light(t))).toBeGreaterThan(3 * mean(t, inside, light(t)));
    expect(mean(t, shell, light(t))).toBeGreaterThan(3 * mean(t, outside, light(t)));
  });

  it('draws the planetary ring bright against its background, with clear red and blue-green', () => {
    const t = bakeNebula(make('planetary', 4242, 0.2, 1), 'photo');
    const ring = region(t, (x, y) => Math.abs(Math.hypot(x, y * 1.1) - 0.5) < 0.07).map(light(t));
    const peak = Math.max(...ring);
    expect(peak).toBeGreaterThan(0.25);
    const outside = region(t, (x, y) => Math.hypot(x, y) > 0.8 && Math.hypot(x, y) < 0.88);
    expect(mean(t, outside, light(t))).toBeLessThan(0.05 * peak);
  });

  it('has a long exposure with deeper reds and blue-greens than the sum of its parts', () => {
    // Pushed saturation: the red rim of a ring is nearly pure red in the long exposure.
    const t = bakeNebula(make('planetary', 4242, 0.2, 1), 'photo');
    const rim = region(t, (x, y) => Math.abs(Math.hypot(x, y * 1.1) - 0.58) < 0.04);
    expect(redShare(t, rim)).toBeGreaterThan(0.6);
  });

  it('keeps the eye under the 0.08 ceiling and less saturated than the long exposure', () => {
    const saturation = (t: NebulaTexture, ceiling = 1) => {
      let sum = 0;
      let count = 0;
      for (let i = 0; i < t.rgba.length; i += 4) {
        const c = [decode(t.rgba[i]!), decode(t.rgba[i + 1]!), decode(t.rgba[i + 2]!)];
        const hi = Math.max(...c);
        expect(hi).toBeLessThan(ceiling);
        if (hi > 0.003) {
          sum += (hi - Math.min(...c)) / hi;
          count += 1;
        }
      }
      return sum / count;
    };
    const photoMax = (t: NebulaTexture) => {
      let hi = 0;
      for (let i = 0; i < t.rgba.length; i += 4) hi = Math.max(hi, decode(t.rgba[i]!), decode(t.rgba[i + 1]!), decode(t.rgba[i + 2]!));
      return hi;
    };
    for (const k of KINDS.filter((x) => x !== 'dark')) {
      const n = make(k);
      expect(saturation(bakeNebula(n, 'eye'), 0.08)).toBeLessThan(saturation(bakeNebula(n, 'photo')));
      expect(photoMax(bakeNebula(n, 'photo'))).toBeLessThanOrEqual(0.35);
    }
  });

  it('fades to nothing at the border, for every kind and exposure', () => {
    for (const k of KINDS) {
      for (const e of ['eye', 'photo'] as SkyExposure[]) {
        const t = bakeNebula(make(k, 99, 0.5, k === 'emission' ? 8 : 1), e);
        const edge = Math.floor(0.05 * t.size);
        for (let row = 0; row < t.size; row += 1) {
          for (let col = 0; col < t.size; col += 1) {
            if (row < edge || row >= t.size - edge || col < edge || col >= t.size - edge) {
              expect(t.rgba[(row * t.size + col) * 4 + 3]).toBe(0);
            }
          }
        }
      }
    }
  });

  it('keeps colour at or under alpha (premultiplied)', () => {
    for (const k of KINDS) {
      const t = bakeNebula(make(k), 'photo');
      for (let i = 0; i < t.rgba.length; i += 4) {
        for (let c = 0; c < 3; c += 1) expect(t.rgba[i + c]!).toBeLessThanOrEqual(t.rgba[i + 3]!);
      }
    }
  });

  it('has visible content in every kind', () => {
    for (const k of KINDS) {
      const t = bakeNebula(make(k), 'photo');
      let alpha = 0;
      for (let i = 3; i < t.rgba.length; i += 4) alpha += t.rgba[i]!;
      expect(alpha / (t.size * t.size)).toBeGreaterThan(2);
    }
  });

  it('logs the bake times', () => {
    const lines: string[] = [];
    for (const [k, deg] of [['planetary', 1], ['emission', 8], ['pillars', 3], ['remnant', 5], ['reflection', 2], ['dark', 3]] as [NebulaKind, number][]) {
      const start = performance.now();
      const t = bakeNebula(make(k, 777, 0.4, deg), 'photo');
      lines.push(`${k} ${t.size}px ${(performance.now() - start).toFixed(0)} ms`);
    }
    console.log(`bakeNebula: ${lines.join(', ')}`);
  });
});
