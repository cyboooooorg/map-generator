import { describe, expect, it } from 'vitest';
import { bakeNebula, nebulaTextureSize, type NebulaTexture } from './nebulaTextures';
import type { SkyExposure } from './skyClouds';
import { pillarAt, pillarColumns, pillarDistance, type Nebula, type NebulaKind } from './skyNebulae';

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

  it('sizes the texture so that a texel is about a screen pixel in the planet view, at a pixel ratio of 1', () => {
    // The planet view's field of view is 40 degrees; the canvas we size for is 900 px high.
    const pixelDeg = 40 / 900;
    for (const deg of [0.5, 1, 2, 3, 5, 7, 12]) {
      const size = nebulaTextureSize(deg * DEG);
      expect((2 * deg) / size).toBeLessThanOrEqual(pixelDeg + 1e-9);
      // Not much finer either, above the smallest size: the bake costs by the texel (within one size step of 32).
      if (size > 64) expect((2 * deg) / (size - 32)).toBeGreaterThan(pixelDeg);
      expect(size % 32).toBe(0);
    }
    expect(nebulaTextureSize(0.3 * DEG)).toBe(64);
    expect(nebulaTextureSize(12 * DEG)).toBe(544);
    expect(bakeNebula(make('emission', 1, 0.3, 8), 'eye').size).toBe(nebulaTextureSize(8 * DEG));
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
      // The main columns (the fingers are thin and the torn edge shifts them by about their width).
      for (const p of pillarColumns(n).filter((q) => q.width >= 0.17)) {
        const at = pillarAt(p, -0.45);
        if (!at) continue;
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
        expect(mean(t, strip, light(t))).toBeLessThan(0.4 * mean(t, around, light(t)));
      }
    }
  });

  it('lights the rim of a pillar: brighter than the column and than the background just beyond', () => {
    let lit = 0;
    let total = 0;
    for (const seed of [1, 2, 3, 4242]) {
      const n = make('pillars', seed, 0.3, 3);
      const t = bakeNebula(n, 'photo');
      const columns = pillarColumns(n);
      const main = columns.filter((q) => q.width >= 0.17);
      for (const p of main) {
        // Texels at the tip's outline (|distance| < 0.015), further out (0.1 to 0.2) and inside (below -0.05), around the top half of the column.
        const near = (x: number, y: number) => Math.abs(x - p.x) < 0.25 && y > p.top - 0.25 && y < p.top + 0.1 && Math.hypot(x, y) < 0.7;
        const edge = region(t, (x, y) => near(x, y) && Math.abs(pillarDistance(columns, x, y)) < 0.015);
        const beyond = region(t, (x, y) => near(x, y) && pillarDistance(columns, x, y) > 0.1 && pillarDistance(columns, x, y) < 0.2);
        const inside = region(t, (x, y) => near(x, y) && pillarDistance(columns, x, y) < -0.05);
        if (edge.length === 0 || beyond.length === 0 || inside.length === 0) continue;
        total += 1;
        const e = mean(t, edge, light(t));
        if (e > 1.3 * mean(t, beyond, light(t)) && e > 3 * mean(t, inside, light(t))) lit += 1;
      }
    }
    expect(total).toBeGreaterThan(5);
    expect(lit / total).toBeGreaterThanOrEqual(0.8);
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

  describe('reflection nebulae (the Pleiades family)', () => {
    // A structureless blue blur was the complaint: the dust must show as
    // streaks lit by a bright star at the heart.
    const lum = (t: NebulaTexture, row: number, col: number) => {
      const i = (row * t.size + col) * 4;
      return 0.2126 * decode(t.rgba[i]!) + 0.7152 * decode(t.rgba[i + 1]!) + 0.0722 * decode(t.rgba[i + 2]!);
    };
    /** The haze's annulus about the star, as (row, col) pairs. */
    const annulus = (t: NebulaTexture, inner = 0.2, outer = 0.6) => {
      const out: [number, number][] = [];
      for (let row = 1; row < t.size - 1; row++) {
        for (let col = 1; col < t.size - 1; col++) {
          const r = Math.hypot((2 * (col + 0.5)) / t.size - 1, 1 - (2 * (row + 0.5)) / t.size);
          if (r > inner && r < outer) out.push([row, col]);
        }
      }
      return out;
    };
    const samples = [1, 2, 3, 4, 5, 6].map((seed) => bakeNebula(make('reflection', 900 + seed, (seed % 3) / 2.5, 2.5), 'photo'));

    it('puts a bright, white star at its heart', () => {
      for (const t of samples) {
        // The brightest texel near the centre: as bright as any (others may be lit stars of the cluster).
        let best = 0;
        let heart = 0;
        let at = 0;
        for (let i = 0; i < t.size * t.size; i++) {
          const v = decode(t.rgba[4 * i]!) + decode(t.rgba[4 * i + 1]!) + decode(t.rgba[4 * i + 2]!);
          best = Math.max(best, v);
          const x = (2 * ((i % t.size) + 0.5)) / t.size - 1;
          const y = 1 - (2 * (Math.floor(i / t.size) + 0.5)) / t.size;
          if (Math.hypot(x, y) < 0.1 && v > heart) [heart, at] = [v, i];
        }
        expect(heart).toBeGreaterThanOrEqual(0.98 * best);
        // White: blue not far over red at the star.
        expect(decode(t.rgba[4 * at + 2]!) / Math.max(1e-6, decode(t.rgba[4 * at]!))).toBeLessThan(1.4);
        // Far brighter than the haze round it.
        const haze = annulus(t).map(([r, c]) => lum(t, r, c));
        expect(heart / 3).toBeGreaterThan(4 * (haze.reduce((a, b) => a + b, 0) / haze.length));
      }
    });

    it('draws the haze in streaks: its gradients share a direction', () => {
      // Coherence of the structure tensor over the haze: 0 for isotropic noise or a round blur, 1 for parallel lines.
      let coherent = 0;
      for (const t of samples) {
        let xx = 0;
        let yy = 0;
        let xy = 0;
        for (const [r, c] of annulus(t)) {
          const gx = lum(t, r, c + 1) - lum(t, r, c - 1);
          const gy = lum(t, r + 1, c) - lum(t, r - 1, c);
          xx += gx * gx;
          yy += gy * gy;
          xy += gx * gy;
        }
        const coherence = Math.sqrt((xx - yy) ** 2 + 4 * xy * xy) / (xx + yy);
        if (coherence > 0.3) coherent += 1;
      }
      expect(coherent).toBeGreaterThanOrEqual(5);
    });

    it('has dark gaps between the streaks, not an even glow', () => {
      for (const t of samples) {
        // The haze divided by its mean at each radius: what is left is the structure.
        const bins = new Map<number, number[]>();
        const ring = annulus(t);
        const binOf = ([r, c]: [number, number]) => Math.round(10 * Math.hypot((2 * (c + 0.5)) / t.size - 1, 1 - (2 * (r + 0.5)) / t.size));
        for (const p of ring) {
          const b = binOf(p);
          if (!bins.has(b)) bins.set(b, []);
          bins.get(b)!.push(lum(t, p[0], p[1]));
        }
        const means = new Map([...bins].map(([b, v]) => [b, v.reduce((x, y) => x + y, 0) / v.length]));
        const ratios = ring.map((p) => lum(t, p[0], p[1]) / Math.max(1e-9, means.get(binOf(p))!));
        const m = ratios.reduce((x, y) => x + y, 0) / ratios.length;
        const sd = Math.sqrt(ratios.reduce((x, y) => x + (y - m) ** 2, 0) / ratios.length);
        expect(sd / m).toBeGreaterThan(0.4);
      }
    });

    it('stays blue in the haze', () => {
      for (const t of samples) {
        const at = region(t, (x, y) => Math.hypot(x, y) > 0.25 && Math.hypot(x, y) < 0.55);
        expect(mean(t, at, (i) => decode(t.rgba[i + 2]!))).toBeGreaterThan(1.3 * mean(t, at, (i) => decode(t.rgba[i]!)));
      }
    });
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
