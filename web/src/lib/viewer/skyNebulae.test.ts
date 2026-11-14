import { describe, expect, it } from 'vitest';
import type { Vec3 } from '../skyBodies';
import { dustAt } from './skyClouds';
import { FIXED_SKY, skyLook, type SkyLook } from './skyLook';
import {
  NEBULA_MARGIN_DEG,
  NEBULA_SIZE_DEG,
  nebulaBasis,
  pillarColumns,
  pillarMask,
  darkOpacity,
  skyNebulae,
  type NebulaKind,
} from './skyNebulae';

const KINDS: NebulaKind[] = ['planetary', 'emission', 'pillars', 'remnant', 'reflection', 'dark'];
const DEG = Math.PI / 180;
const dot = (a: Vec3, b: Vec3) => a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
const lookAt = (x: number, seed = 5): SkyLook =>
  skyLook({
    source: 'Derived',
    position_kpc: [x, 0, 0.02],
    axes: [[1, 0, 0], [0, 1, 0], [0, 0, 1]],
    sky_seed: seed,
  } as never);
const latOf = (look: SkyLook, dir: Vec3) => Math.asin(dot(dir, look.north));
const count = (look: SkyLook, kind: NebulaKind) => skyNebulae(look).filter((n) => n.kind === kind).length;

describe('skyNebulae', () => {
  const sun = lookAt(8.2);

  it('is deterministic and memoised per look', () => {
    expect(skyNebulae(sun)).toBe(skyNebulae(sun));
    expect(skyNebulae({ ...sun })).toBe(skyNebulae(sun));
    expect(skyNebulae(lookAt(8.2))).toEqual(skyNebulae(sun));
  });

  it('differs between sky seeds', () => {
    expect(skyNebulae(lookAt(8.2, 6))).not.toEqual(skyNebulae(sun));
  });

  it('draws every kind for at least 80% of 200 seeds', () => {
    const seen = new Map<NebulaKind, number>(KINDS.map((k) => [k, 0]));
    for (let seed = 1; seed <= 200; seed += 1) {
      const kinds = new Set(skyNebulae(lookAt(8.2, seed * 7919)).map((n) => n.kind));
      for (const k of kinds) seen.set(k, seen.get(k)! + 1);
    }
    for (const k of KINDS) expect(seen.get(k)!).toBeGreaterThanOrEqual(160);
  });

  it("draws the counts of the spec's starting point at the Sun", () => {
    const ranges: Record<NebulaKind, [number, number]> = {
      planetary: [3, 8],
      emission: [2, 5],
      pillars: [1, 3],
      remnant: [1, 3],
      reflection: [2, 6],
      dark: [4, 10],
    };
    for (let seed = 1; seed <= 50; seed += 1) {
      const look = lookAt(8.2, seed);
      for (const k of KINDS) {
        expect(count(look, k)).toBeGreaterThanOrEqual(ranges[k][0]);
        expect(count(look, k)).toBeLessThanOrEqual(ranges[k][1]);
      }
    }
  });

  it('keeps the sizes within the table and the vectors unit', () => {
    for (let seed = 1; seed <= 30; seed += 1) {
      for (const n of skyNebulae(lookAt(8.2, seed))) {
        const [lo, hi] = NEBULA_SIZE_DEG[n.kind];
        expect(n.radius).toBeGreaterThanOrEqual(lo * DEG - 1e-9);
        expect(n.radius).toBeLessThanOrEqual(hi * DEG + 1e-9);
        expect(Math.hypot(...n.dir)).toBeCloseTo(1, 9);
        expect(n.tint).toBeGreaterThanOrEqual(0);
        expect(n.tint).toBeLessThanOrEqual(1);
      }
    }
  });

  it('keeps every kind closer to the plane than a uniform sphere on average, most dark ones within 30 degrees', () => {
    // Mean |latitude| of a uniform sphere is (pi / 2 - 1) radians.
    const uniform = Math.PI / 2 - 1;
    for (const k of KINDS) {
      let sum = 0;
      let total = 0;
      let near = 0;
      for (let seed = 1; seed <= 100; seed += 1) {
        const look = lookAt(8.2, seed);
        for (const n of skyNebulae(look).filter((x) => x.kind === k)) {
          const lat = Math.abs(latOf(look, n.dir));
          if (lat <= 30 * DEG) near += 1;
          sum += lat;
          total += 1;
        }
      }
      expect(sum / total).toBeLessThan(uniform);
      if (k === 'dark') expect(near / total).toBeGreaterThan(0.7);
    }
  });

  it('spreads the nebulae over the sphere, not along a thin band', () => {
    let beyond20 = 0;
    let beyond50 = 0;
    let total = 0;
    for (let seed = 1; seed <= 100; seed += 1) {
      const look = lookAt(8.2, seed);
      for (const n of skyNebulae(look)) {
        const lat = Math.abs(latOf(look, n.dir));
        total += 1;
        if (lat > 20 * DEG) beyond20 += 1;
        if (lat > 50 * DEG) beyond50 += 1;
      }
    }
    // A sphere has 66% of its area beyond 20 degrees of a great circle and 23% beyond 50.
    expect(beyond20 / total).toBeGreaterThan(0.3);
    expect(beyond50 / total).toBeGreaterThan(0.05);
  });

  it('keeps two nebulae apart: their radii plus the margin between the centres', () => {
    for (const x of [3, 8.2, 14]) {
      for (let seed = 1; seed <= 60; seed += 1) {
        const list = skyNebulae(lookAt(x, seed));
        for (let i = 0; i < list.length; i += 1) {
          for (let j = i + 1; j < list.length; j += 1) {
            const angle = Math.acos(Math.min(1, dot(list[i]!.dir, list[j]!.dir)));
            expect(angle).toBeGreaterThanOrEqual(list[i]!.radius + list[j]!.radius + NEBULA_MARGIN_DEG * DEG - 1e-9);
          }
        }
      }
    }
  });

  it('draws more nebulae near the centre than far out', () => {
    let near = 0;
    let far = 0;
    for (let seed = 1; seed <= 20; seed += 1) {
      near += skyNebulae(lookAt(3, seed)).length;
      far += skyNebulae(lookAt(14, seed)).length;
    }
    expect(near).toBeGreaterThan(far);
  });

  it('draws each kind from its own stream, whatever the others hold', () => {
    // The dark nebulae of a seed are the same whichever look scale draws the others.
    const a = skyNebulae(sun).filter((n) => n.kind === 'dark');
    const b = skyNebulae({ ...sun }).filter((n) => n.kind === 'dark');
    expect(a).toEqual(b);
    expect(skyNebulae(lookAt(8.2, 9)).filter((n) => n.kind === 'dark')).not.toEqual(a);
  });

  it('works for the fixed sky', () => {
    expect(skyNebulae(FIXED_SKY).length).toBeGreaterThan(0);
  });
});

