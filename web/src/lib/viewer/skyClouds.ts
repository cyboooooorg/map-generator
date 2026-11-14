// Faint gas and dust clouds of a system's sky, baked into a cube map.
//
// The band's glow, emission nebulae (H-alpha red, O III blue-green) and
// blue reflection patches come from seeded fractal noise with domain
// warping, kept along the band (a gaussian in latitude about the galactic
// north). Dust lanes darken the glow; `dustAt` is shared with the star
// field so the stars behind a lane dim with it. The galactic core is an
// elongated glow (axis ratio 0.6) crossed by a dark lane along the plane.
//
// Two renderings share the noise: 'eye' (what the eye sees: no texel above
// `CLOUD_CEILING` in linear RGB, colours washed out) and 'photo' (a long
// exposure: brighter, saturated, up to `PHOTO_CEILING`). The cube is built
// in the system frame; a view rotates it.

import type { Vec3 } from '../skyBodies';
import { bandFrame, type SkyLook } from './skyLook';
import { random } from './skyRandom';

/** Six RGBA faces in Three's cube order (+x, -x, +y, -y, +z, -z), sRGB-encoded. */
export interface SkyFaces {
  size: number;
  faces: Uint8Array[];
}

/** No cloud texel reaches this linear value (a soft ceiling, never attained). */
export const CLOUD_CEILING = 0.075;

/** How the sky is rendered: as the eye sees it, or as a long exposure shows it. */
export type SkyExposure = 'eye' | 'photo';

/** Per-exposure gains: soft ceiling, glow and patch gain, core gain, colour saturation (1 = full). */
interface Exposure {
  ceiling: number;
  glow: number;
  patch: number;
  core: number;
  saturation: number;
}

const EXPOSURES: Record<SkyExposure, Exposure> = {
  eye: { ceiling: CLOUD_CEILING, glow: 1, patch: 1, core: 5, saturation: 0.45 },
  photo: { ceiling: 0.3, glow: 3.2, patch: 5, core: 10, saturation: 1 },
};

/** Peak linear level of the core's glow at the Sun's place, before the exposure gain. */
const CORE_LEVEL = 0.07;
/** The core's latitude over its longitude extent. */
const CORE_AXIS_RATIO = 0.6;

/** Cloud glow spread about the band, in band widths: clouds are broader than the stars. */
const CLOUD_SPREAD = 2.4;
/** Peak linear level of the diffuse glow at the Sun's place. */
const GLOW_LEVEL = 0.028;
/** Peak linear level of the emission and reflection patches at the Sun's place. */
const PATCH_LEVEL = 0.045;
/**
 * The cloud level follows the look's brightness within these bounds: far out
 * in the disc the clouds stay visible (a floor of 0.35 of the Sun's place),
 * and near the centre they do not outshine the stars.
 */
const CLOUD_LEVEL_MIN = 0.35;
const CLOUD_LEVEL_MAX = 1.6;
/** Dust never blocks more than this fraction of the light behind it. */
const DUST_MAX = 0.85;

/**
 * Direction of a cube texel, following the OpenGL / Three.js cube map
 * convention. `s` runs left to right and `t` top to bottom across the face
 * image, both in [-1, 1]; the result is normalised.
 *
 *   face 0 (+x): ( 1, -t, -s)    face 1 (-x): (-1, -t,  s)
 *   face 2 (+y): ( s,  1,  t)    face 3 (-y): ( s, -1, -t)
 *   face 4 (+z): ( s, -t,  1)    face 5 (-z): (-s, -t, -1)
 */
export function faceDirection(face: number, s: number, t: number): Vec3 {
  let x: number;
  let y: number;
  let z: number;
  switch (face) {
    case 0:
      [x, y, z] = [1, -t, -s];
      break;
    case 1:
      [x, y, z] = [-1, -t, s];
      break;
    case 2:
      [x, y, z] = [s, 1, t];
      break;
    case 3:
      [x, y, z] = [s, -1, -t];
      break;
    case 4:
      [x, y, z] = [s, -t, 1];
      break;
    default:
      [x, y, z] = [-s, -t, -1];
  }
  const length = Math.hypot(x, y, z);
  return [x / length, y / length, z / length];
}

