import { describe, expect, it } from "vitest";
import type { StarActivity } from "./api/types/StarActivity";
import {
  MAX_LIFETIME_DAYS,
  activeRegions,
  angularVelocity,
  bandLatitude,
  cycleLevel,
  hash32,
  regionsBornIn,
  rotationPhase,
} from "./starActivity";

// Seed-1234 outputs of the Rust model (task 1), G2V, A0V and M5V.
const SUN: StarActivity = {"envelope":"convective","age_gyr":4.6,"rotation_days":25.45958,"shear":0.22217314,"turnover_days":13.803839,"rossby":1.844384,"activity":0.00077597605,"cycle_years":8.780228,"cycle_phase":0.22184348,"cycle_depth":0.9,"band_deg":[28.0,7.0],"spot_coverage":0.0029878789,"facula_ratio":10.020264,"region_rate":1.0290694,"region_area":0.00015208183,"region_sigma":0.9466193,"decay_per_day":0.00001,"granule_size":0.0018903897,"granule_contrast":0.14989609,"granule_lifetime_days":0.005687718,"limb":[0.46191466,0.25807068],"seed":3214283846};
const HOT: StarActivity = {"envelope":"radiative","age_gyr":1.4346926,"rotation_days":0.71322525,"shear":0.5,"turnover_days":0.0,"rossby":0.0,"activity":0.0,"cycle_years":0.0,"cycle_phase":0.22184348,"cycle_depth":0.0,"band_deg":[0.0,0.0],"spot_coverage":0.0,"facula_ratio":0.0,"region_rate":0.0,"region_area":0.0,"region_sigma":0.0,"decay_per_day":0.00001,"granule_size":0.003158995,"granule_contrast":0.0,"granule_lifetime_days":0.020596515,"limb":[0.233016,0.32803598],"seed":3214283846};
const M5V: StarActivity = {"envelope":"fully_convective","age_gyr":4.6,"rotation_days":72.5245,"shear":0.002209387,"turnover_days":124.49028,"rossby":0.58257157,"activity":0.017426249,"cycle_years":30.807196,"cycle_phase":0.22184348,"cycle_depth":0.7411045,"band_deg":[75.0,0.0],"spot_coverage":0.021889234,"facula_ratio":3.702076,"region_rate":1.572516,"region_area":0.00041163387,"region_sigma":0.9466193,"decay_per_day":0.00001,"granule_size":0.0011985536,"granule_contrast":0.042158145,"granule_lifetime_days":0.0006984249,"limb":[0.4385,0.346],"seed":3214283846};
// M5V is not saturated (Ro 0.58): a saturated star derived from it, with a
// rate above the normal-approximation threshold (30 per day).
const SATURATED: StarActivity = {
  ...M5V,
  rossby: 0.1,
  activity: 1,
  spot_coverage: 0.3,
  cycle_depth: 0.3,
  region_rate: 40,
  region_area: 0.004,
};

const P_DAYS = SUN.cycle_years * 365.25;

/** Day of the first cycle maximum after day 3 P, found by scanning. */
function maxDay(): number {
  let best = 0;
  let bestDay = 0;
  for (let d = 3 * P_DAYS; d < 4 * P_DAYS; d += 1) {
    const l = cycleLevel(SUN, d);
    if (l > best) { best = l; bestDay = d; }
  }
  return bestDay;
}
/** Day of the cycle minimum (cycle start) after day 3 P. */
function minDay(): number {
  for (let d = 3 * P_DAYS; d < 4 * P_DAYS; d += 1) {
    if (cycleLevel(SUN, d) < cycleLevel(SUN, d + 1) && cycleLevel(SUN, d - 1) > cycleLevel(SUN, d) + 1e-9) return d;
  }
  return 3 * P_DAYS;
}

