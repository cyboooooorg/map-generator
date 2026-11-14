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
import { nebulaDustAt, skyNebulae, type NebulaKind } from './skyNebulae';
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
  /** Colours (linear RGB weights) of the diffuse glow, H-alpha, O III and reflection light. */
  glowColour: Vec3;
  haColour: Vec3;
  oiiiColour: Vec3;
  refColour: Vec3;
  /** Clumpiness of the diffuse glow: 0 is the first version's soft noise, 1 is thresholded clumps with dark gaps. */
  clumps: number;
  /** Weight of the reflection patches against the emission ones. */
  reflection: number;
  /** Thresholds of the emission patches: a narrow, high window makes few, sharp-edged patches. */
  emission: [number, number];
}

const EXPOSURES: Record<SkyExposure, Exposure> = {
  eye: {
    ceiling: CLOUD_CEILING,
    glow: 0.6,
    patch: 1,
    core: 9,
    saturation: 0.45,
    glowColour: [1, 0.94, 0.84],
    haColour: [1, 0.15, 0.1],
    oiiiColour: [0.1, 0.8, 0.7],
    refColour: [0.35, 0.5, 1],
    clumps: 0,
    reflection: 0.7,
    emission: [0.5, 0.75],
  },
  // A long exposure: a warm dust glow in clumps with dark gaps, deep H-alpha reds and O III teals, colours pushed past the sum.
  photo: {
    ceiling: 0.3,
    glow: 0.28,
    patch: 2.2,
    core: 6,
    saturation: 1.5,
    glowColour: [1, 0.8, 0.6],
    haColour: [1, 0.06, 0.1],
    oiiiColour: [0.02, 0.85, 0.8],
    refColour: [0.2, 0.4, 1],
    clumps: 1,
    reflection: 0.25,
    emission: [0.55, 0.8],
  },
};

/** Peak linear level of the core's glow at the Sun's place, before the exposure gain and the soft curve. */
const CORE_LEVEL = 0.07;
/** The core's colour: a warm yellow-white, an old stellar population. */
const CORE_COLOUR: Vec3 = [1, 0.72, 0.36];
/** Where the core's soft curve levels off, as a share of the exposure's ceiling. */
const CORE_PEAK = 1;
/** The core's latitude over its longitude extent. */
const CORE_AXIS_RATIO = 0.6;

/** Cloud glow spread about the band, in band widths: clouds are broader than the stars. */
const CLOUD_SPREAD = 2.4;
/** Peak linear level of the diffuse glow at the Sun's place. */
const GLOW_LEVEL = 0.028;
/** Peak linear level of a nebula's faint coloured halo, before the exposure gain. */
const HALO_LEVEL = 0.03;
/** A halo reaches this many nebula radii, and its colour weights (H-alpha, O III, reflection) per kind. */
const HALO_REACH = 2.4;
const HALO_MIX: Record<NebulaKind, Vec3> = {
  planetary: [0.1, 1, 0.3],
  emission: [1, 0.35, 0.1],
  pillars: [1, 0.4, 0.1],
  remnant: [0.7, 0.8, 0],
  reflection: [0, 0, 1],
  dark: [0, 0, 0],
};
/** Peak linear level of the emission and reflection patches at the Sun's place. */
const PATCH_LEVEL = 0.045;
/** The band's own coloured patches are a few small spots: their share of the patch level (the colour sits in the nebulae's haloes). */
const BAND_PATCH_SHARE = 0.18;
/**
 * The cloud level follows the look's brightness within these bounds: far out
 * in the disc the clouds stay visible (a floor of 0.35 of the Sun's place),
 * and near the centre they do not outshine the stars.
 */
const CLOUD_LEVEL_MIN = 0.35;
const CLOUD_LEVEL_MAX = 1.3;
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

/**
 * The core's dust lane: not one thin line but a main strand and a second one
 * that wander about the plane with the longitude (the Great Rift forks), both
 * ragged, fading toward the core's ends.
 */
