/**
 * The star's active-region timeline: a pure function of a `StarActivity`
 * (which carries the seed) and a day. Nothing here depends on frame
 * history, wall time or the path taken to reach a day, so scrubbing in
 * either direction gives the same surface.
 *
 * Time is cut into one-day slots. Slot n holds a Poisson number of
 * regions born in [n, n + 1), each drawn from its own hash stream. A
 * region lives at most `MAX_LIFETIME_DAYS`, so the regions alive at a day
 * come from the last `MAX_LIFETIME_DAYS + 1` slots.
 *
 * Days are Earth days from the system's t = 0 (f64, may be negative or
 * 1e7). Angles are radians, areas shares of the visible hemisphere.
 * The science is in `docs/science/star-activity.md`; the Rust side that
 * supplies the parameters is `crates/mapgen/src/activity.rs`.
 */
import type { StarActivity } from "./api/types/StarActivity";

/** A region as born: fixed for its whole life. Angles in radians, areas in shares of the hemisphere, days from the system's t = 0. */
export interface ActiveRegion {
  /** Stable: hash of (seed, slot, k); unique in practice. */
  id: number;
  /** Day of birth. */
  born: number;
  /** Days to its peak. */
  growth: number;
  /** Days from birth to gone. */
  lifetime: number;
  peakArea: number;
  /** Signed. */
  latitude: number;
  /** In the star's inertial frame at its birth, (−π, π]. */
  longitude: number;
  /** Joy's law, radians (a magnitude: the leader is equatorward). */
  tilt: number;
  /** Bipole footpoints, radians on the sphere. */
  separation: number;
  /** Leader's sign (Hale). */
  polarity: 1 | -1;
}

export interface RegionNow {
  region: ActiveRegion;
  area: number;
  /** Now, (−π, π]. */
  longitude: number;
}

/**
 * Longest life of a region, days. A **calibration**; it equals
 * `MAX_LIFETIME_DAYS` of `activity.rs`, which derives `region_rate` with
 * the same cap.
 */
export const MAX_LIFETIME_DAYS = 120;

const TWO_PI = 2 * Math.PI;
const DAYS_PER_YEAR = 365.25;
/** Normal approximation of the Poisson count above this mean, capped per slot. */
const NORMAL_ABOVE = 30;
const MAX_PER_SLOT = 64;

/**
 * Constant c of the cycle shape `f(x) = x³ / (exp(x²) − c)`, Hathaway,
 * Wilson & Reichmann 1994 (Sol. Phys. 151, 177). The value 0.71 is the
 * one the plan takes from that paper; the abstract was not reachable
 * when this was written. Hathaway 2015 (Living Rev. Sol. Phys. 12, 4)
 * gives 0.8 for its own parameterisation of the same function.
 */
const SHAPE_C = 0.71;
/**
 * Extent of x over one cycle: f(2.6) is 4 % of its peak, so a cycle
 * starts and ends near its minimum. A **calibration**.
 */
const SHAPE_X_END = 2.6;
const shapeRaw = (x: number): number => (x * x * x) / (Math.exp(x * x) - SHAPE_C);
/** Peak of `shapeRaw` on [0, SHAPE_X_END], found on a fine grid. */
const SHAPE_PEAK = (() => {
  let best = 0;
  for (let i = 0; i <= 2600; i++) best = Math.max(best, shapeRaw((i / 2600) * SHAPE_X_END));
  return best;
})();

/**
 * Drift constant of the butterfly, in units of the cycle: latitude
 * follows `exp(−K u)` (Hathaway 2011, Sol. Phys. 273, 221: 28° e^(−t/90
 * months) over an 11-year cycle, 132/90 = 1.47). Rounded to 1.5.
 */
const BAND_DRIFT = 1.5;
/** Scatter of the latitude about the band centre, degrees. A **calibration**. */
const BAND_SCATTER_DEG = 4;
/** Joy's law, tilt = 32.1° sin|λ| (Stenflo & Kosovichev 2012, ApJL 745, L129). */
const JOY_RAD = (32.1 * Math.PI) / 180;
/**
 * Spread of the growth time about `min(0.1 T, 3 d)`: a factor in
 * [0.8, 1.2]. A **calibration**, small so that the plan's rule stays
 * the rule.
 */
