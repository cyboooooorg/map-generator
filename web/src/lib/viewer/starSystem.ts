// What the star view draws, from a world's star: the look of its surface
// and the layout of its system.
import type { Moon } from '../api/types/Moon';
import type { Giant } from '../api/types/Giant';
import type { StarInfo } from '../api/types/StarInfo';

/** The surface of a star as the star view draws it (see starSurfaceFragment). */
export interface StarSurface {
  /** Linear limb darkening coefficient: stronger on cool stars. */
  limb: number;
  /** Contrast of the convective granules; none on hot stars (above about 7 500 K). */
  granulation: number;
  /** About the share of the surface under spots: large on red dwarfs, none on hot stars. */
  spots: number;
  /** 1 on red dwarfs, which flare. */
  flares: number;
}

/** The surface of a star of effective temperature `teffK` and spectral class `klass`. */
export function starSurface(teffK: number, klass: string): StarSurface {
  const clamp = (x: number, lo: number, hi: number) => Math.min(hi, Math.max(lo, x));
  const spots: Record<string, number> = { M: 0.25, K: 0.12, G: 0.05, F: 0.02 };
  return {
    limb: clamp(0.6 + (5772 - teffK) / 5000, 0.3, 0.85),
    granulation: teffK > 7500 ? 0 : 0.12,
    spots: spots[klass] ?? 0,
    flares: klass === 'M' ? 1 : 0,
  };
}

/**
 * A planet of a planetary system as the star view draws it: a giant, with
 * its orbit's tilt to the star's equator and the longitude of its
 * ascending node (radians), and its moons. A world's giants have none.
 */
export type SystemGiant = Giant & {
  inclination?: number;
  node?: number;
  moons?: readonly Pick<Moon, 'radius_km' | 'inclination_deg' | 'period_days' | 'barycentre_km' | 'semi_major_axis_km'>[];
};

/** Radius of the star in the view: the system is drawn around it, not to scale. */
export const STAR_RADIUS = 1;

/**
 * Places distances (AU) on the system schematic, in view units: linear in
 * AU, so the habitable zone and the orbit keep their proportions, starting
 * just outside the star (which is not to scale) and fitting the farthest of
 * the orbit and the zone's outer edge within `span`. `from` is where the
 * linear part ends: the world's orbit by default, the habitable zone in a
 * planetary system (see systemCompressedFrom).
 */
export function systemScale(
  star: Pick<StarInfo, 'apoapsis_au' | 'hz_outer_au'>,
  giants: Pick<Giant, 'orbit_au' | 'eccentricity'>[] = [],
  span = 6,
  from = compressedFrom(star),
): (au: number) => number {
  const farthest = from;
  const linear = (au: number) => STAR_RADIUS * 1.6 + (span * au) / farthest;
  const outer = Math.max(farthest, ...giants.map((g) => g.orbit_au * (1 + g.eccentricity)));
  if (outer <= farthest) return linear;
  // Beyond the world's system, giants far out (tens of AU) would squash it
  // to a dot: their distances are compressed logarithmically, the farthest
  // GIANT_REACH further out, never growing faster than the linear part.
  const k = Math.min(span, GIANT_REACH / Math.log(outer / farthest));
  return (au) => (au <= farthest ? linear(au) : linear(farthest) + k * Math.log(au / farthest));
}

/** Where the star view starts compressing distances, AU: just beyond the world's orbit and the habitable zone. */
export function compressedFrom(star: Pick<StarInfo, 'apoapsis_au' | 'hz_outer_au'>): number {
  return Math.max(star.apoapsis_au, star.hz_outer_au) * 1.15;
}

/**
 * Where a planetary system's view starts compressing distances, AU: just
 * beyond the habitable zone. It depends on the star only, so the scale
 * stays the same whichever planet is selected.
 */
export function systemCompressedFrom(star: Pick<StarInfo, 'hz_outer_au'>): number {
  return star.hz_outer_au * 1.15;
}

