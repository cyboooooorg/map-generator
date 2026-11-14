// The star field behind the globe: fixed points on the sky sphere, so they
// turn with the planet when the camera orbits it.
//
// The sky comes from a `SkyLook`: its seed picks the stars, its north pole
// orients the band of fainter stars like the Milky Way, its brightness
// scales how many there are, and its bulge adds a cluster toward the
// galactic centre. Compared worlds each keep their own sky, even with
// linked cameras. `FIXED_SKY` is the sky every world had before: seed 7,
// the band tilted 60 degrees to the planet's equator. Most stars are
// faint and white; a few are bright.

import type { Vec3 } from '../skyBodies';
import { FIXED_SKY, type SkyLook } from './skyLook';

/** One buffer per attribute, `count` stars long. */
export interface StarField {
  /** Unit directions, x y z per star. */
  directions: Float32Array;
  /** Linear RGB, already scaled by the star's brightness. */
  colours: Float32Array;
  /** Point size in CSS pixels. */
  sizes: Float32Array;
}

/** Stars spread over the whole sky, and stars along the band, at the Sun's place. */
export const STAR_COUNTS = { sky: 5500, band: 3000 } as const;

/** A long exposure records more stars: the sky's and the band's counts are scaled by this (the bulge's are not). */
export const PHOTO_STAR_DENSITY = 1.6;

/** The band keeps at least this share of the Sun's place's stars, so far out it still reads as a band. */
const BAND_FLOOR = 0.4;

/** Stars of the bulge cluster at full strength. */
export const BULGE_STARS = 1500;
/** Spread of the bulge cluster around the galactic centre, radians (standard deviation). */
const BULGE_WIDTH = (25 * Math.PI) / 180;

/** Star colours (linear RGB) and how common they are: cool red dwarfs are many, hot blue stars rare. */
const PALETTE: readonly { colour: readonly [number, number, number]; weight: number }[] = [
  { colour: [0.62, 0.72, 1.0], weight: 0.12 },
  { colour: [0.85, 0.9, 1.0], weight: 0.3 },
  { colour: [1.0, 1.0, 1.0], weight: 0.28 },
  { colour: [1.0, 0.92, 0.75], weight: 0.18 },
  { colour: [1.0, 0.75, 0.5], weight: 0.12 },
];

/** Small deterministic generator (mulberry32), uniform in [0, 1). */
function random(seed: number): () => number {
  let state = seed >>> 0;
  return () => {
    state = (state + 0x6d2b79f5) >>> 0;
    let z = state;
    z = Math.imul(z ^ (z >>> 15), z | 1);
    z ^= z + Math.imul(z ^ (z >>> 7), z | 61);
    return ((z ^ (z >>> 14)) >>> 0) / 4_294_967_296;
  };
}

type Basis = { u: Vec3; w: Vec3 };

const cross = (a: Vec3, b: Vec3): Vec3 => [
  a[1] * b[2] - a[2] * b[1],
  a[2] * b[0] - a[0] * b[2],
  a[0] * b[1] - a[1] * b[0],
];

/** Two unit vectors completing `axis` to a right-handed frame (u, axis, w), u as close to `hint` as possible. */
function frameAbout(axis: Vec3, hint: Vec3): Basis {
  const d = hint[0] * axis[0] + hint[1] * axis[1] + hint[2] * axis[2];
  let u: Vec3 = [hint[0] - d * axis[0], hint[1] - d * axis[1], hint[2] - d * axis[2]];
  let length = Math.hypot(...u);
  if (length < 1e-6) {
    // The hint is along the axis: any perpendicular will do.
    const other: Vec3 = Math.abs(axis[0]) < 0.9 ? [1, 0, 0] : [0, 1, 0];
    const e = other[0] * axis[0] + other[1] * axis[1] + other[2] * axis[2];
    u = [other[0] - e * axis[0], other[1] - e * axis[1], other[2] - e * axis[2]];
    length = Math.hypot(...u);
  }
  u = [u[0] / length, u[1] / length, u[2] / length];
  return { u, w: cross(u, axis) };
}

/**
 * The star field of a sky. The same look always gives the same stars.
 * `dust` gives the extinction (0 to 1) toward a unit direction: a star
 * behind a lane is dimmed by it. `density` scales the sky's and the band's
 * star counts (1 by default; `PHOTO_STAR_DENSITY` for a long exposure).
 */
export function starField(look: SkyLook = FIXED_SKY, dust?: (dir: Vec3) => number, density = 1): StarField {
  return look.coreBrightness > 0 ? seededField(look, dust, density) : legacyField(look, dust, density);
}

/** Stars in the system's sky across the whole sphere, at the Sun's place (before the brightness law). */
export const SEEDED_STARS = { sky: 6000, band: 48000, halo: 0.25, core: 28000, bulge: 6000 } as const;
/** Most stars a system's sky holds, whatever the exposure: the points are GPU work. */
export const SEEDED_STAR_MAX = 150_000;
/** The core's star swarm spreads over this many times the glow's width, and a third of it over three times that. */
const SWARM_SPREAD = 1.2;
const SWARM_TAIL = 0.35;