/** Lattice value noise of one seed. */
interface Noise {
  perm: Uint8Array;
  values: Float32Array;
  /** Offsets of the three fields drawn from the seed, so they do not align. */
  offsets: Vec3[];
  /** Colour weights: H-alpha, O III, reflection. */
  mix: Vec3;
}

const noiseCache = new Map<number, Noise>();

function noiseOf(seed: number): Noise {
  const cached = noiseCache.get(seed);
  if (cached) return cached;
  const next = random(seed ^ 0x5bd1e995);
  const perm = new Uint8Array(512);
  const order = Array.from({ length: 256 }, (_, i) => i);
  for (let i = 255; i > 0; i -= 1) {
    const j = Math.floor(next() * (i + 1));
    [order[i], order[j]] = [order[j]!, order[i]!];
  }
  for (let i = 0; i < 512; i += 1) perm[i] = order[i & 255]!;
  const values = new Float32Array(256);
  for (let i = 0; i < 256; i += 1) values[i] = next();
  const offsets: Vec3[] = [0, 1, 2, 3].map(() => [next() * 50, next() * 50, next() * 50] as Vec3);
  const mix: Vec3 = [0.5 + next(), 0.4 + next(), 0.5 + next()];
  const noise = { perm, values, offsets, mix };
  if (noiseCache.size >= 16) noiseCache.clear();
  noiseCache.set(seed, noise);
  return noise;
}

/** Value noise in [0, 1] at a point. */
function value(n: Noise, x: number, y: number, z: number): number {
  const xi = Math.floor(x);
  const yi = Math.floor(y);
  const zi = Math.floor(z);
  const fx = x - xi;
  const fy = y - yi;
  const fz = z - zi;
  const u = fx * fx * (3 - 2 * fx);
  const v = fy * fy * (3 - 2 * fy);
  const w = fz * fz * (3 - 2 * fz);
  const p = n.perm;
  const vs = n.values;
  const x0 = xi & 255;
  const y0 = yi & 255;
  const z0 = zi & 255;
  const corner = (dx: number, dy: number, dz: number) =>
    vs[p[(p[(p[x0 + dx]! + y0 + dy) & 255]! + z0 + dz) & 255]!]!;
  const a = corner(0, 0, 0) + u * (corner(1, 0, 0) - corner(0, 0, 0));
  const b = corner(0, 1, 0) + u * (corner(1, 1, 0) - corner(0, 1, 0));
  const c = corner(0, 0, 1) + u * (corner(1, 0, 1) - corner(0, 0, 1));
  const d = corner(0, 1, 1) + u * (corner(1, 1, 1) - corner(0, 1, 1));
  const e = a + v * (b - a);
  const f = c + v * (d - c);
  return e + w * (f - e);
}

/** Fractal noise in [0, 1) with the given octave count. */
function fbm(n: Noise, x: number, y: number, z: number, octaves: number): number {
  let sum = 0;
  let amplitude = 0.5;
  let norm = 0;
  let fx = x;
  let fy = y;
  let fz = z;
  for (let i = 0; i < octaves; i += 1) {
    sum += amplitude * value(n, fx, fy, fz);
    norm += amplitude;
    amplitude *= 0.5;
    fx = fx * 2.03 + 11.7;
    fy = fy * 2.03 + 5.3;
    fz = fz * 2.03 + 2.9;
  }
  return sum / norm;
}

const smoothstep = (a: number, b: number, x: number) => {
  const t = Math.min(1, Math.max(0, (x - a) / (b - a)));
  return t * t * (3 - 2 * t);
};

const dot = (a: Vec3, b: Vec3) => a[0] * b[0] + a[1] * b[1] + a[2] * b[2];