/** View units the farthest giant lies beyond the start of the compression. */
const GIANT_REACH = 3.5;

/**
 * Size of a giant's marker in the star view (view units), from its radius
 * in Earth radii: larger than the world's, growing slowly (Neptune about
 * 1.3 times the world's marker, Jupiter about 1.6 times).
 */
export function giantMarkerSize(radiusEarth: number): number {
  return PLANET_MARKER * Math.min(2.2, Math.max(1, 0.8 + 0.35 * Math.log(radiusEarth)));
}

/**
 * Where a planet is on its orbit at `meanAnomaly` (radians, growing evenly
 * with time): its true anomaly (the angle from the periapsis, seen from the
 * star) and its distance as a share of the semi-major axis, from Kepler's
 * equation `M = E − e sin E`, solved by Newton's method. Faster near the
 * star, slower far from it.
 */
export function keplerPosition(eccentricity: number, meanAnomaly: number): { trueAnomaly: number; distance: number } {
  const e = eccentricity;
  // Within one turn (−π to π): Newton's method from M + e sin M (from ±π on
  // very eccentric orbits) then converges for any time, however long the
  // animation has run.
  const m = meanAnomaly - 2 * Math.PI * Math.round(meanAnomaly / (2 * Math.PI));
  let anomaly = e < 0.8 ? m + e * Math.sin(m) : m < 0 ? -Math.PI : Math.PI;
  for (let i = 0; i < 12; i += 1) {
    anomaly -= (anomaly - e * Math.sin(anomaly) - m) / (1 - e * Math.cos(anomaly));
  }
  const trueAnomaly =
    2 * Math.atan2(Math.sqrt(1 + e) * Math.sin(anomaly / 2), Math.sqrt(1 - e) * Math.cos(anomaly / 2));
  return { trueAnomaly, distance: 1 - e * Math.cos(anomaly) };
}

/** Distance from the star at a true anomaly, as a share of the semi-major axis: `(1 − e²) / (1 + e cos ν)`. */
export function orbitDistance(eccentricity: number, trueAnomaly: number): number {
  return (1 - eccentricity ** 2) / (1 + eccentricity * Math.cos(trueAnomaly));
}

/** Orientation of a world's orbit in the view (its periapsis), radians, from its seed: only a look. */
export function periapsisAngle(seed: number): number {
  return ((seed * 0.618_033_988_75) % 1) * 2 * Math.PI;
}

/** Earth's mean orbital speed, km/s. */
const EARTH_SPEED_KMS = 29.78;

/**
 * Orbital speed at `distanceAu` from a star of `mass` (M☉) on an orbit of
 * semi-major axis `orbitAu`, km/s: the vis-viva equation,
 * `v = 29.78 km/s × sqrt(M (2/r − 1/a))` (Earth: 29.78 km/s).
 */
export function orbitalSpeed(mass: number, orbitAu: number, distanceAu: number): number {
  return EARTH_SPEED_KMS * Math.sqrt(Math.max(mass * (2 / distanceAu - 1 / orbitAu), 0));
}

/** Length of the fastest planet's velocity vector in the star view, view units. */
export const VELOCITY_MAX_LENGTH = 0.7;

/**
 * View units per km/s of the velocity vectors of a system's planets: one
 * scale for all, so they compare at a glance, the fastest (at its
 * periapsis) `maxLength` long, short enough not to cross many orbits.
 */
export function velocityScale(
  mass: number,
  orbits: readonly { orbit_au: number; eccentricity: number }[],
  maxLength = VELOCITY_MAX_LENGTH,
): number {
  const fastest = Math.max(0, ...orbits.map((o) => orbitalSpeed(mass, o.orbit_au, o.orbit_au * (1 - o.eccentricity))));
  return fastest > 0 ? maxLength / fastest : 0;
}

/**
 * Seconds of animation a planetary system's innermost planet takes to go
 * round at the default speed (see defaultSystemSpeed).
 */