const GROWTH_JITTER = 0.2;

const cyclePeriodDays = (a: StarActivity): number => a.cycle_years * DAYS_PER_YEAR;

/** Fraction of the cycle elapsed at `days`, [0, 1); 0 when there is no cycle. */
function cycleFraction(a: StarActivity, days: number): number {
  if (!(a.cycle_years > 0)) return 0;
  const phase = days / cyclePeriodDays(a) + a.cycle_phase;
  return phase - Math.floor(phase);
}

/**
 * Cycle level at `days`, [0, 1] (1 at maximum); 1 when the star has no
 * cycle. The shape is `x³ / (exp(x²) − c)` (Hathaway, Wilson & Reichmann
 * 1994), normalised to a peak of 1 and stretched over `cycle_years`.
 */
export function cycleLevel(a: StarActivity, days: number): number {
  if (!(a.cycle_years > 0)) return 1;
  return shapeRaw(cycleFraction(a, days) * SHAPE_X_END) / SHAPE_PEAK;
}

/**
 * Active-band centre latitude (degrees, ≥ 0) at `days`: from
 * `band_deg[0]` at cycle start to `band_deg[1]` at its end, exponentially
 * in time (the Spörer law, Hathaway 2011). A star with no cycle stays at
 * `band_deg[0]`.
 */
export function bandLatitude(a: StarActivity, days: number): number {
  const [start, end] = a.band_deg;
  if (!(a.cycle_years > 0)) return start;
  const u = cycleFraction(a, days);
  const e = (Math.exp(-BAND_DRIFT * u) - Math.exp(-BAND_DRIFT)) / (1 - Math.exp(-BAND_DRIFT));
  return end + (start - end) * e;
}

/** Angular velocity at `latitude`, rad/day: Ω_eq (1 − shear sin²λ). */
export function angularVelocity(a: StarActivity, latitude: number): number {
  const s = Math.sin(latitude);
  return (TWO_PI / a.rotation_days) * (1 - a.shear * s * s);
}

/** Equatorial rotation phase at `days`, reduced to [0, 2π). */
export function rotationPhase(a: StarActivity, days: number): number {
  const p = (((TWO_PI / a.rotation_days) * days) % TWO_PI + TWO_PI) % TWO_PI;
  return p >= TWO_PI ? 0 : p;
}

/** Integer hash used by the timeline (and by coronalLoops for per-loop jitter). Unsigned 32 bits. */
export function hash32(...keys: number[]): number {
  let h = 0x9e3779b9;
  for (const key of keys) {
    h = Math.imul(h ^ (key | 0), 0x85ebca6b);
    h ^= h >>> 13;
    h = Math.imul(h, 0xc2b2ae35);
    h ^= h >>> 16;
  }
  // Final avalanche so that short key lists still mix well.
  h = Math.imul(h ^ (h >>> 16), 0x7feb352d);
  h = Math.imul(h ^ (h >>> 15), 0x846ca68b);
  h ^= h >>> 16;
  return h >>> 0;
}

const unit = (h: number): number => (h + 0.5) / 4294967296;

/** Standard normal from two uniforms (Box–Muller). */
const gauss = (u1: number, u2: number): number =>
  Math.sqrt(-2 * Math.log(u1)) * Math.cos(TWO_PI * u2);

/** Poisson-like count for slot `n` with mean `mean`, from `hash32(seed, n)`-seeded uniforms. */
function slotCount(seed: number, n: number, mean: number): number {
  if (!(mean > 0)) return 0;
  const uniform = (j: number): number => unit(hash32(seed, n, -1, j));
  if (mean > NORMAL_ABOVE) {
    const z = gauss(uniform(0), uniform(1));
    return Math.max(0, Math.min(MAX_PER_SLOT, Math.round(mean + Math.sqrt(mean) * z)));
  }
  // Knuth: multiply uniforms until the product falls below e^(−mean).
  const limit = Math.exp(-mean);
  let k = 0;
  let p = 1;
  for (let j = 0; k < MAX_PER_SLOT; j++) {
    p *= uniform(j);
    if (p <= limit) break;
    k++;
  }
  return k;
}

