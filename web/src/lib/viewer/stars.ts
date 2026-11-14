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