describe('star activity timeline', () => {
  it('gives the same regions for the same day, whatever the path', () => {
    const at = activeRegions(SUN, 1234.5, 32);
    activeRegions(SUN, 9000, 32);
    activeRegions(SUN, -50, 32);
    expect(activeRegions(SUN, 1234.5, 32)).toEqual(at);
  });

  it('scrubs forward and backward to the same surface', () => {
    const days: number[] = [];
    for (let d = 1200; d <= 1300; d += 0.37) days.push(d);
    const forward = days.map((d) => activeRegions(SUN, d, 32));
    for (let i = days.length - 1; i >= 0; i--) {
      expect(activeRegions(SUN, days[i]!, 32)).toEqual(forward[i]);
    }
  });

  it('stays finite and in range at negative and far days', () => {
    for (const d of [-3650, 0, 1e7]) {
      const now = activeRegions(SUN, d, 32);
      expect(now.length).toBeGreaterThan(0);
      for (const r of now) {
        expect(Number.isFinite(r.area) && r.area > 0).toBe(true);
        expect(Math.abs(r.longitude)).toBeLessThanOrEqual(Math.PI);
      }
    }
  });

  it('keeps regions in the band of their birth', () => {
    let seen = 0;
    for (let slot = 0; slot < 2000; slot++) {
      for (const r of regionsBornIn(SUN, slot)) {
        seen++;
        const lat = Math.abs((r.latitude * 180) / Math.PI);
        expect(Math.abs(lat - bandLatitude(SUN, r.born))).toBeLessThanOrEqual(16);
      }
    }
    expect(seen).toBeGreaterThan(100);
  });

  it('follows the cycle', () => {
    const count = (centre: number): number => {
      let n = 0;
      for (let s = Math.floor(centre - 182); s < centre + 183; s++) n += regionsBornIn(SUN, s).length;
      return n;
    };
    expect(count(maxDay())).toBeGreaterThan(3 * count(minDay()));
  });

  it('covers spot_coverage at cycle maximum on average', () => {
    // spot_coverage is a share of the visible hemisphere; regions spread
    // evenly in longitude, so the whole star carries twice that area.
    const centre = maxDay();
    let sum = 0;
    for (let i = -182; i <= 182; i++) {
      for (const r of activeRegions(SUN, centre + i, Infinity)) sum += r.area;
    }
    const mean = sum / 365;
    expect(mean).toBeGreaterThan(0.7 * 2 * SUN.spot_coverage);
    expect(mean).toBeLessThan(1.3 * 2 * SUN.spot_coverage);
  });

  it('draws nothing on a radiative star', () => {
    expect(activeRegions(HOT, 100, 32)).toEqual([]);
  });

  it('turns the equator faster than 60°', () => {
    expect(angularVelocity(SUN, 0)).toBeGreaterThan(angularVelocity(SUN, Math.PI / 3));
  });

  it('caps the regions, largest first', () => {
    const now = activeRegions(SATURATED, 500, 8);
    expect(now.length).toBe(8);
    for (let i = 1; i < now.length; i++) expect(now[i]!.area).toBeLessThanOrEqual(now[i - 1]!.area);
    expect(regionsBornIn(SATURATED, 10).length).toBeLessThanOrEqual(64);
  });

  it('pins a vector', () => {
    expect(activeRegions(SUN, 1000, 4).map((r) => r.region.id)).toEqual([677345221, 1708615553, 2492798505, 2162515033]);
  });

  it('has no cycle on a star without one', () => {
    const flat = { ...SUN, cycle_years: 0 };
    expect(cycleLevel(flat, 123)).toBe(1);
    expect(bandLatitude(flat, 123)).toBe(28);
  });

  it('drifts the band from start to end and keeps levels in [0, 1]', () => {
    const start = minDay();
    expect(bandLatitude(SUN, start + 0.5)).toBeCloseTo(28, 0);
    expect(bandLatitude(SUN, start + P_DAYS - 0.5)).toBeCloseTo(7, 0);
    for (let d = 0; d < P_DAYS; d += 20) {
      const l = cycleLevel(SUN, d);
      expect(l).toBeGreaterThanOrEqual(0);
      expect(l).toBeLessThanOrEqual(1);
    }
  });

  it('flips polarity with hemisphere and with the cycle', () => {
    const born = [...Array(1500).keys()].flatMap((s) => regionsBornIn(SUN, s));
    for (const r of born) {
      const cycle = Math.floor(r.born / P_DAYS + SUN.cycle_phase);
      expect(r.polarity).toBe((cycle % 2 === 0 ? 1 : -1) * Math.sign(r.latitude));
    }
  });

  it('keeps ids unique and reduces the rotation phase', () => {
    const ids = new Set<number>();
    let n = 0;
    for (let s = 0; s < 500; s++) for (const r of regionsBornIn(SUN, s)) { ids.add(r.id); n++; }
    expect(ids.size).toBe(n);
    for (const d of [-100, 0, 1e7]) {
      const p = rotationPhase(SUN, d);
      expect(p).toBeGreaterThanOrEqual(0);
      expect(p).toBeLessThan(2 * Math.PI);
    }
    expect(hash32(1, 2)).not.toBe(hash32(2, 1));
    expect(MAX_LIFETIME_DAYS).toBe(120);
  });
});