/** The core's dust lane: a thin dark band along the plane, ragged, fading toward the core's ends. */
function coreLane(look: SkyLook, dir: Vec3): number {
  if (look.coreBrightness <= 0) return 0;
  const frame = bandFrame(look);
  const x = dot(dir, frame.along);
  if (x <= 0) return 0;
  const lat = Math.asin(Math.max(-1, Math.min(1, dot(dir, frame.north))));
  const lon = Math.atan2(dot(dir, frame.across), x);
  const sigmaLat = 0.1 * look.coreSize;
  if (Math.abs(lat) > 4 * sigmaLat) return 0;
  const n = noiseOf(look.seed);
  const o = n.offsets[2]!;
  const ragged = 0.55 + 0.9 * fbm(n, dir[0] * 9 + o[0], dir[1] * 9 + o[1], dir[2] * 9 + o[2], 2);
  const along = Math.exp(-(lon * lon) / (2 * (0.8 * look.coreSize) ** 2));
  return Math.min(0.8, 0.7 * ragged * along * Math.exp(-(lat * lat) / (2 * sigmaLat * sigmaLat)));
}

/**
 * Dust extinction toward a unit direction, 0 (clear) to 1 (opaque; never
 * reached): the band's lanes, the core's lane, dark nebulae and pillar
 * silhouettes.
 */
export function dustAt(look: SkyLook, dir: Vec3): number {
  const extra = 1 - (1 - coreLane(look, dir));
  const lat = Math.asin(Math.max(-1, Math.min(1, dot(dir, look.north))));
  const sigma = look.bandWidth * 0.8;
  const inBand = Math.exp(-(lat * lat) / (2 * sigma * sigma));
  if (inBand < 0.01) return extra;
  const n = noiseOf(look.seed);
  const o = n.offsets[3]!;
  // Lanes: thresholded noise, stronger toward the centre.
  const lanes = smoothstep(0.5, 0.7, fbm(n, dir[0] * 4 + o[0], dir[1] * 4 + o[1], dir[2] * 4 + o[2], 3));
  const toCentre = Math.max(0, dot(dir, look.centre));
  const band = Math.min(1, DUST_MAX * inBand * lanes * (0.6 + 0.4 * toCentre));
  return 1 - (1 - band) * (1 - extra);
}

/** Linear value to an sRGB byte. */
function encode(linear: number): number {
  const v = linear <= 0.0031308 ? 12.92 * linear : 1.055 * linear ** (1 / 2.4) - 0.055;
  return Math.round(255 * Math.min(1, Math.max(0, v)));
}