function coreLane(look: SkyLook, dir: Vec3): number {
  if (look.coreBrightness <= 0) return 0;
  const frame = bandFrame(look);
  const x = dot(dir, frame.along);
  const lat = Math.asin(Math.max(-1, Math.min(1, dot(dir, frame.north))));
  const lon = Math.atan2(dot(dir, frame.across), x);
  const sigmaLat = 0.075 * look.coreSize;
  // Windowed to zero between 3 and 4 sigma of the widest wander: no step where the lane ends.
  const reach = sigmaLat * 4.5;
  const window = 1 - smoothstep(3 * reach, 4 * reach, Math.abs(lat));
  if (window <= 0) return 0;
  const n = noiseOf(look.seed);
  const o = n.offsets[2]!;
  const ragged = 0.55 + 0.9 * fbm(n, dir[0] * 9 + o[0], dir[1] * 9 + o[1], dir[2] * 9 + o[2], 2);
  const wander = (k: number) => 2 * fbm(n, lon * 2.3 + o[k]!, o[(k + 1) % 3]! + k, 0.5, 3) - 1;
  const along = Math.exp(-(lon * lon) / (2 * (0.9 * look.coreSize) ** 2));
  const strand = (offset: number, width: number) => Math.exp(-((lat - offset) ** 2) / (2 * (sigmaLat * width) ** 2));
  const a = strand(sigmaLat * 3.2 * wander(0), 1);
  const b = 0.8 * strand(sigmaLat * (1.6 + 4 * wander(1)), 0.7);
  const c = 0.6 * strand(-sigmaLat * (2.2 + 4 * wander(2)), 0.6);
  // A soft ceiling at 0.8, not a clamp: no kink at the lane's darkest.
  const depth = 0.8 * ragged * along * Math.max(a, b, c) * window;
  return 0.9 * Math.tanh(depth / 0.9);
}

/** Ridged noise in [0, 1]: 1 along thin curves where the noise crosses its middle, falling off either side. */
const ridge = (f: number) => 1 - Math.abs(2 * f - 1);

/**
 * The band's rifts: filaments that branch. A warped ridged noise draws the
 * main rifts; finer ridges only grow next to them, so they read as side
 * branches; a mottled haze fills between. Thick toward the core, thin out
 * at the band's edge.
 */
function rifts(look: SkyLook, dir: Vec3): number {
  const n = noiseOf(look.seed);
  const o = n.offsets[3]!;
  const p = n.offsets[1]!;
  const toCore = Math.acos(Math.min(1, Math.max(-1, dot(dir, look.centre))));
  const core = Math.exp(-((toCore / 0.9) ** 2)) * Math.min(1, look.coreBrightness ** 0.5 + 0.3);
  // Wider in latitude near the core, where the rifts are thickest.
  const lat = Math.asin(Math.max(-1, Math.min(1, dot(dir, look.north))));
  const sigma = look.bandWidth * 0.8 * (1 + 1.2 * core);
  const env = Math.exp(-(lat * lat) / (2 * sigma * sigma));
  if (env < 1e-3) return 0;
  // Domain warp: the rifts' coordinates are bent twice, at two scales.
  const w = (k: number, f: number) => fbm(n, dir[0] * f + p[k % 3]!, dir[1] * f + p[(k + 1) % 3]!, dir[2] * f + p[(k + 2) % 3]!, 2) - 0.5;
  const qx = dir[0] * 3 + 1.4 * w(0, 1.7);
  const qy = dir[1] * 3 + 1.4 * w(1, 1.7);
  const qz = dir[2] * 3 + 1.4 * w(2, 1.7);
  const main = ridge(fbm(n, qx + o[0], qy + o[1], qz + o[2], 3));
  const rx = qx * 2.6 + 0.9 * w(3, 4.2);
  const ry = qy * 2.6 + 0.9 * w(4, 4.2);
  const rz = qz * 2.6 + 0.9 * w(5, 4.2);
  const side = ridge(fbm(n, rx + o[1], ry + o[2], rz + o[0], 3));
  const fine = ridge(fbm(n, rx * 2.3 + o[2], ry * 2.3 + o[0], rz * 2.3 + o[1], 3));
  const trunk = smoothstep(0.88, 0.97, main);
  // Branches grow only near a trunk (its broad halo) and thin out away from it.
  const near = smoothstep(0.7, 0.93, main);
  const branch = 0.85 * smoothstep(0.9, 0.975, side) * near;
  const twig = 0.65 * smoothstep(0.92, 0.985, fine) * near * near;
  const haze = 0.3 * smoothstep(0.5, 0.75, fbm(n, qx * 1.5 + o[1], qy * 1.5 + o[2], qz * 1.5 + o[0], 3));
  const lanes = Math.max(trunk, branch, twig, haze);
  return Math.min(1, lanes * env);
}

/**
 * Dust extinction toward a unit direction, 0 (clear) to 1 (opaque; never
 * reached): the band's lanes, the core's lane, dark nebulae and pillar
 * silhouettes.
 */
export function dustAt(look: SkyLook, dir: Vec3): number {
  const extra = 1 - (1 - nebulaDustAt(look, dir)) * (1 - coreLane(look, dir));
  const band = Math.min(1, DUST_MAX * rifts(look, dir));
  return 1 - (1 - band) * (1 - extra);
}

/** Linear value to an sRGB byte. */
function encode(linear: number): number {
  const v = linear <= 0.0031308 ? 12.92 * linear : 1.055 * linear ** (1 / 2.4) - 0.055;
  return Math.round(255 * Math.min(1, Math.max(0, v)));
}