/** Mean number of regions born per day in slot `n` (cycle level at its middle). */
function slotRate(a: StarActivity, n: number): number {
  const level = cycleLevel(a, n + 0.5);
  return a.region_rate * (1 - a.cycle_depth + a.cycle_depth * level);
}

/** Regions born in day-slot `slot` (an integer, may be negative): a pure function of (a.seed, slot). */
export function regionsBornIn(a: StarActivity, slot: number): ActiveRegion[] {
  if (!(a.region_rate > 0)) return [];
  const count = slotCount(a.seed, slot, slotRate(a, slot));
  const out: ActiveRegion[] = [];
  const P = cyclePeriodDays(a);
  for (let k = 0; k < count; k++) {
    const f = (field: number): number => unit(hash32(a.seed, slot, k, field));
    const born = slot + f(0);
    const hemisphere = f(1) < 0.5 ? -1 : 1;
    const bandDeg = bandLatitude(a, born) + BAND_SCATTER_DEG * gauss(f(2), f(3));
    const absLat = Math.min(89, Math.abs(bandDeg)) * (Math.PI / 180);
    const latitude = hemisphere * absLat;
    // Lognormal peak area: median region_area, σ of ln A region_sigma.
    const peakArea = a.region_area * Math.exp(a.region_sigma * gauss(f(4), f(5)));
    const longitude = reduce(TWO_PI * f(6));
    // Gnevyshev–Waldmeier: T = A / W, capped; growth min(0.1 T, 3 d) with a small jitter.
    const lifetime = a.decay_per_day > 0
      ? Math.min(peakArea / a.decay_per_day, MAX_LIFETIME_DAYS)
      : MAX_LIFETIME_DAYS;
    const growth = Math.min(0.1 * lifetime, 3) * (1 - GROWTH_JITTER + 2 * GROWTH_JITTER * f(7));
    const cycleIndex = a.cycle_years > 0 ? Math.floor(born / P + a.cycle_phase) : 0;
    const polarity = ((((cycleIndex % 2) + 2) % 2 === 0 ? 1 : -1) * hemisphere) as 1 | -1;
    out.push({
      id: hash32(a.seed, slot, k),
      born,
      growth,
      lifetime,
      peakArea,
      latitude,
      longitude,
      tilt: JOY_RAD * Math.sin(absLat),
      separation: 4 * Math.sqrt(2 * peakArea),
      polarity,
    });
  }
  return out;
}

/** Reduces an angle to (−π, π]. */
function reduce(x: number): number {
  const m = ((x % TWO_PI) + TWO_PI) % TWO_PI;
  return m > Math.PI ? m - TWO_PI : m;
}

/** Area of `r` at `days` after its birth `t`: linear rise, then linear fall to 0 at `lifetime`. */
function areaAt(r: ActiveRegion, t: number): number {
  if (t < 0 || t >= r.lifetime) return 0;
  if (t < r.growth) return (r.peakArea * t) / r.growth;
  return (r.peakArea * (r.lifetime - t)) / (r.lifetime - r.growth);
}

/** Regions alive at `days`, with their area and longitude now, largest first, at most `max`. */
export function activeRegions(a: StarActivity, days: number, max: number): RegionNow[] {
  if (!(a.region_rate > 0)) return [];
  const out: RegionNow[] = [];
  for (let n = Math.floor(days - MAX_LIFETIME_DAYS); n <= Math.floor(days); n++) {
    for (const region of regionsBornIn(a, n)) {
      const t = days - region.born;
      const area = areaAt(region, t);
      if (!(area > 0)) continue;
      out.push({
        region,
        area,
        longitude: reduce(region.longitude + angularVelocity(a, region.latitude) * t),
      });
    }
  }
  out.sort((x, y) => y.area - x.area || x.region.id - y.region.id);
  return out.length > max ? out.slice(0, max) : out;
}
