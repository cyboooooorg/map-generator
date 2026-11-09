// What the star view draws, from a world's star: the layout of its system
// (the look of its surface is in starSurface.ts).
import type { Trojans } from '../api/types/Trojans';
import type { Moon } from '../api/types/Moon';
import type { Giant } from '../api/types/Giant';
import type { RingSystem } from '../api/types/RingSystem';
import type { StarInfo } from '../api/types/StarInfo';

/**
 * A planet of a planetary system as the star view draws it: a giant, with
 * its orbit's tilt to the star's equator and the longitude of its
 * ascending node (radians), and its moons. A world's giants have none.
 */
export type SystemGiant = Giant & {
  /**
   * In a planetary system, its rings as the system drew them (edges and
   * gaps in its radii, opacity, colour), which `rings` sums up; absent when
   * the system was drawn before rings were.
   */
  ringSystem?: RingSystem | null;
  /** In a planetary system, its trojans at L4 and L5 (bodies larger than 1 km), where they last. */
  trojans?: Trojans | null;
  inclination?: number;
  node?: number;
  moons?: readonly OrbitMoon[];
};

/**
 * A moon as the star view draws it; `ring` for a moon of a giant's rings
 * (in a gap, at their edge, shepherding them, or making a gap by
 * resonance), drawn where it is, among the rings.
 */