describe('dark nebulae', () => {
  const look = lookAt(8.2);
  const darks = (seed: number) => skyNebulae(lookAt(8.2, seed)).filter((n) => n.kind === 'dark');

  it('are irregular: elongated, not round discs', () => {
    let elongated = 0;
    let total = 0;
    for (let seed = 1; seed <= 20; seed += 1) {
      for (const n of darks(seed)) {
        // Extent of the dense part along each axis of the texture.
        let wide = 0;
        let tall = 0;
        for (let k = -45; k <= 45; k += 1) {
          if (darkOpacity(n, k / 50, 0) > 0.4) wide += 1;
          if (darkOpacity(n, 0, k / 50) > 0.4) tall += 1;
        }
        total += 1;
        if (wide >= 1.6 * Math.max(1, tall)) elongated += 1;
      }
    }
    // The edge is torn at three scales now, so a few come out rounder.
    expect(elongated / total).toBeGreaterThan(0.55);
  });

  it('have ragged edges: the outline is not a smooth circle', () => {
    for (const n of darks(3)) {
      // The radius of the 0.4 contour along 24 directions varies by more than 25%.
      const radii: number[] = [];
      for (let k = 0; k < 24; k += 1) {
        const a = (k / 24) * 2 * Math.PI;
        let r = 0;
        for (let step = 0; step <= 90; step += 1) if (darkOpacity(n, Math.cos(a) * step / 100, Math.sin(a) * step / 100) > 0.4) r = step / 100;
        radii.push(r);
      }
      expect(Math.max(...radii) / Math.max(1e-3, Math.min(...radii.filter((r) => r > 0), 1))).toBeGreaterThan(1.25);
    }
  });

  it('lie along the band: their long axis follows the plane', () => {
    for (let seed = 1; seed <= 10; seed += 1) {
      const lk = lookAt(8.2, seed);
      for (const n of skyNebulae(lk).filter((x) => x.kind === 'dark')) {
        const { right } = nebulaBasis(n);
        const tangent = [
          lk.north[1] * n.dir[2] - lk.north[2] * n.dir[1],
          lk.north[2] * n.dir[0] - lk.north[0] * n.dir[2],
          lk.north[0] * n.dir[1] - lk.north[1] * n.dir[0],
        ];
        const l = Math.hypot(...tangent);
        if (l < 0.2) continue;
        const along = Math.abs(dot(right, tangent as Vec3)) / l;
        // Within the roll's jitter (0.25 rad) of the plane's direction.
        expect(along).toBeGreaterThan(Math.cos(0.3));
      }
    }
    expect(look.bandWidth).toBeGreaterThan(0);
  });
});

describe('dust of nebulae', () => {
  const look = lookAt(8.2);

  it('is higher at the centre of a dark nebula than just outside it', () => {
    const dark = skyNebulae(look).find((n) => n.kind === 'dark')!;
    const { right } = nebulaBasis(dark);
    const at = (angle: number): Vec3 => {
      const d = dark.dir.map((v, i) => v * Math.cos(angle) + right[i]! * Math.sin(angle)) as Vec3;
      return d;
    };
    expect(dustAt(look, dark.dir)).toBeGreaterThan(dustAt(look, at(dark.radius * 1.3)));
    expect(dustAt(look, dark.dir)).toBeGreaterThan(0.5);
  });

  it('dims the stars behind a pillar column', () => {
    const p = skyNebulae(look).find((n) => n.kind === 'pillars')!;
    const column = pillarColumns(p)[0]!;
    const { right, up } = nebulaBasis(p);
    const t = Math.tan(p.radius);
    const at = (x: number, y: number): Vec3 => {
      const v = p.dir.map((c, i) => c + t * (x * right[i]! + y * up[i]!)) as Vec3;
      const l = Math.hypot(...v);
      return [v[0] / l, v[1] / l, v[2] / l];
    };
    const inside = dustAt(look, at(column.x, -0.6));
    const outside = dustAt(look, at(column.x + 0.9, -0.6));
    expect(pillarMask(pillarColumns(p), column.x, -0.6)).toBeGreaterThan(0.9);
    // The pillar's own extinction (0.75 at most), whatever rift crosses beside it.
    expect(inside).toBeGreaterThan(0.6);
    expect(outside).toBeLessThan(inside + 0.2);
  });
});
