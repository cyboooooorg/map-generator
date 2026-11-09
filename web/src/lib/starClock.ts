/**
 * The clock of the star view (StarSurfacePane) and how it meets the system
 * view's (StarPane): both show the same day, in Earth days from the
 * system's t = 0, and each runs at its own speed, a power of two of days
 * per second.
 *
 * - The system view runs from 1 to 4 096 days a second (SYSTEM_SPEED_MIN..MAX,
 *   lib/viewer/starSystem.ts): planets going round.
 * - The star view runs from 1/4 096 day a second (21 s a second: a Sun-like
 *   granule boils in about 23 s) to 64 days a second (a solar rotation in
 *   under half a second). Both clocks run forwards; the time bar and the
 *   jumps move the day either way.
 *
 * Each view keeps its own speed and the star view opens at
 * `defaultStarSpeed`; nothing is carried over on a switch, so nothing is
 * clamped. `clampSpeed` keeps each view's own setting in range. The day
 * itself is shared and carried over unchanged.
 */
import type { StarActivity } from './api/types/StarActivity';
import { cycleLevel } from './starActivity';
import { SUN_GRANULE_LIFETIME_DAYS } from './viewer/starSurface';
import { SYSTEM_SPEED_MAX, SYSTEM_SPEED_MIN } from './viewer/starSystem';

/** The star view's slowest speed, log2 of days per second (1/4 096 day a second). */
export const STAR_SPEED_MIN = -12;
/** The star view's fastest speed, log2 of days per second (64 days a second). */
export const STAR_SPEED_MAX = 6;

const DAYS_PER_YEAR = 365.25;

/** Seconds of animation one granule lifetime lasts at the star view's default speed. */
const GRANULE_SHOWN_S = 10;

/** `speed` (log2 of days per second) clamped into the range of `mode`'s view. */
export function clampSpeed(speed: number, mode: 'system' | 'star'): number {
  const [min, max] = mode === 'star' ? [STAR_SPEED_MIN, STAR_SPEED_MAX] : [SYSTEM_SPEED_MIN, SYSTEM_SPEED_MAX];
  return Math.min(max, Math.max(min, speed));
}

/**
 * log2 of days/s so that one granule lifetime lasts about 10 s; SYSTEM
 * default for a radiative star (none boil). Without an activity, the
 * fallback surface's granules (the Sun's, see starLook).
 */
export function defaultStarSpeed(activity: StarActivity | null): number {
  if (activity && !(activity.granule_contrast > 0)) return SYSTEM_SPEED_MIN;
  const lifetime = activity ? activity.granule_lifetime_days : SUN_GRANULE_LIFETIME_DAYS;
  if (!(lifetime > 0)) return SYSTEM_SPEED_MIN;
  return clampSpeed(Math.round(Math.log2(lifetime / GRANULE_SHOWN_S)), 'star');
}

/**
 * Share of the cycle (in [0, 1), from its minimum) where the cycle level
 * peaks: found once by a golden-section search on the shape of
 * `cycleLevel`, which is the same for every star (only its period and
 * phase differ).
 */
const PEAK_FRACTION = (() => {
  // A unit cycle starting at day 0: one day is the share of the cycle.
  const unitCycle = { cycle_years: 1 / DAYS_PER_YEAR, cycle_phase: 0 } as StarActivity;
  const level = (f: number) => cycleLevel(unitCycle, f);
  const ratio = (Math.sqrt(5) - 1) / 2;
  let lo = 0.05;
  let hi = 0.95;
  for (let i = 0; i < 80; i++) {
    const a = hi - ratio * (hi - lo);
    const b = lo + ratio * (hi - lo);
    if (level(a) < level(b)) lo = a;
    else hi = b;
  }
  return (lo + hi) / 2;
})();

/** The first day after `days` at which the cycle's fraction is `fraction` (mod 1). */
function nextAt(activity: StarActivity, days: number, fraction: number): number {
  const period = activity.cycle_years * DAYS_PER_YEAR;
  const turns = days / period + activity.cycle_phase;
  let at = (Math.floor(turns - fraction) + 1 + fraction - activity.cycle_phase) * period;
  // Rounding may land on `days` itself (a jump repeated): the next cycle's then.
  if (!(at > days + 1e-9 * period)) at += period;
  return at;
}

/** The next cycle maximum and minimum after `days` (days), null without a cycle. */
export function cycleJumps(activity: StarActivity, days: number): { maximum: number; minimum: number } | null {
  if (!(activity.cycle_years > 0) || !Number.isFinite(days)) return null;
  return { maximum: nextAt(activity, days, PEAK_FRACTION), minimum: nextAt(activity, days, 0) };
}

/** Where a star is in its cycle (see cyclePhase). */
export type CyclePhase = 'rising' | 'maximum' | 'declining' | 'minimum';

/**
 * Cycle level above which the star is said at its maximum, and below which
 * at its minimum: a **calibration** of the words, not of the model.
 */
const MAXIMUM_LEVEL = 0.8;
const MINIMUM_LEVEL = 0.2;

/**
 * The phase of `activity`'s cycle at `days`, from `cycleLevel`: maximum
 * near the peak, minimum near the trough, otherwise rising (the maximum
 * comes before the next minimum) or declining; null without a cycle.
 */
export function cyclePhase(activity: StarActivity, days: number): CyclePhase | null {
  const jumps = cycleJumps(activity, days);
  if (!jumps) return null;
  const level = cycleLevel(activity, days);
  if (level >= MAXIMUM_LEVEL) return 'maximum';
  if (level <= MINIMUM_LEVEL) return 'minimum';
  return jumps.maximum < jumps.minimum ? 'rising' : 'declining';
}