export const SYSTEM_INNER_YEAR_S = 10;
/** The slowest and fastest speeds of a planetary system's clock, powers of two of Earth days per second. */
export const SYSTEM_SPEED_MIN = 0;
export const SYSTEM_SPEED_MAX = 12;

/**
 * The speed a planetary system opens at, a power of two of Earth days per
 * second: about its innermost year in SYSTEM_INNER_YEAR_S, a day a second
 * at the slowest.
 */
export function defaultSystemSpeed(innermostYearDays: number): number {
  const exact = Math.round(Math.log2(Math.max(innermostYearDays, 1e-6) / SYSTEM_INNER_YEAR_S));
  return Math.min(SYSTEM_SPEED_MAX, Math.max(SYSTEM_SPEED_MIN, exact));
}

/** The star's pull on a planet `distanceAu` from it, m/s² (G M / r², `mass` in Suns): the Sun's on Earth is 5.93 mm/s². */
export function starGravity(mass: number, distanceAu: number): number {
  return (5.93e-3 * mass) / (distanceAu * distanceAu);
}

/** Length of the strongest pull's vector in the star view, view units. */
export const GRAVITY_MAX_LENGTH = 0.5;

/**
 * View units per m/s² of the vectors of the star's pull on a system's
 * planets: one scale for all, the strongest (the closest planet at its
 * periapsis) `maxLength` long. The pull falls as the square of the
 * distance, so far planets get very short vectors: their labels give the
 * value.
 */
export function gravityScale(
  mass: number,
  orbits: readonly { orbit_au: number; eccentricity: number }[],
  maxLength = GRAVITY_MAX_LENGTH,
): number {
  const strongest = Math.max(0, ...orbits.map((o) => starGravity(mass, o.orbit_au * (1 - o.eccentricity))));
  return strongest > 0 ? maxLength / strongest : 0;
}

/** Mean orbital speed over a year, km/s: the orbit's length over its period (close to the circular speed). */
export function meanOrbitalSpeed(mass: number, orbitAu: number): number {
  return EARTH_SPEED_KMS * Math.sqrt(mass / orbitAu);
}

/** A prominence: an arc of plasma between two points of the surface (unit vectors), `height` star radii high. */
export interface Prominence {
  from: [number, number, number];
  to: [number, number, number];
  height: number;
  /** Thickness of the arc, star radii. */
  thickness: number;
}

/** Prominences by spectral class: magnetically active cool stars have many, hot stars none. */
const PROMINENCES: Record<string, number> = { M: 7, K: 5, G: 4, F: 2 };

/** The prominences of a star of spectral class `klass`, placed from `seed`. */
export function prominences(klass: string, seed: number): Prominence[] {
  let state = (seed * 2654435761) >>> 0;
  const next = () => {
    state = (state + 0x6d2b79f5) >>> 0;
    let z = state;
    z = Math.imul(z ^ (z >>> 15), z | 1);
    z ^= z + Math.imul(z ^ (z >>> 7), z | 61);
    return ((z ^ (z >>> 14)) >>> 0) / 4_294_967_296;
  };
  return Array.from({ length: PROMINENCES[klass] ?? 0 }, () => {
    // A foot anywhere on the sphere, the other 10° to 30° away.
    const y = 2 * next() - 1;
    const lon = next() * 2 * Math.PI;
    const r = Math.sqrt(1 - y * y);
    const from: [number, number, number] = [r * Math.cos(lon), y, r * Math.sin(lon)];
    const spread = ((10 + 20 * next()) * Math.PI) / 180;
    const heading = next() * 2 * Math.PI;
    // Turn `from` by `spread` towards `heading` on the sphere.
    const east = [-Math.sin(lon), 0, Math.cos(lon)];
    const north = [-y * Math.cos(lon), r, -y * Math.sin(lon)];
    const d = [0, 1, 2].map((k) => Math.cos(heading) * east[k]! + Math.sin(heading) * north[k]!);
    const to = [0, 1, 2].map((k) => Math.cos(spread) * from[k]! + Math.sin(spread) * d[k]!) as [number, number, number];
    return { from, to, height: 0.08 + 0.22 * next() ** 2, thickness: 0.012 + 0.014 * next() };
  });
}

