// What the star view draws, from a world's star: the look of its surface
// and the layout of its system.
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

/** Radius of the star in the view: the system is drawn around it, not to scale. */
export const STAR_RADIUS = 1;

/**
 * Places distances (AU) on the system schematic, in view units: linear in
 * AU, so the habitable zone and the orbit keep their proportions, starting
 * just outside the star (which is not to scale) and fitting the farthest of
 * the orbit and the zone's outer edge within `span`.
 */
export function systemScale(star: Pick<StarInfo, 'apoapsis_au' | 'hz_outer_au'>, span = 6): (au: number) => number {
  const farthest = Math.max(star.apoapsis_au, star.hz_outer_au) * 1.15;
  return (au) => STAR_RADIUS * 1.6 + (span * au) / farthest;
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
  let anomaly = e < 0.8 ? meanAnomaly : Math.PI;
  for (let i = 0; i < 12; i += 1) {
    anomaly -= (anomaly - e * Math.sin(anomaly) - meanAnomaly) / (1 - e * Math.cos(anomaly));
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
    // A foot anywhere on the sphere, the other 5° to 15° away.
    const y = 2 * next() - 1;
    const lon = next() * 2 * Math.PI;
    const r = Math.sqrt(1 - y * y);
    const from: [number, number, number] = [r * Math.cos(lon), y, r * Math.sin(lon)];
    const spread = ((5 + 10 * next()) * Math.PI) / 180;
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
};