export type OrbitMoon = Pick<Moon, 'radius_km' | 'inclination_deg' | 'period_days' | 'barycentre_km' | 'semi_major_axis_km'> & {
  ring?: boolean;
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

/** View units per e-fold of distance in a planetary system's scale: a doubling is about 1.8 units. */
const SYSTEM_LOG_STEP = 2.6;
/** Most view units a planetary system's scale reaches beyond the star's surface, so the camera can frame it. */
const SYSTEM_LOG_SPAN = 14.5;

/**
 * Places distances (AU) on a planetary system's schematic, in view units:
 * logarithmic in AU from just outside the star to the farthest orbit or
 * edge of the habitable zone, so equal ratios of distance take equal room.
 * A compact inner system (planets at 0.07, 0.09 and 0.15 AU) is as readable
 * as giants tens of AU out, and the habitable zone keeps its place and its
 * band, only its width is on the same log scale. The innermost orbit or
 * zone edge sits 0.6 view units off the star's surface; closer than that
 * the scale goes linearly down to the star's centre. Continuous and
 * increasing; it depends on the system alone, so it does not change with
 * the planet selected.
 */
export function systemLogScale(
  star: Pick<StarInfo, 'hz_inner_au' | 'hz_outer_au'>,
  planets: Pick<Giant, 'orbit_au' | 'eccentricity'>[],
): (au: number) => number {
  const nearest = Math.min(star.hz_inner_au, ...planets.map((p) => p.orbit_au * (1 - p.eccentricity)));
  const farthest = Math.max(star.hz_outer_au, ...planets.map((p) => p.orbit_au * (1 + p.eccentricity)));
  const origin = Math.max(nearest, 1e-6) * 0.85;
  const r0 = STAR_RADIUS * 1.6;
  const ratio = Math.log(Math.max(farthest, origin * 1.001) / origin);
  const k = Math.min(SYSTEM_LOG_STEP, SYSTEM_LOG_SPAN / ratio);
  return (au) => (au <= origin ? (r0 * Math.max(au, 0)) / origin : r0 + k * Math.log(au / origin));
}

/** Smallest marker of a planet in a planetary system's view, view units: it stays visible (picking has its own tolerance). */
export const MARKER_MIN = 0.035;
/** Share of the gap to the nearest orbit a planet's body may fill. */
const MARKER_GAP_SHARE = 0.4;
/** Share of that gap everything drawn round a planet (rings, halo, moons) may fill: half, so it stays between the orbits. */
export const FOOTPRINT_GAP_SHARE = 0.5;

/**
 * The room round each planet in a planetary system's view, view units: the
 * distance, on screen, from its orbit curve to the nearest other orbit
 * curve (the neighbour's periapsis to its apoapsis, so eccentric orbits
 * count) or to the star's surface. `orbits` are the planets' periapsis
 * and apoapsis on the view's scale (`systemLogScale`).
 */
export function orbitGaps(orbits: readonly { near: number; far: number }[]): number[] {
  return orbits.map((own, k) => {
    let gap = own.near - STAR_RADIUS;
    orbits.forEach((other, j) => {
      if (j === k) return;
      gap = Math.min(gap, other.near >= own.far ? other.near - own.far : own.near >= other.far ? own.near - other.far : 0);
    });
    return Math.max(gap, 0);
  });
}

/**
 * The sizes planets' markers are drawn with in a planetary system's view,
 * view units: each planet's own size (`natural`) at most, shrunk to
 * `MARKER_GAP_SHARE` of its `gaps` (see orbitGaps), and so that all it
 * draws round it, `extent` times its radius (its rings, its highlight; 1
 * for none), stays within `FOOTPRINT_GAP_SHARE` of the gap, and never
 * under `MARKER_MIN`: nothing a planet draws reaches another's orbit.
 */
export function markerSizes(natural: readonly number[], gaps: readonly number[], extent: readonly number[] = []): number[] {
  return natural.map((size, k) =>
    Math.max(MARKER_MIN, Math.min(size, MARKER_GAP_SHARE * gaps[k]!, (FOOTPRINT_GAP_SHARE * gaps[k]!) / Math.max(extent[k] ?? 1, 1))),
  );
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

/**
 * The frost line of a star, AU: where water ice condensed in the disk the
 * planets formed from, 2.7 AU for the Sun, as the square root of the
 * luminosity (L in Suns). Rocky planets form inside it, giants beyond.
 */
export function frostLineAu(luminosity: number): number {
  return 2.7 * Math.sqrt(Math.max(luminosity, 0));
}

/** Earth masses in a Sun's mass. */
const EARTH_PER_SUN = 332_946;

/**
 * A planet's Hill radius, AU: how far round it its own gravity holds moons
 * against the star's (at its periapsis, `a (1 − e)`, the tightest),
 * `a (1 − e) ∛(m / 3M)`.
 */
export function hillRadiusAu(orbitAu: number, eccentricity: number, massEarth: number, starMass: number): number {
  return orbitAu * (1 - eccentricity) * Math.cbrt(massEarth / EARTH_PER_SUN / (3 * Math.max(starMass, 1e-6)));
}

/**
 * The radius a planet's Hill sphere is drawn with in a planetary system's
 * view, and whether that is to scale: `trueRadius` (its Hill radius on the
 * view's scale), unless that is smaller than the planet's marker
 * (`markerSize`) or than its outermost moon's ring (`moonReach`, 0 for
 * none), both drawn larger than to scale: then just outside those.
 */
export function hillShownRadius(trueRadius: number, markerSize: number, moonReach: number): { radius: number; toScale: boolean } {
  const floor = Math.max(markerSize * 1.6, moonReach * 1.15);
  return trueRadius >= floor ? { radius: trueRadius, toScale: true } : { radius: floor, toScale: false };
}

/**
 * A sphere's grid of `radius`, as line segments (pairs of points): its
 * parallels and its meridians every `spacing` degrees (30: the equator and
 * two parallels each side, and six meridians).
 */
export function sphereGrid(radius: number, steps = 64, spacing = 30): [number, number, number][] {
  const points: [number, number, number][] = [];
  const circle = (at: (t: number) => [number, number, number]) => {
    for (let i = 0; i < steps; i++) points.push(at((i / steps) * 2 * Math.PI), at(((i + 1) / steps) * 2 * Math.PI));
  };
  const parallels = Array.from({ length: Math.round(180 / spacing) - 1 }, (_, i) => -90 + spacing * (i + 1));
  for (const lat of parallels) {
    const y = radius * Math.sin((lat * Math.PI) / 180);
    const r = radius * Math.cos((lat * Math.PI) / 180);
    circle((t) => [r * Math.cos(t), y, -r * Math.sin(t)]);
  }
  const meridians = Math.round(180 / spacing);
  for (let m = 0; m < meridians; m++) {
    const lon = (m / meridians) * Math.PI;
    circle((t) => [radius * Math.cos(t) * Math.cos(lon), radius * Math.sin(t), -radius * Math.cos(t) * Math.sin(lon)]);
  }
  return points;
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

/** A coronal loop (or the prominence an eruption lifts): an arc of plasma between two points of the surface (unit vectors), `height` star radii high. */
export interface Prominence {
  from: [number, number, number];
  to: [number, number, number];
  height: number;
  /** Thickness of the arc, star radii. */
  thickness: number;
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
 * Tilt of a giant's rings round its marker, radians out of the view's
 * plane (seen a little open, as most giants' are): a ring moon's orbit too.
 */
export const RING_TILT = 0.45;
/** Radius of a ring moon's marker, in planet markers: small enough for its gap. */
export const RING_MOON_SIZE = 0.06;
/** The rings drawn round a giant's marker without their system's edges, in its radii. */
export const SYMBOLIC_RINGS: [number, number] = [1.35, 2.3];

/**
 * The bands a giant's rings are drawn as round its marker, in its radii:
 * from their inner to their outer edge, cut by their gaps, each gap at
 * least `least` wide on either side of its middle (a ring moon's marker
 * fits in it). Bands narrower than a hundredth of a radius are dropped.
 */
export function ringBands(rings: Pick<RingSystem, 'inner' | 'outer' | 'gaps'>, least: number): [number, number][] {
  const cuts = rings.gaps
    .map(([at, width]) => [at - Math.max(width / 2, least), at + Math.max(width / 2, least)] as const)
    .sort((x, y) => x[0] - y[0]);
  const bands: [number, number][] = [];
  let from = rings.inner;
  for (const [start, end] of cuts) {
    if (start > from) bands.push([from, Math.min(start, rings.outer)]);
    from = Math.max(from, end);
  }
  if (from < rings.outer) bands.push([from, rings.outer]);
  return bands.filter(([a, b]) => b - a > 0.01);
}

/**
 * The orbits the star view draws for a planet's moons, nearest first: the
 * nth one at a fixed spacing from the planet (real distances span 2 to
 * 1 000 planet radii), the largest moons drawn larger, each at its real
 * period relative to the planet's year (`yearS` seconds when animated, two
 * seconds at the shortest). A binary's planet swings around the barycentre.
 *
 * A ring moon is drawn to scale instead, in the rings' plane, where the
 * rings round the marker (see ringBands) put its gap or their edge; the
 * other moons then start beyond the outermost ring moon.
 */
export function moonOrbits(
  moons: readonly OrbitMoon[],
  planetRadiusKm: number,
  yearDays: number,
  yearS: number,
  binary: boolean,
): MoonOrbit[] {
  const radii = (moon: OrbitMoon) => moon.semi_major_axis_km / Math.max(planetRadiusKm, 1e-9);
  const ringReach = Math.max(0, ...moons.filter((m) => m.ring).map(radii));
  const first = ringReach > 0 ? Math.max(1.6, ringReach + 0.35) : 1.6;
  let slot = 0;
  return moons.map((moon, k) => {
    const period = Math.max(MOON_MIN_PERIOD_S, (moon.period_days / Math.max(yearDays, 1e-3)) * yearS);
    if (moon.ring) {
      return {
        radius: PLANET_MARKER * radii(moon),
        size: PLANET_MARKER * RING_MOON_SIZE,
        inclination: RING_TILT,
        speed: (2 * Math.PI) / period,
        phase: k * 2.4,
        swing: 0,
      };
    }
    const radius = PLANET_MARKER * (first + 0.55 * slot);
    slot += 1;
    const relative = Math.min(1, Math.max(0.02, moon.radius_km / planetRadiusKm));
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
