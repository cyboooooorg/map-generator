import { describe, expect, it } from 'vitest';
import type { StarActivity } from '../api/types/StarActivity';
import { activeRegions } from '../starActivity';
import { starSurfaceFragment, starSurfaceVertex } from './starShaders';
import { SUN_SYSTEM } from './starSystem';
import {
  exposureFade,
  STAR_SURFACE_GLSL,
  StarSurfaceState,
  starLook,
  starSurfaceUniforms,
  surfaceDetail,
} from './starSurface';

/** A G2V star's activity, as activity.rs derives it (rounded). */
const SUN_ACTIVITY: StarActivity = {
  envelope: 'convective',
  age_gyr: 4.6,
  rotation_days: 25.4,
  shear: 0.2,
  turnover_days: 13.8,
  rossby: 1.84,
  activity: 2e-4,
  cycle_years: 11.1,
  cycle_phase: 0.3,
  cycle_depth: 0.9,
  band_deg: [28, 7],
  spot_coverage: 0.003,
  facula_ratio: 10,
  region_rate: 1.03,
  region_area: 152e-6,
  region_sigma: 0.95,
  decay_per_day: 10e-6,
  granule_size: 1.3 / 695.7,
  granule_contrast: 0.15,
  granule_lifetime_days: 8 / 1440,
  limb: [0.462, 0.258],
  seed: 12345,
};

/** A view where the star is large and the clock slow: every detail on. */
const BIG = { radiusPx: 4000, daysPerSecond: 2 ** -11 };