/**
 * The linear colour of the clouds toward a unit direction, after the soft
 * ceiling (what `bake` encodes), as a function of the direction alone: no
 * face, no pixel grid, so it can be sampled along any curve.
 *
 * The glow is windowed to zero by a smoothstep between 3 and 4 sigma (the
 * Gaussian is then under 1% and 0.03% of its peak): a hard cutoff there drew
 * a straight edge across the views (a step of ~2% of the peak).
 */
export function cloudSampler(look: SkyLook, exposure: SkyExposure): (dir: Vec3) => Vec3 {
  const ex = EXPOSURES[exposure];
  const frame = bandFrame(look);
  const coreOn = look.coreBrightness > 0;
  const sigmaLon = 0.5 * look.coreSize;
  const sigmaLat = CORE_AXIS_RATIO * sigmaLon;
  const n = noiseOf(look.seed);
  const sigma = look.bandWidth * CLOUD_SPREAD;
  const cutoff = Math.max(4 * sigma, coreOn ? 4.5 * sigmaLat : 0);
  const [wHa, wO3, wRef] = n.mix;
  const [oa, ob, oc] = n.offsets as [Vec3, Vec3, Vec3, Vec3];
  const level = Math.min(CLOUD_LEVEL_MAX, Math.max(CLOUD_LEVEL_MIN, look.brightness));
  const bandNoise = 0.6;
  const halos = skyNebulae(look)
    .filter((nb) => nb.kind !== 'dark')
    .map((nb) => ({ nb, reach: HALO_REACH * nb.radius, cos: Math.cos(HALO_REACH * nb.radius), mix: HALO_MIX[nb.kind] }));
  /** The faint coloured haloes round the nebulae, wherever they are on the sphere: linear RGB. */
  const haloAt = (dir: Vec3): Vec3 => {
    let r = 0;
    let g = 0;
    let b = 0;
    for (const h of halos) {
      const c = dot(dir, h.nb.dir);
      if (c < h.cos) continue;
      const x = Math.acos(Math.min(1, c)) / h.reach;
      // Smooth to zero at the reach, brightest toward the nebula; cloudy, never a disc.
      const fall = Math.exp(-3.2 * x * x) * (1 - smoothstep(0.7, 1, x));
      const cloud = 0.25 + 1.5 * smoothstep(0.3, 0.75, fbm(n, dir[0] * 5 + h.nb.seed * 1e-9 + oa[0], dir[1] * 5 + oa[1], dir[2] * 5 + oa[2] + (h.nb.seed % 97), 3));
      const k = HALO_LEVEL * ex.patch * fall * cloud * (0.6 + 0.8 * h.nb.tint);
      r += k * (h.mix[0] * ex.haColour[0] + h.mix[1] * ex.oiiiColour[0] + h.mix[2] * ex.refColour[0]);
      g += k * (h.mix[0] * ex.haColour[1] + h.mix[1] * ex.oiiiColour[1] + h.mix[2] * ex.refColour[1]);
      b += k * (h.mix[0] * ex.haColour[2] + h.mix[1] * ex.oiiiColour[2] + h.mix[2] * ex.refColour[2]);
    }
    return [r, g, b];
  };
  const squash = (r: number, g: number, b: number): Vec3 => [ex.ceiling * Math.tanh(r / ex.ceiling), ex.ceiling * Math.tanh(g / ex.ceiling), ex.ceiling * Math.tanh(b / ex.ceiling)];
  return (dir) => {
    const halo = haloAt(dir);
    const trueLat = Math.asin(Math.max(-1, Math.min(1, dot(dir, look.north))));
    // The band's edge is never a smooth great circle: its latitude is bent by slow and medium noise.
    const bend =
      sigma * (bandNoise * (2 * fbm(n, dir[0] * 1.3 + oc[1], dir[1] * 1.3 + oc[2], dir[2] * 1.3 + oc[0], 3) - 1) +
        0.25 * (2 * fbm(n, dir[0] * 4 + ob[1], dir[1] * 4 + ob[2], dir[2] * 4 + ob[0], 3) - 1));
    const lat = trueLat + bend;
    const window = 1 - smoothstep(0.75 * cutoff, cutoff, Math.abs(trueLat));
    if (window <= 0) return squash(halo[0], halo[1], halo[2]);
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
    const soft = fbm(n, px + oa[0], py + oa[1], pz + oa[2], 3);
    const glowNoise = (1 - ex.clumps) * (0.55 + 0.9 * soft) + ex.clumps * (0.2 + 1.6 * smoothstep(0.3, 0.7, soft));
    const emission = smoothstep(ex.emission[0], ex.emission[1], fbm(n, px * 1.7 + ob[0], py * 1.7 + ob[1], pz * 1.7 + ob[2], 5));
    const reflection = smoothstep(0.52, 0.78, fbm(n, px * 2.1 + oc[0], py * 2.1 + oc[1], pz * 2.1 + oc[2], 4));
    const o3 = smoothstep(0.45, 0.7, fbm(n, px * 1.3 + oc[1], py * 1.3 + oc[2], pz * 1.3 + oc[0], 2));
    const strength = level * inBand * (1 + 1.5 * bulge);
    // A fine grain: the glow is unresolved stars, not a smooth wash.
    const grain = 0.7 + 0.6 * value(n, dir[0] * 110 + oc[2], dir[1] * 110 + oc[0], dir[2] * 110 + oc[1]);
    const glow = GLOW_LEVEL * ex.glow * strength * glowNoise * grain;
    const patch = PATCH_LEVEL * BAND_PATCH_SHARE * ex.patch * strength;
    // Inside a patch the light is uneven: bright knots and thinner veils.
    const detail = 0.35 + 1.3 * fbm(n, px * 6 + oa[1], py * 6 + oa[2], pz * 6 + oa[0], 4);
    const ha = patch * wHa * emission * detail * (1 - o3);
    const oiii = patch * wO3 * emission * detail * o3;
    const ref = patch * wRef * reflection * ex.reflection;
    const clear = (1 - dustAt(look, dir)) * window;
    let r = 0;
    let g = 0;
    let b = 0;
    r += glow * ex.glowColour[0] + ha * ex.haColour[0] + oiii * ex.oiiiColour[0] + ref * ex.refColour[0];
    g += glow * ex.glowColour[1] + ha * ex.haColour[1] + oiii * ex.oiiiColour[1] + ref * ex.refColour[1];
    b += glow * ex.glowColour[2] + ha * ex.haColour[2] + oiii * ex.oiiiColour[2] + ref * ex.refColour[2];
    r = (r + halo[0]) * clear;
    g = (g + halo[1]) * clear;
    b = (b + halo[2]) * clear;
    // Washed-out colours for the eye, pushed ones for the long exposure: mix away from or toward the luminance.
    const lum = 0.2126 * r + 0.7152 * g + 0.0722 * b;
    r = Math.max(0, lum + (r - lum) * ex.saturation);
    g = Math.max(0, lum + (g - lum) * ex.saturation);
    b = Math.max(0, lum + (b - lum) * ex.saturation);
    if (coreOn) {
      // The core: an elongated warm glow, mottled, with a brighter nucleus. Its longitude is
      // far past its width on the far side: no half-sphere gate. A soft curve (not a clip)
      // keeps a gradient to the middle at any brightness, and its own lane darkens it after
      // the curve, so the lane shows even where the glow is brightest.
      const lon = Math.atan2(dot(dir, frame.across), dot(dir, frame.along));
      const q = (lon / sigmaLon) ** 2 + (trueLat / sigmaLat) ** 2;
      const mottle = 0.8 + 0.4 * fbm(n, dir[0] * 7 + oc[0], dir[1] * 7 + oc[1], dir[2] * 7 + oc[2], 3);
      const raw = CORE_LEVEL * ex.core * look.coreBrightness ** 0.4 * (0.5 * Math.exp(-q / 2) + Math.exp(-1.6 * Math.sqrt(q))) * mottle;
      const peak = CORE_PEAK * ex.ceiling;
      const x = (raw / peak) ** 0.9;
      const core = ((peak * x) / (1 + x)) * (1 - Math.min(0.95, 1.2 * coreLane(look, dir))) * window;
      r += core * CORE_COLOUR[0];
      g += core * CORE_COLOUR[1];
      b += core * CORE_COLOUR[2];
    }
    return squash(r, g, b);
  };
}

function bake(look: SkyLook, exposure: SkyExposure, size: number): SkyFaces {
  const sample = cloudSampler(look, exposure);
  const faces: Uint8Array[] = [];
  for (let face = 0; face < 6; face += 1) {
    const pixels = new Uint8Array(size * size * 4);
    for (let row = 0; row < size; row += 1) {
      for (let col = 0; col < size; col += 1) {
        const at = (row * size + col) * 4;
        const [r, g, b] = sample(faceDirection(face, (2 * (col + 0.5)) / size - 1, (2 * (row + 0.5)) / size - 1));
        pixels[at] = encode(r);
        pixels[at + 1] = encode(g);
        pixels[at + 2] = encode(b);
        pixels[at + 3] = 255;
      }
    }
    faces.push(pixels);
  }
  return { size, faces };
}

const bakes = new Map<string, SkyFaces>();

/** The look's cloud cube map in an exposure; the last four are kept. */
export function bakeSkyClouds(look: SkyLook, exposure: SkyExposure, size = 512): SkyFaces {
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
