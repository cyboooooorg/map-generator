import type { Vector4 } from 'three';
import { describe, expect, it } from 'vitest';
import type { StarActivity } from '../api/types/StarActivity';
import { activeRegions, hash32, rotationPhase } from '../starActivity';
import { starSurfaceFragment, starSurfaceVertex } from './starShaders';
import { SUN_SYSTEM } from './starSystem';
import {
  exposureFade,
  granuleMotionHarmonics,
  granuleDaysWrap,
  granulePhase,
  networkPhase,
  STAR_SURFACE_GLSL,
  SUPERGRANULE_LIFETIME_DAYS,
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

/** An A-type star's activity (starActivity.test.ts): radiative, no granules. */
const HOT: StarActivity = {
  envelope: 'radiative',
  age_gyr: 1.4346926,
  rotation_days: 0.71322525,
  shear: 0.5,
  turnover_days: 0,
  rossby: 0,
  activity: 0,
  cycle_years: 0,
  cycle_phase: 0.22184348,
  cycle_depth: 0,
  band_deg: [0, 0],
  spot_coverage: 0,
  facula_ratio: 0,
  region_rate: 0,
  region_area: 0,
  region_sigma: 0,
  decay_per_day: 1e-5,
  granule_size: 0.003158995,
  granule_contrast: 0,
  granule_lifetime_days: 0.020596515,
  limb: [0.233016, 0.32803598],
  seed: 3214283846,
};
const HOT_INFO = { ...SUN_SYSTEM, teff_k: 9700 };

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
    expect(surfaceDetail(5, 0.002)).toEqual({ granulation: 0, regions: 0, network: 0 });
    expect(surfaceDetail(4000, 0.002).granulation).toBe(1);
    expect(surfaceDetail(9, 0.002).regions).toBeCloseTo(0.5, 6);
    expect(surfaceDetail(20, 0.002).regions).toBe(1);
  });

  it('keeps granules off until they span 2.5 pixels', () => {
    // The star view's default distance puts G2V granules under 2 px.
    expect(surfaceDetail(1200, 0.002).granulation).toBe(0);
    expect(surfaceDetail(1250, 0.002).granulation).toBe(0);
    expect(surfaceDetail(1875, 0.002).granulation).toBeCloseTo(0.5, 6);
    expect(surfaceDetail(2500, 0.002).granulation).toBe(1);
  });

  it('shows the network from afar', () => {
    expect(surfaceDetail(500, 0.0019).network).toBe(1); // whole disc in a 1080p view
    expect(surfaceDetail(20, 0.0019).network).toBe(0); // a small star: off
  });

  it('keeps a radiative star plain', () => {
    const u = starSurfaceUniforms(8);
    const s = new StarSurfaceState(u, 8);
    s.setStar(starLook(HOT_INFO, HOT), HOT);
    s.update(100, { radiusPx: 500, daysPerSecond: 1 });
    expect((u.uNetwork!.value as Vector4).y).toBe(0);
  });

  it('draws the network on a convective star, but not on the small sky disc', () => {
    const u = starSurfaceUniforms(8);
    const s = new StarSurfaceState(u, 8);
    s.setStar(starLook(SUN_SYSTEM, SUN_ACTIVITY), SUN_ACTIVITY);
    s.update(100.3, { radiusPx: 500, daysPerSecond: 1 });
    const network = u.uNetwork!.value as Vector4;
    expect(network.y).toBeCloseTo(0.15, 9);
    expect(network.z).toBeCloseTo(networkPhase(100.3), 9);
    expect(network.w).toBe(1);
    // The sky disc passes its own detail (hostStarDetail): 0 for the usual disc.
    s.update(100.3, { radiusPx: 500, daysPerSecond: 1, regionDetail: 0, networkDetail: 0 });
    expect(network.w).toBe(0);
    // A clock far faster than a supergranule lives averages it out.
    s.update(100.3, { radiusPx: 500, daysPerSecond: 4096 });
    expect(network.w).toBeLessThan(0.1);
  });

  it('turns the network with the star', () => {
    const u = starSurfaceUniforms(8);
    const s = new StarSurfaceState(u, 8);
    s.setStar(starLook(SUN_SYSTEM, SUN_ACTIVITY), SUN_ACTIVITY);
    for (const day of [0, 3.7, 100.3, 2e4 + 0.25]) {
      s.update(day, { radiusPx: 500, daysPerSecond: 1 });
      expect(u.uNetworkSpin!.value).toBeCloseTo(rotationPhase(SUN_ACTIVITY, day), 9);
    }
    // n is taken to the star's frame: longitude minus the phase, as the regions add it.
    expect(STAR_SURFACE_GLSL).toContain('vec3(n.x * spinC - n.z * spinS, n.y, n.x * spinS + n.z * spinC)');
  });

  it('fades the network out on a fast rotator, as the regions', () => {
    const fast: StarActivity = { ...SUN_ACTIVITY, rotation_days: 0.5 };
    const u = starSurfaceUniforms(8);
    const s = new StarSurfaceState(u, 8);
    s.setStar(starLook(SUN_SYSTEM, fast), fast);
    s.update(100, { radiusPx: 500, daysPerSecond: 4 });
    const fade = (u.uNetwork!.value as Vector4).w;
    // A frame of 4/30 day outlasts a twelfth of a 0.5-day rotation.
    expect(fade).toBeCloseTo(exposureFade(4 / 30, 0.5 / 12), 9);
    expect(fade).toBeLessThan(0.6);
    expect(fade).toBeCloseTo(u.uRegionFade!.value as number, 9);
  });

  it('caps the network below the plages at the limb', () => {
    expect(STAR_SURFACE_GLSL).toMatch(/float limb = max\(0\.35, min\(0\.35 \+ 1\.4 \* edge, STAR_NETWORK_LIMB_CAP \* edge\)\);/);
    const cap = Number(/const float STAR_NETWORK_LIMB_CAP = ([\d.]+);/.exec(STAR_SURFACE_GLSL)![1]);
    // A ribbon's peak (contrast 0.15 times at most 1) under a full plage's 0.25 (1 − μ).
    expect(0.15 * cap).toBeLessThan(0.25);
  });

  it('shades granules by their size on screen', () => {
    const u = starSurfaceUniforms(8);
    const s = new StarSurfaceState(u, 8);
    s.setStar(starLook(SUN_SYSTEM, SUN_ACTIVITY), SUN_ACTIVITY);
    s.update(100, { radiusPx: 20000, daysPerSecond: 0 });
    expect(u.uGranulePx!.value).toBeCloseTo(20000 * SUN_ACTIVITY.granule_size, 9);
    expect(STAR_SURFACE_GLSL).toMatch(/float small = 1\.0 - smoothstep\(40\.0, 120\.0, uGranulePx\);/);
  });

  it('draws small granules as a continuous sum of blobs, without specks', () => {
    // Small on screen, the blobs alone; up close, the cells' edges; in between, both in one pass.
    expect(STAR_SURFACE_GLSL).toContain('float starCellBlobs(vec3 p, float phase)');
    expect(STAR_SURFACE_GLSL).toMatch(/if \(small >= 1\.0\) \{\n\s+blob = starCellBlobs\(q, phase\);/);
    expect(STAR_SURFACE_GLSL).toContain('cell = starCellDetail(q, phase, STAR_BLOB_SHARPNESS)');
    // The bright points, cut by the lanes they sat in, are gone.
    expect(STAR_SURFACE_GLSL).not.toMatch(/cell\.point|cell\.pair/);
  });

  it('fades the mottle out as the granules fade in', () => {
    // Without granules compiled in (the sky disc), uGranule.w is not read.
    expect(STAR_SURFACE_GLSL).toMatch(/#ifdef STAR_NO_GRANULATION\n\s+float mottleFade = 1\.0;/);
    expect(STAR_SURFACE_GLSL).toContain('float mottleFade = 1.0 - uGranule.w;');
    expect(STAR_SURFACE_GLSL).toMatch(/if \(mottleFade > 0\.0\) \{\n\s+vec3 v = starValueNoise3/);
  });

  it('keeps the network phase continuous where its day clock wraps', () => {
    const wrap = granuleDaysWrap(SUPERGRANULE_LIFETIME_DAYS);
    const eps = 1e-3;
    const before = networkPhase(wrap - eps);
    const after = networkPhase(wrap + eps);
    const step = (((after - before) % 289) + 289) % 289;
    expect(step).toBeCloseTo((2 * eps) / SUPERGRANULE_LIFETIME_DAYS, 6);
    expect(networkPhase(-3.7)).toBeGreaterThanOrEqual(0);
    expect(networkPhase(-3.7)).toBeLessThan(289);
  });

  it('draws the network outside STAR_NO_GRANULATION, behind its fade', () => {
    const gate = STAR_SURFACE_GLSL.indexOf('#ifndef STAR_NO_GRANULATION\n  // No granules inside a spot');
    const network = STAR_SURFACE_GLSL.indexOf('if (uNetwork.w > 0.0)');
    expect(network).toBeGreaterThan(0);
    expect(gate).toBeGreaterThan(0);
    // Outside the granules' gated block in starBrightness (which comes
    // after the regions, to skip the granules inside spots).
    const end = STAR_SURFACE_GLSL.indexOf('#endif', gate);
    expect(network < gate || network > end).toBe(true);
    expect(STAR_SURFACE_GLSL).toMatch(/0\.35 \+ 1\.4 \* edge/);
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

  it('packs a seed per region', () => {
    const u = starSurfaceUniforms(32);
    const s = new StarSurfaceState(u, 32);
    s.setStar(starLook(SUN_SYSTEM, SUN_ACTIVITY), SUN_ACTIVITY);
    s.update(1000, { radiusPx: 4000, daysPerSecond: 0 });
    const c = u['uRegionC']!.value as Vector4[];
    expect(s.regions.length).toBeGreaterThan(0);
    s.regions.forEach((r, i) => expect(c[i]!.x).toBeCloseTo(hash32(r.region.id, 7) / 2 ** 32, 6));
  });

  it('shapes the leader and the follower from the region seed and age', () => {
    expect(STAR_SURFACE_GLSL).toMatch(/starSpotShaped\(local - axis, r \* 0\.7745967, seed, age, aa\)/);
    expect(STAR_SURFACE_GLSL).toMatch(/starSpotShaped\(local \+ axis, r \* 0\.6324555, fract\(seed \+ 0\.5\), age, aa\)/);
    expect(STAR_SURFACE_GLSL).toMatch(/float age = t \/ b\.z;/);
    // The moat reaches 2.6 of the leader's radius: the region's reach covers it.
    expect(STAR_SURFACE_GLSL).toMatch(/0\.5 \* a\.w \+ 2\.02 \* r/);
  });

  it("fades the spots' moving details with the clock's speed", () => {
    const u = starSurfaceUniforms(8);
    const s = new StarSurfaceState(u, 8);
    s.setStar(starLook(SUN_SYSTEM, SUN_ACTIVITY), SUN_ACTIVITY);
    s.update(1000, BIG);
    expect(u['uSpotMotion']!.value).toBe(1);
    // A day a second: 48 minutes a frame, far beyond the details' 5 minutes.
    s.update(1000, { radiusPx: 4000, daysPerSecond: 1 });
    expect(u['uSpotMotion']!.value as number).toBeLessThan(0.5);
  });

  it('draws shaped spots', () =>
    expect(STAR_SURFACE_GLSL).toMatch(/float starSpotShaped\(vec2 local, float rs, float seed, float age, float aa\)/));

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

  it("takes a view's own region detail over the radius ramp", () => {
    const uniforms = starSurfaceUniforms(12);
    const state = new StarSurfaceState(uniforms, 12);
    state.setStar(starLook(SUN_SYSTEM, SUN_ACTIVITY), SUN_ACTIVITY);
    // 7 px of radius would show a sixth of the regions; the sky disc says none.
    expect(state.update(1000, { radiusPx: 7, daysPerSecond: 0, regionDetail: 0 })).toBe(false);
    expect(uniforms['uRegionFade']!.value).toBe(0);
    expect(uniforms['uRegionCount']!.value).toBe(0);
    state.update(1000, { radiusPx: 7, daysPerSecond: 0, regionDetail: 1 });
    expect(uniforms['uRegionFade']!.value).toBe(1);
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

  it('draws granules as cells', () => {
    expect(STAR_SURFACE_GLSL).toMatch(/vec2 starCells\(vec3 p, float phase\)/);
    expect(STAR_SURFACE_GLSL).not.toMatch(/starNoise\(p \+ mod\(step3/); // the simplex cross-fade is gone
    expect(STAR_SURFACE_GLSL).toMatch(/float starGranules\(vec3 n\)/);
  });

  it('keeps every cell motion periodic in the phase wrap', () => {
    // Each angular rate on the phase is 2π·k/GRANULE_PHASE_PERIOD for an integer k.
    const harmonics = granuleMotionHarmonics();
    expect(harmonics.length).toBeGreaterThan(0);
    for (const k of harmonics) {
      expect(Number.isInteger(k)).toBe(true);
      expect(k).toBeGreaterThan(0);
      // The chunk's rates are written from the list, not typed in again.
      expect(STAR_SURFACE_GLSL).toContain(((2 * Math.PI * k) / 289).toFixed(8));
    }
  });

  it('keeps the granule phase continuous where the day clock wraps', () => {
    const lifetime = SUN_ACTIVITY.granule_lifetime_days;
    const wrap = granuleDaysWrap(lifetime);
    // A whole number of 289-lifetime periods, near 1e4 days.
    const periods = wrap / (289 * lifetime);
    expect(Math.abs(periods - Math.round(periods))).toBeLessThan(1e-9);
    expect(Math.abs(wrap - 1e4)).toBeLessThan(289 * lifetime);
    const eps = 1e-4;
    for (const k of [1, 2, 7]) {
      const before = granulePhase(k * wrap - eps, lifetime);
      const after = granulePhase(k * wrap + eps, lifetime);
      const step = (((after - before) % 289) + 289) % 289;
      expect(step).toBeCloseTo((2 * eps) / lifetime, 6);
    }
    // The state sets that phase.
    const uniforms = starSurfaceUniforms(8);
    const state = new StarSurfaceState(uniforms, 8);
    state.setStar(starLook(SUN_SYSTEM, SUN_ACTIVITY), SUN_ACTIVITY);
    state.update(wrap + 0.3, BIG);
    expect((uniforms['uGranule']!.value as { z: number }).z).toBeCloseTo(granulePhase(wrap + 0.3, lifetime), 9);
  });

  it('keeps the cell helpers out of STAR_NO_GRANULATION, for the sky disc', () => {
    const gate = STAR_SURFACE_GLSL.indexOf('#ifndef STAR_NO_GRANULATION');
    expect(gate).toBeGreaterThan(0);
    for (const helper of ['vec2 starCells(', 'float starFbm2(', 'float starHash(']) {
      const at = STAR_SURFACE_GLSL.indexOf(helper);
      expect(at, helper).toBeGreaterThan(0);
      expect(at, helper).toBeLessThan(gate);
    }
    expect(STAR_SURFACE_GLSL.indexOf('float starGranules(vec3 n)')).toBeGreaterThan(gate);
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
      for (const name of ['uRegionCount', 'uRegionFade', 'uNetworkSpin']) expect(swept[name]!.value).toBe(fresh[name]!.value);
      for (const name of ['uGranule', 'uNetwork']) {
        expect((swept[name]!.value as { toArray(): number[] }).toArray()).toEqual(
          (fresh[name]!.value as { toArray(): number[] }).toArray(),
        );
      }
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
