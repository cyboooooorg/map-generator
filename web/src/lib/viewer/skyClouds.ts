// Faint gas and dust clouds of a system's sky, baked into a cube map.
//
// The band's glow, emission nebulae (H-alpha red, O III blue-green) and
// blue reflection patches come from seeded fractal noise with domain
// warping, kept along the band (a gaussian in latitude about the galactic
// north). Dust lanes darken the glow; `dustAt` is shared with the star
// field so the stars behind a lane dim with it. Everything stays far
// below the band's own brightness: no texel above `CLOUD_CEILING` in
// linear RGB. The cube is built in the system frame; a view rotates it.

import type { Vec3 } from '../skyBodies';
import type { SkyLook } from './skyLook';

/** Six RGBA faces in Three's cube order (+x, -x, +y, -y, +z, -z), sRGB-encoded. */
export interface SkyFaces {
  size: number;
  faces: Uint8Array[];
}

/** No cloud texel reaches this linear value (a soft ceiling, never attained). */
export const CLOUD_CEILING = 0.075;

/** Cloud glow spread about the band, in band widths: clouds are broader than the stars. */
const CLOUD_SPREAD = 2.4;
/** Peak linear level of the diffuse glow at the Sun's place. */
const GLOW_LEVEL = 0.028;
/** Peak linear level of the emission and reflection patches at the Sun's place. */
const PATCH_LEVEL = 0.045;
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

/** Dust extinction toward a unit direction, 0 (clear) to 1 (opaque; never reached). */
export function dustAt(look: SkyLook, dir: Vec3): number {
  const lat = Math.asin(Math.max(-1, Math.min(1, dot(dir, look.north))));
  const sigma = look.bandWidth * 0.8;
  const inBand = Math.exp(-(lat * lat) / (2 * sigma * sigma));
  if (inBand < 0.01) return 0;
  const n = noiseOf(look.seed);
  const o = n.offsets[3]!;
  // Lanes: thresholded noise, stronger toward the centre.
  const lanes = smoothstep(0.5, 0.7, fbm(n, dir[0] * 4 + o[0], dir[1] * 4 + o[1], dir[2] * 4 + o[2], 3));
  const toCentre = Math.max(0, dot(dir, look.centre));
  return Math.min(1, DUST_MAX * inBand * lanes * (0.6 + 0.4 * toCentre));
}

/** Linear value to an sRGB byte. */
function encode(linear: number): number {
  const v = linear <= 0.0031308 ? 12.92 * linear : 1.055 * linear ** (1 / 2.4) - 0.055;
  return Math.round(255 * Math.min(1, Math.max(0, v)));
}

function bake(look: SkyLook, size: number): SkyFaces {
  const n = noiseOf(look.seed);
  const sigma = look.bandWidth * CLOUD_SPREAD;
  const cutoff = 2.8 * sigma; // the glow is under 2% of its peak beyond
  const [wHa, wO3, wRef] = n.mix;
  const [oa, ob, oc] = n.offsets as [Vec3, Vec3, Vec3, Vec3];
  const level = Math.min(1.6, look.brightness);
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
        const glow = GLOW_LEVEL * strength * glowNoise;
        const patch = PATCH_LEVEL * strength;
        const ha = patch * wHa * emission * (1 - o3);
        const oiii = patch * wO3 * emission * o3;
        const ref = patch * wRef * reflection * 0.7;
        const clear = 1 - dustAt(look, dir);
        const r = (glow * 1.0 + ha * 1.0 + oiii * 0.1 + ref * 0.35) * clear;
        const g = (glow * 0.92 + ha * 0.15 + oiii * 0.8 + ref * 0.5) * clear;
        const b = (glow * 0.8 + ha * 0.1 + oiii * 0.7 + ref * 1.0) * clear;
        pixels[at] = encode(CLOUD_CEILING * Math.tanh(r / CLOUD_CEILING));
        pixels[at + 1] = encode(CLOUD_CEILING * Math.tanh(g / CLOUD_CEILING));
        pixels[at + 2] = encode(CLOUD_CEILING * Math.tanh(b / CLOUD_CEILING));
      }
    }
    faces.push(pixels);
  }
  return { size, faces };
}

const bakes = new Map<string, SkyFaces>();

/** The look's cloud cube map; the last four looks are kept. */
export function bakeSkyClouds(look: SkyLook, size = 256): SkyFaces {
  const key = [size, look.seed, look.brightness, look.bandWidth, look.bulge, ...look.north, ...look.centre].join('|');
  const cached = bakes.get(key);
  if (cached) {
    bakes.delete(key);
    bakes.set(key, cached);
    return cached;
  }
  const faces = bake(look, size);
  bakes.set(key, faces);
  if (bakes.size > 4) bakes.delete(bakes.keys().next().value!);
  return faces;
}