/** The standard normal draw (Box-Muller). */
function gaussian(next: () => number): number {
  return Math.sqrt(-2 * Math.log(1 - next())) * Math.cos(2 * Math.PI * next());
}

/**
 * The star counts of a system's sky: the whole-sky stars, the band's, the
 * core swarm and the wider bulge. The band's count follows the look's
 * brightness (never above 1.6 times the Sun's) and the swarm follows the
 * core's brightness, so the star density climbs toward the core and along the
 * band; a long exposure (`density` above 1) records more of the sky's and the
 * band's. The total is capped.
 */
export function seededStarCounts(look: SkyLook, density = 1): { sky: number; band: number; core: number; bulge: number } {
  const sky = SEEDED_STARS.sky * Math.sqrt(look.brightness) * density;
  const band = SEEDED_STARS.band * Math.min(1.6, Math.max(BAND_FLOOR, look.brightness)) * density;
  const core = SEEDED_STARS.core * Math.sqrt(look.coreBrightness);
  const bulge = SEEDED_STARS.bulge * look.bulge;
  const scale = Math.min(1, SEEDED_STAR_MAX / (sky + band + core + bulge));
  return { sky: Math.floor(sky * scale), band: Math.floor(band * scale), core: Math.floor(core * scale), bulge: Math.floor(bulge * scale) };
}

/**
 * A system's star field: faint stars everywhere, a band of countless faint
 * stars thicker toward the centre's side, a dense warm swarm at the core and
 * a looser bulge round it. The glow reads as unresolved stars because the
 * stars themselves are the glow.
 */
function seededField(look: SkyLook, dust: ((dir: Vec3) => number) | undefined, density: number): StarField {
  const next = random(look.seed);
  const counts = seededStarCounts(look, density);
  const skyCount = counts.sky;
  const bandCount = counts.band;
  const coreCount = counts.core;
  const count = skyCount + bandCount + coreCount + counts.bulge;
  const directions = new Float32Array(count * 3);
  const colours = new Float32Array(count * 3);
  const sizes = new Float32Array(count);
  const north = look.north;
  const band = frameAbout(north, look.centre);
  const bulge = frameAbout(look.centre, north);
  const sigmaLon = SWARM_SPREAD * 0.5 * look.coreSize;
  const sigmaLat = 0.6 * sigmaLon;
  for (let i = 0; i < count; i += 1) {
    const inBand = i >= skyCount && i < skyCount + bandCount;
    const inCore = i >= skyCount + bandCount && i < skyCount + bandCount + coreCount;
    const inBulge = i >= skyCount + bandCount + coreCount;
    let x: number;
    let y: number;
    let z: number;
    if (inBulge) {
      const angle = BULGE_WIDTH * Math.sqrt(-2 * Math.log(1 - next()));
      const azimuth = next() * 2 * Math.PI;
      const along = Math.cos(angle);
      const across = Math.sin(angle);
      const a = across * Math.cos(azimuth);
      const b = across * Math.sin(azimuth);
      x = along * look.centre[0] + a * bulge.u[0] + b * bulge.w[0];
      y = along * look.centre[1] + a * bulge.u[1] + b * bulge.w[1];
      z = along * look.centre[2] + a * bulge.u[2] + b * bulge.w[2];
    } else if (inCore) {
      // An elongated gaussian swarm on the centre (longitude along the plane), with a wider tail.
      const widen = next() < SWARM_TAIL ? 3 : 1;
      const lon = gaussian(next) * sigmaLon * widen;
      const lat = gaussian(next) * sigmaLat * widen;
      const bx = Math.cos(lat) * Math.cos(lon);
      const by = Math.sin(lat);
      const bz = Math.cos(lat) * Math.sin(lon);
      x = bx * band.u[0] + by * north[0] + bz * band.w[0];
      y = bx * band.u[1] + by * north[1] + bz * band.w[1];
      z = bx * band.u[2] + by * north[2] + bz * band.w[2];
    } else if (inBand) {
      // The plane's stars: thicker on the centre's side (longitude 0), a quarter of them in a wider halo.
      let lon = 0;
      for (let t = 0; t < 8; t += 1) {
        lon = (next() * 2 - 1) * Math.PI;
        if (next() < 0.5 + 0.5 * Math.exp(-(lon * lon) / (2 * 0.9 * 0.9))) break;
      }
      const lat = look.bandWidth * gaussian(next) * (next() < SEEDED_STARS.halo ? 2.5 : 1);
      const bx = Math.cos(lat) * Math.cos(lon);
      const by = Math.sin(lat);
      const bz = Math.cos(lat) * Math.sin(lon);
      x = bx * band.u[0] + by * north[0] + bz * band.w[0];
      y = bx * band.u[1] + by * north[1] + bz * band.w[1];
      z = bx * band.u[2] + by * north[2] + bz * band.w[2];
    } else {
      y = 2 * next() - 1;
      const r = Math.sqrt(1 - y * y);
      const lon = next() * 2 * Math.PI;
      x = r * Math.cos(lon);
      z = r * Math.sin(lon);
    }
    directions.set([x, y, z], i * 3);
    // Mostly faint, a few bright; the dense populations are fainter still, and the core's are old, so warm.
    const dense = inBand || inCore || inBulge;
    const brightness = next() ** (dense ? 6 : 4);
    let pick = next();
    let colour = PALETTE[PALETTE.length - 1]!.colour;
    for (const entry of PALETTE) {
      if (pick < entry.weight) {
        colour = entry.colour;
        break;
      }
      pick -= entry.weight;
    }
    if (inCore || inBulge) colour = colour.map((c, k) => c * SWARM_TINT[k]!) as unknown as typeof colour;
    let intensity = (dense ? 0.15 : 0.25) + 0.75 * brightness;
    if (dust) intensity *= 1 - dust([x, y, z]);
    colours.set(
      colour.map((c) => c * intensity),
      i * 3,
    );
    sizes[i] = 1 + 2.2 * brightness;
  }
  return { directions, colours, sizes };
}