function bake(look: SkyLook, exposure: SkyExposure, size: number): SkyFaces {
  const ex = EXPOSURES[exposure];
  const frame = bandFrame(look);
  const coreOn = look.coreBrightness > 0;
  const sigmaLon = 0.5 * look.coreSize;
  const sigmaLat = CORE_AXIS_RATIO * sigmaLon;
  const n = noiseOf(look.seed);
  const sigma = look.bandWidth * CLOUD_SPREAD;
  // The glow is under 2% of its peak beyond (the core's, when there is one, reaches further).
  const cutoff = Math.max(2.8 * sigma, coreOn ? 3.2 * sigmaLat : 0);
  const [wHa, wO3, wRef] = n.mix;
  const [oa, ob, oc] = n.offsets as [Vec3, Vec3, Vec3, Vec3];
  const level = Math.min(CLOUD_LEVEL_MAX, Math.max(CLOUD_LEVEL_MIN, look.brightness));
  const faces: Uint8Array[] = [];
  for (let face = 0; face < 6; face += 1) {
    const pixels = new Uint8Array(size * size * 4);
    for (let row = 0; row < size; row += 1) {
      for (let col = 0; col < size; col += 1) {
        const at = (row * size + col) * 4;
        pixels[at + 3] = 255;
        const dir = faceDirection(face, (2 * (col + 0.5)) / size - 1, (2 * (row + 0.5)) / size - 1);
        const lat = Math.asin(Math.max(-1, Math.min(1, dot(dir, look.north))));
        if (Math.abs(lat) > cutoff) continue;
        const inBand = Math.exp(-(lat * lat) / (2 * sigma * sigma));
        const toCentre = Math.max(0, dot(dir, look.centre));
        const bulge = look.bulge * toCentre ** 4;
        // Domain warping: the patches' coordinates are bent by a slow noise.
        const wx = fbm(n, dir[0] * 1.6 + oa[0], dir[1] * 1.6 + oa[1], dir[2] * 1.6 + oa[2], 2) - 0.5;
        const wy = fbm(n, dir[0] * 1.6 + ob[0], dir[1] * 1.6 + ob[1], dir[2] * 1.6 + ob[2], 2) - 0.5;
        const wz = fbm(n, dir[0] * 1.6 + oc[0], dir[1] * 1.6 + oc[1], dir[2] * 1.6 + oc[2], 2) - 0.5;
        const px = dir[0] * 3 + 1.5 * wx;
        const py = dir[1] * 3 + 1.5 * wy;
        const pz = dir[2] * 3 + 1.5 * wz;
        const glowNoise = 0.55 + 0.9 * fbm(n, px + oa[0], py + oa[1], pz + oa[2], 3);
        const emission = smoothstep(0.5, 0.75, fbm(n, px * 1.7 + ob[0], py * 1.7 + ob[1], pz * 1.7 + ob[2], 3));
        const reflection = smoothstep(0.52, 0.78, fbm(n, px * 2.1 + oc[0], py * 2.1 + oc[1], pz * 2.1 + oc[2], 3));
        const o3 = smoothstep(0.45, 0.7, fbm(n, px * 1.3 + oc[1], py * 1.3 + oc[2], pz * 1.3 + oc[0], 2));
        const strength = level * inBand * (1 + 1.5 * bulge);
        const glow = GLOW_LEVEL * ex.glow * strength * glowNoise;
        const patch = PATCH_LEVEL * ex.patch * strength;
        const ha = patch * wHa * emission * (1 - o3);
        const oiii = patch * wO3 * emission * o3;
        const ref = patch * wRef * reflection * 0.7;
        // The core: an elongated warm glow, mottled, with a brighter nucleus.
        let coreGlow = 0;
        if (coreOn) {
          const along = dot(dir, frame.along);
          if (along > 0) {
            const lon = Math.atan2(dot(dir, frame.across), along);
            const q = (lon / sigmaLon) ** 2 + (lat / sigmaLat) ** 2;
            const mottle = 0.8 + 0.4 * fbm(n, dir[0] * 7 + oc[0], dir[1] * 7 + oc[1], dir[2] * 7 + oc[2], 3);
            coreGlow = CORE_LEVEL * ex.core * look.coreBrightness * (Math.exp(-q / 2) + 0.6 * Math.exp(-q * 2)) * mottle;
          }
        }
        const clear = 1 - dustAt(look, dir);
        let r = (glow * 1.0 + ha * 1.0 + oiii * 0.1 + ref * 0.35 + coreGlow * 1.0) * clear;
        let g = (glow * 0.92 + ha * 0.15 + oiii * 0.8 + ref * 0.5 + coreGlow * 0.82) * clear;
        let b = (glow * 0.8 + ha * 0.1 + oiii * 0.7 + ref * 1.0 + coreGlow * 0.55) * clear;
        // Washed-out colours for the eye: mix toward the luminance.
        const lum = 0.2126 * r + 0.7152 * g + 0.0722 * b;
        r = lum + (r - lum) * ex.saturation;
        g = lum + (g - lum) * ex.saturation;
        b = lum + (b - lum) * ex.saturation;
        pixels[at] = encode(ex.ceiling * Math.tanh(r / ex.ceiling));
        pixels[at + 1] = encode(ex.ceiling * Math.tanh(g / ex.ceiling));
        pixels[at + 2] = encode(ex.ceiling * Math.tanh(b / ex.ceiling));
      }
    }
    faces.push(pixels);
  }
  return { size, faces };
}

const bakes = new Map<string, SkyFaces>();

/** The look's cloud cube map in an exposure; the last four are kept. */
export function bakeSkyClouds(look: SkyLook, exposure: SkyExposure, size = 256): SkyFaces {
  const key = [exposure, size, look.coreSize, look.coreBrightness, look.seed, look.brightness, look.bandWidth, look.bulge, ...look.north, ...look.centre].join('|');
  const cached = bakes.get(key);
  if (cached) {
    bakes.delete(key);
    bakes.set(key, cached);
    return cached;
  }
  const faces = bake(look, exposure, size);
  bakes.set(key, faces);
  if (bakes.size > 4) bakes.delete(bakes.keys().next().value!);
  return faces;
}