/**
 * A stored star with the orbit fields of worlds made before eccentric
 * orbits filled in: a circle at the semi-major axis.
 */
export function withOrbitDefaults(star: StarInfo): StarInfo {
  const e = star.eccentricity ?? 0;
  return {
    ...star,
    eccentricity: e,
    periapsis_au: star.periapsis_au || star.orbit_au * (1 - e),
    apoapsis_au: star.apoapsis_au || star.orbit_au * (1 + e),
    insolation_periapsis: star.insolation_periapsis || star.insolation,
    insolation_apoapsis: star.insolation_apoapsis || star.insolation,
  };
}

/** The Sun and Earth, for worlds made before stars. */
export const SUN_SYSTEM: StarInfo = {
  type: 'G2V',
  teff_k: 5770,
  luminosity: 1.0233,
  mass: 1,
  radius: 1.012,
  orbit_au: 1,
  eccentricity: 0,
  periapsis_au: 1,
  apoapsis_au: 1,
  insolation: 1,
  insolation_periapsis: 1,
  insolation_apoapsis: 1,
  hz_position: 0.068,
  hz_inner_au: 0.962,
  hz_outer_au: 1.697,
  equilibrium_k: 255,
  year_days: 365.25,
  angular_diameter_deg: 0.533,
  lifetime_gyr: 9.8,
  locking_gyr: 166,
  tidally_locked: false,
  light: [1, 1, 1],
  metallicity: 0,
};

/** A moon as the star view draws it around its planet (not to scale). */
export interface MoonOrbit {
  /** Radius of its orbit around the planet, view units. */
  radius: number;
  /** Size of its marker, view units. */
  size: number;
  /** Inclination of its orbit to the plane of the system, radians. */
  inclination: number;
  /** Radians per animated second; negative on a retrograde orbit. */
  speed: number;
  /** Angle at the start of the clock, radians. */
  phase: number;
  /** Share of the orbit's radius the planet swings around the barycentre (a binary), else 0. */
  swing: number;
}

/** Radius of the planet's marker in the star view, view units. */
export const PLANET_MARKER = 0.11;
/** Shortest animated period of a moon, s: closer moons would only flicker. */
const MOON_MIN_PERIOD_S = 2;

/**
 * The orbits the star view draws for a planet's moons, nearest first: the
 * nth one at a fixed spacing from the planet (real distances span 2 to
 * 1 000 planet radii), the largest moons drawn larger, each at its real
 * period relative to the planet's year (`yearS` seconds when animated, two
 * seconds at the shortest). A binary's planet swings around the barycentre.
 */
export function moonOrbits(
  moons: readonly Pick<Moon, 'radius_km' | 'inclination_deg' | 'period_days' | 'barycentre_km' | 'semi_major_axis_km'>[],
  planetRadiusKm: number,
  yearDays: number,
  yearS: number,
  binary: boolean,
): MoonOrbit[] {
  return moons.map((moon, k) => {
    const radius = PLANET_MARKER * (1.6 + 0.55 * k);
    const relative = Math.min(1, Math.max(0.02, moon.radius_km / planetRadiusKm));
    const period = Math.max(MOON_MIN_PERIOD_S, (moon.period_days / Math.max(yearDays, 1e-3)) * yearS);
    const retrograde = moon.inclination_deg > 90;
    const swing = binary && moon.barycentre_km > planetRadiusKm ? moon.barycentre_km / moon.semi_major_axis_km : 0;
    return {
      radius,
      size: PLANET_MARKER * Math.max(0.18, Math.sqrt(relative)),
      inclination: ((retrograde ? 180 - moon.inclination_deg : moon.inclination_deg) * Math.PI) / 180,
      speed: ((retrograde ? -1 : 1) * 2 * Math.PI) / period,
      phase: k * 2.4,
      swing,
    };
  });
}
