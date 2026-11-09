import { describe, expect, it } from 'vitest';
import type { StarActivity } from './api/types/StarActivity';
import { activityRelativeToSun, cycleLevel } from './starActivity';
import { clampSpeed, cycleJumps, cyclePhase, defaultStarSpeed, STAR_SPEED_MAX, STAR_SPEED_MIN } from './starClock';
import { SYSTEM_SPEED_MAX, SYSTEM_SPEED_MIN } from './viewer/starSystem';

// Seed-1234 outputs of the Rust model (task 1), as in starActivity.test.ts.
const SUN: StarActivity = {"envelope":"convective","age_gyr":4.6,"rotation_days":25.45958,"shear":0.22217314,"turnover_days":13.803839,"rossby":1.844384,"activity":0.00077597605,"cycle_years":8.780228,"cycle_phase":0.22184348,"cycle_depth":0.9,"band_deg":[28.0,7.0],"spot_coverage":0.0029878789,"facula_ratio":10.020264,"region_rate":1.0290694,"region_area":0.00015208183,"region_sigma":0.9466193,"decay_per_day":0.00001,"granule_size":0.0018903897,"granule_contrast":0.14989609,"granule_lifetime_days":0.005687718,"limb":[0.46191466,0.25807068],"seed":3214283846};
const HOT: StarActivity = {"envelope":"radiative","age_gyr":1.4346926,"rotation_days":0.71322525,"shear":0.5,"turnover_days":0.0,"rossby":0.0,"activity":0.0,"cycle_years":0.0,"cycle_phase":0.22184348,"cycle_depth":0.0,"band_deg":[0.0,0.0],"spot_coverage":0.0,"facula_ratio":0.0,"region_rate":0.0,"region_area":0.0,"region_sigma":0.0,"decay_per_day":0.00001,"granule_size":0.003158995,"granule_contrast":0.0,"granule_lifetime_days":0.020596515,"limb":[0.233016,0.32803598],"seed":3214283846};

describe('star clock', () => {
  it('clamps speeds into each view', () => {
    expect(clampSpeed(10, 'star')).toBe(6);
    expect(clampSpeed(-12, 'system')).toBe(0);
    expect(clampSpeed(-20, 'star')).toBe(STAR_SPEED_MIN);
    expect(clampSpeed(20, 'system')).toBe(SYSTEM_SPEED_MAX);
    expect(clampSpeed(3, 'system')).toBe(3);
    expect(STAR_SPEED_MAX).toBe(6);
    expect(SYSTEM_SPEED_MIN).toBe(0);
  });

  it('boils granules at the default star speed', () => {
    const speed = defaultStarSpeed(SUN);
    expect(Number.isInteger(speed)).toBe(true);
    expect(speed).toBeGreaterThanOrEqual(STAR_SPEED_MIN);
    expect(speed).toBeLessThanOrEqual(STAR_SPEED_MAX);
    // One granule lifetime in about 10 s of animation, within a factor 2.
    const perTenSeconds = 2 ** speed * 10;
    expect(perTenSeconds).toBeGreaterThan(SUN.granule_lifetime_days / 2);
    expect(perTenSeconds).toBeLessThan(SUN.granule_lifetime_days * 2);
    // A radiative star has no granules to boil: the system view's default.
    expect(defaultStarSpeed(HOT)).toBe(SYSTEM_SPEED_MIN);
    // Without activity, the fallback surface's granules (the Sun's).
    expect(defaultStarSpeed(null)).toBe(speed);
  });

  it('jumps to the next maximum and minimum', () => {
    for (const d of [0, 1234.5, -3650, 1e7]) {
      const jumps = cycleJumps(SUN, d)!;
      expect(jumps.maximum).toBeGreaterThan(d);
      expect(jumps.minimum).toBeGreaterThan(d);
      // Within one cycle.
      expect(jumps.maximum - d).toBeLessThanOrEqual(SUN.cycle_years * 365.25 + 1e-6);
      expect(jumps.minimum - d).toBeLessThanOrEqual(SUN.cycle_years * 365.25 + 1e-6);
      expect(cycleLevel(SUN, jumps.maximum)).toBeGreaterThan(0.95);
      expect(cycleLevel(SUN, jumps.minimum)).toBeLessThan(0.1);
    }
    // At a maximum, the next one is a cycle later.
    const at = cycleJumps(SUN, 0)!.maximum;
    expect(cycleJumps(SUN, at)!.maximum).toBeGreaterThan(at + 365);
    expect(cycleJumps(HOT, 0)).toBeNull();
  });

  it('names the phase of the cycle', () => {
    const { maximum, minimum } = cycleJumps(SUN, 0)!;
    expect(cyclePhase(SUN, maximum)).toBe('maximum');
    expect(cyclePhase(SUN, minimum)).toBe('minimum');
    const P = SUN.cycle_years * 365.25;
    // Between a minimum and the maximum after it, the cycle rises; then declines.
    const nextMax = cycleJumps(SUN, minimum)!.maximum;
    expect(cyclePhase(SUN, (minimum + nextMax) / 2)).toBe('rising');
    const nextMin = cycleJumps(SUN, nextMax)!.minimum;
    expect(nextMin - nextMax).toBeLessThan(P);
    expect(cyclePhase(SUN, (nextMax + nextMin) / 2)).toBe('declining');
    expect(cyclePhase(HOT, 0)).toBeNull();
  });

  it("measures activity against the model's Sun", () => {
    // The fixture's Sun turns in 25.46 days, the model's anchor in 25.4.
    expect(activityRelativeToSun(SUN)).toBeGreaterThan(0.98);
    expect(activityRelativeToSun(SUN)).toBeLessThan(1.02);
    expect(activityRelativeToSun(HOT)).toBe(0);
  });
});