/** The core's and bulge's stars are old and warm: their palette colours are multiplied by this. */
const SWARM_TINT: readonly [number, number, number] = [1, 0.88, 0.7];

function legacyField(look: SkyLook, dust: ((dir: Vec3) => number) | undefined, density: number): StarField {
  const next = random(look.seed);
  const skyCount = Math.round(STAR_COUNTS.sky * Math.sqrt(look.brightness) * density);
  const bandCount = Math.round(STAR_COUNTS.band * Math.max(BAND_FLOOR, look.brightness) * density);
  const bulgeCount = Math.round(BULGE_STARS * look.bulge);
  const count = skyCount + bandCount + bulgeCount;
  const directions = new Float32Array(count * 3);
  const colours = new Float32Array(count * 3);
  const sizes = new Float32Array(count);
  // The band's frame: latitude along the north pole, longitude from the centre.
  const north = look.north;
  const band = frameAbout(north, look.centre);
  // The bulge's frame: around the centre.
  const bulge = frameAbout(look.centre, north);
  for (let i = 0; i < count; i += 1) {
    const inBand = i >= skyCount;
    const inBulge = i >= skyCount + bandCount;
    let x: number;
    let y: number;
    let z: number;
    if (inBulge) {
      // A round cluster: angle from the centre from a Rayleigh law, any azimuth.
      const angle = BULGE_WIDTH * Math.sqrt(-2 * Math.log(1 - next()));
      const azimuth = next() * 2 * Math.PI;
      const along = Math.cos(angle);
      const across = Math.sin(angle);
      const a = across * Math.cos(azimuth);
      const b = across * Math.sin(azimuth);
      x = along * look.centre[0] + a * bulge.u[0] + b * bulge.w[0];
      y = along * look.centre[1] + a * bulge.u[1] + b * bulge.w[1];
      z = along * look.centre[2] + a * bulge.u[2] + b * bulge.w[2];
    } else if (inBand) {
      // Around the band's great circle: latitude from a normal law (Box-Muller).
      const lon = next() * 2 * Math.PI;
      const lat = look.bandWidth * Math.sqrt(-2 * Math.log(1 - next())) * Math.cos(2 * Math.PI * next());
      const bx = Math.cos(lat) * Math.cos(lon);
      const by = Math.sin(lat);
      const bz = Math.cos(lat) * Math.sin(lon);
      x = bx * band.u[0] + by * north[0] + bz * band.w[0];
      y = bx * band.u[1] + by * north[1] + bz * band.w[1];
      z = bx * band.u[2] + by * north[2] + bz * band.w[2];
    } else {
      // Uniform on the sphere.
      y = 2 * next() - 1;
      const r = Math.sqrt(1 - y * y);
      const lon = next() * 2 * Math.PI;
      x = r * Math.cos(lon);
      z = r * Math.sin(lon);
    }
    directions.set([x, y, z], i * 3);
    // Brightness: many faint stars, few bright ones; the band's are fainter.
    const brightness = next() ** (inBand ? 6 : 4);
    let pick = next();
    let colour = PALETTE[PALETTE.length - 1]!.colour;
    for (const entry of PALETTE) {
      if (pick < entry.weight) {
        colour = entry.colour;
        break;
      }
      pick -= entry.weight;
    }
    let intensity = (inBand ? 0.18 : 0.25) + 0.75 * brightness;
    if (dust) intensity *= 1 - dust([x, y, z]);
    colours.set(
      colour.map((c) => c * intensity),
      i * 3,
    );
    sizes[i] = 1 + 2.2 * brightness;
  }
  return { directions, colours, sizes };
}