describe('star surface', () => {
  it('fades a long exposure', () => {
    expect(exposureFade(0.001, 0.01)).toBe(1);
    // System view at 2^12 d/s: spots smear out.
    expect(exposureFade(4096 / 30, 25 / 12)).toBeLessThan(0.15);
    // Star view at its slowest: granules boil.
    expect(exposureFade(2 ** -11 / 30, 8 / 1440)).toBe(1);
  });

  it('drops detail on a small star', () => {
    expect(surfaceDetail(5, 0.002)).toEqual({ granulation: 0, regions: 0 });
    expect(surfaceDetail(4000, 0.002).granulation).toBe(1);
    expect(surfaceDetail(9, 0.002).regions).toBeCloseTo(0.5, 6);
    expect(surfaceDetail(20, 0.002).regions).toBe(1);
  });

  it('starLook falls back without activity', () => {
    const look = starLook(SUN_SYSTEM, null);
    expect(look.limb[0]).toBeCloseTo(0.6, 2);
    expect(look.limb[1]).toBe(0.2);
    expect(look.granuleContrast).toBe(0.12);
    expect(look.shear).toBe(0);
    expect(look.omegaEq).toBeCloseTo((2 * Math.PI) / 25, 9);
    expect(look.colour).toEqual(SUN_SYSTEM.light);
    const hot = starLook({ ...SUN_SYSTEM, teff_k: 9700 }, null);
    expect(hot.granuleContrast).toBe(0);
    expect(hot.limb[0]).toBe(0.3);
  });

  it('starLook takes the activity when there is one', () => {
    const look = starLook(SUN_SYSTEM, SUN_ACTIVITY);
    expect(look.limb).toEqual([0.462, 0.258]);
    expect(look.omegaEq).toBeCloseTo((2 * Math.PI) / 25.4, 9);
    expect(look.shear).toBe(0.2);
    expect(look.granuleSize).toBe(SUN_ACTIVITY.granule_size);
    expect(look.faculaRatio).toBe(10);
  });

  it('repacks only when the set changes', () => {
    const uniforms = starSurfaceUniforms(32);
    const state = new StarSurfaceState(uniforms, 32);
    state.setStar(starLook(SUN_SYSTEM, SUN_ACTIVITY), SUN_ACTIVITY);
    expect(state.update(1000, BIG)).toBe(true);
    expect(state.regions.length).toBeGreaterThan(0);
    expect(uniforms['uRegionCount']!.value).toBe(state.regions.length);
    expect(state.update(1000.001, BIG)).toBe(false);
    expect(state.update(1000 + 30, BIG)).toBe(true);
  });

  it('keeps the shader time small', () => {
    const uniforms = starSurfaceUniforms(32);
    const state = new StarSurfaceState(uniforms, 32);
    state.setStar(starLook(SUN_SYSTEM, SUN_ACTIVITY), SUN_ACTIVITY);
    state.update(1e7, BIG);
    state.update(1e7 + 19.5, BIG);
    expect(Math.abs(uniforms['uLocalDays']!.value as number)).toBeLessThan(21);
    const granule = uniforms['uGranule']!.value as { z: number };
    expect(granule.z).toBeGreaterThanOrEqual(0);
    expect(granule.z).toBeLessThan(289);
  });

  it('packs each region at the epoch, so the shader finds it where the timeline does', () => {
    const uniforms = starSurfaceUniforms(32);
    const state = new StarSurfaceState(uniforms, 32);
    state.setStar(starLook(SUN_SYSTEM, SUN_ACTIVITY), SUN_ACTIVITY);
    state.update(500, BIG);
    state.update(500.5, BIG);
    const a = uniforms['uRegionA']!.value as Float32Array;
    const b = uniforms['uRegionB']!.value as Float32Array;
    const local = uniforms['uLocalDays']!.value as number;
    const now = state.regions[0]!;
    const count = uniforms['uRegionCount']!.value as number;
    const i = Array.from({ length: count }, (_, k) => k).find(
      (k) => Math.abs(a[4 * k]! - now.region.latitude) < 1e-6 && Math.abs(b[4 * k + 3]! - now.region.peakArea) < 1e-9,
    );
    expect(i).toBeDefined();
    if (i === undefined) return;
    const lat = a[4 * i]!;
    const omega = ((2 * Math.PI) / 25.4) * (1 - 0.2 * Math.sin(lat) ** 2);
    const lon = a[4 * i + 1]! + omega * local;
    const wrap = (x: number) => Math.atan2(Math.sin(x), Math.cos(x));
    expect(wrap(lon - now.longitude)).toBeCloseTo(0, 4);
    // The triangular area history, from (born − epoch, growth, lifetime, peak).
    const t = local - b[4 * i]!;
    const [growth, lifetime, peak] = [b[4 * i + 1]!, b[4 * i + 2]!, b[4 * i + 3]!];
    const area = t < growth ? (peak * t) / growth : (peak * (lifetime - t)) / (lifetime - growth);
    expect(area / now.area).toBeCloseTo(1, 4);
  });

  it('offers loop regions only while the clock is slow enough for the spots', () => {
    const state = new StarSurfaceState(starSurfaceUniforms(32), 32);
    state.setStar(starLook(SUN_SYSTEM, SUN_ACTIVITY), SUN_ACTIVITY);
    state.update(1000, BIG);
    expect(state.loopRegions.length).toBeGreaterThan(0);
    expect(state.loopFade).toBe(1);
    // 2^12 days a second: 136 days a frame, far above 25 / 12 days.
    state.update(1136, { ...BIG, daysPerSecond: 4096 });
    expect(state.loopRegions).toEqual([]);
  });

  it('draws nothing of the regions when they are faded out or absent', () => {
    const uniforms = starSurfaceUniforms(12);
    const state = new StarSurfaceState(uniforms, 12);
    state.setStar(starLook(SUN_SYSTEM, SUN_ACTIVITY), SUN_ACTIVITY);
    expect(state.update(1000, { radiusPx: 5, daysPerSecond: 1 })).toBe(false);
    expect(uniforms['uRegionFade']!.value).toBe(0);
    expect(state.regions).toEqual([]);
    state.setStar(starLook(SUN_SYSTEM, null), null);
    expect(state.update(1000, BIG)).toBe(false);
    expect(uniforms['uRegionCount']!.value).toBe(0);
    expect(state.update(1000, BIG)).toBe(false);
  });

  it('declares every uniform the chunk uses', () => {
    const uniforms = starSurfaceUniforms(8);
    for (const name of Object.keys(uniforms)) {
      expect(STAR_SURFACE_GLSL).toMatch(new RegExp(`uniform \\w+ ${name}\\b`));
    }
    const used = new Set(STAR_SURFACE_GLSL.match(/\bu[A-Z]\w*/g));
    for (const name of used) expect(Object.keys(uniforms)).toContain(name);
    expect(STAR_SURFACE_GLSL).toContain('vec3 starSurfaceColour(vec3 n, float mu)');
    expect(STAR_SURFACE_GLSL).toContain('STAR_NO_GRANULATION');
    expect((uniforms['uRegionA']!.value as Float32Array).length).toBe(32);
  });

  it('re-packs after 20 days even when the same regions are alive', () => {
    // Long-lived, rare regions: look for a day whose set is unchanged 21 days on.
    const rare: StarActivity = { ...SUN_ACTIVITY, region_rate: 0.03, decay_per_day: 1e-7, cycle_depth: 0 };
    const ids = (d: number) => activeRegions(rare, d, 32).map((r) => r.region.id).sort().join(' ');
    let day = Number.NaN;
    for (let d = 200; d < 5000; d += 1) {
      if (ids(d) !== '' && ids(d) === ids(d + 21) && ids(d) === ids(d + 10)) {
        day = d;
        break;
      }
    }
    expect(Number.isNaN(day)).toBe(false);
    const uniforms = starSurfaceUniforms(32);
    const state = new StarSurfaceState(uniforms, 32);
    state.setStar(starLook(SUN_SYSTEM, rare), rare);
    expect(state.update(day, BIG)).toBe(true);
    expect(state.update(day + 10, BIG)).toBe(false);
    expect(uniforms['uLocalDays']!.value).toBeCloseTo(10, 9);
    expect(state.update(day + 21, BIG)).toBe(true);
    expect(uniforms['uLocalDays']!.value).toBe(0);
  });

  it('is a function of the day alone, whatever days came before', () => {
    // What the shader rebuilds from the uniforms, region by region, sorted.
    const decode = (u: ReturnType<typeof starSurfaceUniforms>) => {
      const a = u['uRegionA']!.value as Float32Array;
      const b = u['uRegionB']!.value as Float32Array;
      const local = u['uLocalDays']!.value as number;
      const omega = u['uOmegaEq']!.value as number;
      const shear = u['uShear']!.value as number;
      const out: number[][] = [];
      for (let i = 0; i < (u['uRegionCount']!.value as number); i++) {
        const lat = a[4 * i]!;
        const t = local - b[4 * i]!;
        const [growth, lifetime, peak] = [b[4 * i + 1]!, b[4 * i + 2]!, b[4 * i + 3]!];
        const area = t <= 0 || t >= lifetime ? 0 : t < growth ? (peak * t) / growth : (peak * (lifetime - t)) / (lifetime - growth);
        const lon = a[4 * i + 1]! + omega * (1 - shear * Math.sin(lat) ** 2) * local;
        out.push([lat, Math.cos(lon), Math.sin(lon), a[4 * i + 2]!, a[4 * i + 3]!, area]);
      }
      return out.sort((x, y) => x[0]! - y[0]! || x[5]! - y[5]!);
    };
    const swept = starSurfaceUniforms(32);
    const state = new StarSurfaceState(swept, 32);
    state.setStar(starLook(SUN_SYSTEM, SUN_ACTIVITY), SUN_ACTIVITY);
    // Forwards in small and large steps, then backwards.
    const days = [
      ...Array.from({ length: 60 }, (_, i) => 3000 + i * 0.37),
      ...Array.from({ length: 20 }, (_, i) => 3030 + i * 7.3),
      ...Array.from({ length: 30 }, (_, i) => 3200 - i * 2.9),
    ];
    for (const d of days) {
      state.update(d, BIG);
      const fresh = starSurfaceUniforms(32);
      const once = new StarSurfaceState(fresh, 32);
      once.setStar(starLook(SUN_SYSTEM, SUN_ACTIVITY), SUN_ACTIVITY);
      once.update(d, BIG);
      expect(state.regions).toEqual(activeRegions(SUN_ACTIVITY, d, 32));
      expect(state.regions).toEqual(once.regions);
      for (const name of ['uRegionCount', 'uRegionFade']) expect(swept[name]!.value).toBe(fresh[name]!.value);
      expect((swept['uGranule']!.value as { toArray(): number[] }).toArray()).toEqual(
        (fresh['uGranule']!.value as { toArray(): number[] }).toArray(),
      );
      const x = decode(swept);
      const y = decode(fresh);
      expect(x.length).toBe(y.length);
      x.forEach((row, i) => row.forEach((v, k) => expect(v).toBeCloseTo(y[i]![k]!, k === 5 ? 9 : 4)));
    }
  });

  it('takes μ towards the eye, not from the view-space normal', () => {
    // From a camera d radii from the centre, the limb seen has a
    // view-space normal z of about 1/d: z as μ never reaches the limb
    // (from 3.2 radii, the M4V close-up, the edge stopped at μ ≈ 0.3).
    expect(starSurfaceFragment).toMatch(/dot\(normalize\(vWorldNormal\), normalize\(cameraPosition - vWorld\)\)/);
    expect(starSurfaceFragment).not.toContain('vViewNormal');
    expect(starSurfaceVertex).toContain('vWorld = world.xyz');
  });
});
