// Procedural textures of the sky's nebulae, one per kind.
//
// A texture is square and covers the plus and minus `radius` around the
// nebula's centre, in the frame of `nebulaBasis` (x right, y up; row 0 is
// the top), so the roll is not baked in. Pixels are sRGB bytes with
// premultiplied alpha. Every shape fades to zero before the texture's
// border: no visible square. Two exposures share the shapes: 'eye' keeps
// the faint ceiling of the clouds with washed-out colours, 'photo' is a
// long exposure with saturated H-alpha reds and O III blue-greens.

import { CLOUD_CEILING, type SkyExposure } from './skyClouds';
import { darkOpacity, pillarColumns, pillarDistance, pillarMask, type Nebula, type NebulaKind } from './skyNebulae';
import { fbm2, lattice, smoothstep } from './nebulaNoise';
import { subSeed } from './skyRandom';

export interface NebulaTexture {
  size: number;
  /** sRGB bytes, premultiplied alpha, row 0 at the top. */
  rgba: Uint8Array;
  /** 'add' for glows; 'darken' for dark nebulae (alpha is the opacity). */
  blend: 'add' | 'darken';
}

type Rgb = [number, number, number];

const H_ALPHA: Rgb = [1, 0.17, 0.2];
const O_III: Rgb = [0.12, 0.78, 0.82];
const BLUE: Rgb = [0.3, 0.5, 1];
const WHITE: Rgb = [1, 0.95, 0.9];

/** Per-exposure peak linear level of a unit-intensity texel, soft ceiling and colour saturation. */
const EXPOSURES: Record<SkyExposure, { level: number; ceiling: number; saturation: number; dark: number }> = {
  eye: { level: 0.06, ceiling: CLOUD_CEILING, saturation: 0.45, dark: 0.6 },
  // Over 1: the long exposure pushes the colours past what the sum gives.
  photo: { level: 0.5, ceiling: 0.3, saturation: 1.45, dark: 1 },
};
/** The faint eye shows some kinds better than others. */
const EYE_KIND_GAIN: Record<NebulaKind, number> = {
  planetary: 1.2,
  emission: 1,
  pillars: 0.8,
  remnant: 0.6,
  reflection: 0.8,
  dark: 1,
};

const mix = (a: Rgb, b: Rgb, t: number): Rgb => [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t];

/** Ridged noise in [0, 1]: 1 on thin curves, for wisps and filaments. */
const ridged = (x: number, y: number, s: number, octaves = 3) => 1 - Math.abs(2 * fbm2(x, y, s, octaves) - 1);

/** A texel's glow: an intensity (about 0 to 1.5) and its colour. */
interface Texel {
  i: number;
  colour: Rgb;
}

type Shader = (x: number, y: number, n: Nebula, s: number) => Texel;

/** Planetary: a bold ring (or two lobes) with an O III core and an H-alpha rim, knots, a faint outer halo and a central star. */
const planetary: Shader = (x, y, n, s) => {
  // Knots at two scales and fine radial-ish filaments, as in the Helix's cometary knots.
  const knots = (0.45 + 1.1 * fbm2(x * 6 + 3, y * 6 + 1, s, 3)) * (0.65 + 0.7 * ridged(x * 16 + 2, y * 16, s ^ 0x61, 3)) * (0.8 + 0.4 * fbm2(x * 40, y * 40, s ^ 0x62, 2));
  const star = 1.1 * Math.exp(-((x * x + y * y) / (0.03 * 0.03)));
  if (n.tint > 0.6) {
    // Bipolar: two shells joined at a pinched waist, along the vertical.
    let i = 0;
    let rim = 0;
    for (const side of [-1, 1]) {
      const d = Math.hypot(x / (0.3 - 0.04 * n.tint), (y - side * 0.36) / 0.38);
      const shell = Math.exp(-(((d - 0.9) / 0.2) ** 2)) * knots * 1.3;
      i += shell + 0.3 * Math.exp(-d * d * 1.6);
      rim = Math.max(rim, smoothstep(0.7, 1.15, d));
    }
    return { i: i + star, colour: mix(star > 0.1 ? WHITE : O_III, H_ALPHA, rim) };
  }
  const rho = 0.5;
  const thick = 0.1 + 0.04 * lattice(n.seed & 255, 3, s);
  const rr = Math.hypot(x, y * 1.1);
  const ring = Math.exp(-(((rr - rho) / thick) ** 2)) * knots * 1.3;
  const inner = 0.18 * (1 - smoothstep(0.1, rho, rr)) * smoothstep(0.02, 0.1, rr);
  // A fainter outer ring of broken arcs, as in the Helix.
  const arcs = 0.3 * Math.exp(-(((rr - rho - 0.2) / 0.06) ** 2)) * smoothstep(0.45, 0.65, fbm2(x * 3 + 8, y * 3, s + 7, 3));
  const halo = 0.12 * Math.exp(-(((rr - rho - 0.17) / 0.14) ** 2)) * (0.5 + fbm2(x * 3, y * 3, s + 5, 3));
  const rimMix = smoothstep(rho - 0.7 * thick, rho + 0.9 * thick, rr);
  return { i: ring + inner + arcs + halo + star, colour: star > 0.15 ? WHITE : mix(O_III, H_ALPHA, rimMix) };
};

/** Warped fractal glow of an emission complex at a texel, under an irregular envelope; `q` is the warp. */
function emissionGlow(x: number, y: number, s: number): { glow: number; q: number } {
  const q1 = fbm2(x * 1.6 + 3.1, y * 1.6 + 1.7, s ^ 0x51, 3) - 0.5;
  const q2 = fbm2(x * 1.6 + 7.7, y * 1.6 - 4.2, s ^ 0xa3, 3) - 0.5;
  const f = fbm2(x * 2.2 + 2.6 * q1, y * 2.2 + 2.6 * q2, s, 5);
  const env = 1 - smoothstep(0.1, 0.9, Math.hypot(x, y) + 0.5 * (fbm2(x * 1.3, y * 1.3, s ^ 0x17, 3) - 0.5));
  const body = 0.22 + 0.95 * smoothstep(0.32, 0.7, f);
  return { glow: body * env * (0.7 + 0.5 * fbm2(x * 5, y * 5, s ^ 0x29, 3)), q: q1 + q2 };
}

/** Emission (Carina's family): a bright red-pink complex torn by long dark lanes with lit edges and a hot core. */
const emission: Shader = (x, y, n, s) => {
  const { glow, q } = emissionGlow(x, y, s);
  const l1 = fbm2(x * 1.1 + 1.6 * q, y * 1.1 - 1.6 * q, s ^ 0x77, 3);
  const l2 = fbm2(x * 2.6 - 2 * q, y * 2.6 + 2 * q, s ^ 0x78, 3);
  // Broad dark bays where the low-frequency noise is high, and a few thin lanes.
  const bay = smoothstep(0.55, 0.63, l1);
  const d1 = Math.abs(l2 - 0.5);
  const lane = (1 - 0.94 * bay) * (1 - 0.85 * Math.exp(-((d1 / 0.04) ** 2)) * smoothstep(0.4, 0.55, fbm2(x * 1.2, y * 1.2, s ^ 0x79, 2)));
  // The bays' edges are lit by the stars beyond.
  const edge = 1 + 1.1 * Math.exp(-(((l1 - 0.54) / 0.035) ** 2));
  const hotspot = 0.9 * Math.exp(-(((x - 0.1 * (n.tint - 0.5)) ** 2 + y * y) / 0.05));
  // Fine structure: grain and thin bright filaments along the ridges of a second noise.
  const fine = 0.75 + 0.5 * fbm2(x * 18 + 5, y * 18, s ^ 0x7a, 3) + 0.7 * smoothstep(0.78, 0.95, ridged(x * 9 + 1, y * 9 - 2, s ^ 0x7b, 3));
  const i = (glow * edge + hotspot * glow) * lane * fine;
  const teal = smoothstep(0.45, 0.7, fbm2(x * 1.4 + 11, y * 1.4, s ^ 0x3b, 3)) * n.tint * 0.7;
  const hot = smoothstep(1.1, 2.2, i);
  return { i, colour: mix(mix(H_ALPHA, O_III, teal), WHITE, hot * 0.5) };
};

/** Direction toward the light (the hot stars beyond the top of the texture), normalised. */
const LIGHT: [number, number] = [0.28, 0.96];

/**
 * Pillars (the Eagle's family): tapering, bent columns with narrow finger tips
 * and small globules, a sharp bright rim where the light hits them (the
 * photo-evaporation front: white-hot on the edge, red and then O III
 * teal outside it), a dim textured inside, over a bright, filamentary
 * emission background that is brightest toward the light.
 */
const pillars: Shader = (x, y, n, s) => {
  const { glow } = emissionGlow(x * 0.9, y * 0.9, s);
  const columns = pillarColumns(n);
  const d = pillarDistance(columns, x, y);
  const mask = pillarMask(columns, x, y);
  const lit = 0.45 + 0.55 * smoothstep(-0.9, 0.7, y);
  const env = 1 - smoothstep(0.2, 0.9, Math.hypot(x, y) + 0.3 * (fbm2(x * 2, y * 2, s ^ 0x18, 3) - 0.5));
  // Background: fractal glow with filaments and a fine grain on top.
  const wisps = ridged(x * 7 + 3, y * 7, s ^ 0x41);
  const grain = fbm2(x * 24, y * 24, s ^ 0x42, 3);
  const structure = 0.55 + 0.75 * smoothstep(0.45, 0.9, wisps) + 0.5 * (grain - 0.5);
  let background = (0.12 + 0.8 * glow) * structure * lit * env;
  let rim = 0;
  let outer = 0;
  if (d < 0.2) {
    // Outline normal by finite differences (the shading uses only the direction).
    const h = 0.006;
    const gx = pillarDistance(columns, x + h, y) - pillarDistance(columns, x - h, y);
    const gy = pillarDistance(columns, x, y + h) - pillarDistance(columns, x, y - h);
    const gl = Math.hypot(gx, gy) || 1;
    const facing = Math.min(1, Math.max(0, (gx * LIGHT[0] + gy * LIGHT[1]) / gl));
    const light = 0.3 + 0.7 * facing;
    // A thin white-hot line just inside the edge, a red-orange front around it, a fainter teal glow beyond.
    rim = light * (Math.exp(-(((d + 0.003) / 0.011) ** 2)) + 0.45 * Math.exp(-(((d - 0.012) / 0.03) ** 2)));
    outer = light * Math.exp(-(Math.max(0, d - 0.02) / 0.06)) * (d > 0 ? 1 : 0);
    // Compressed gas piles up around the columns on the lit side.
    background += 0.5 * outer * lit * env;
  }
  // Inside the columns: a dim, wispy glow of lighter gas on dark cores, fading to black toward the foot.
  const body = mask * (0.5 + 0.5 * smoothstep(-0.2, 0.5, y)) * (0.04 + 0.22 * smoothstep(0.55, 0.95, ridged(x * 17 + 2, y * 17, s ^ 0x43, 3)) * (0.3 + 0.7 * grain));
  const i = (1 - mask) * background + body * env + 1.5 * rim * env;
  const teal = Math.min(1, 0.15 + 0.45 * smoothstep(-0.2, 0.9, y) + 0.7 * outer) * (0.4 + 0.6 * n.tint);
  const hot = Math.min(1, rim * 0.9);
  return { i, colour: mix(mix(H_ALPHA, O_III, teal), WHITE, hot) };
};

/** Remnant: a thin, broken, filamentary shell with bright arcs, red outside and blue-green inside. */
const remnant: Shader = (x, y, n, s) => {
  const warp = fbm2(x * 2 + 4, y * 2, s, 3) - 0.5;
  const d = Math.hypot(x, y) + 0.4 * warp;
  const ridge = 1 - Math.abs(2 * fbm2(x * 7 + 9, y * 7 - 3, s ^ 0x21, 4) - 1);
  const filament = smoothstep(0.62, 0.97, ridge);
  const gaps = smoothstep(0.3, 0.55, fbm2(x * 1.8 + 9, y * 1.8, s ^ 0x4d, 3));
  const shell = Math.exp(-(((d - 0.68) / 0.035) ** 2));
  const inner = 0.6 * Math.exp(-(((d - 0.5) / 0.035) ** 2)) * smoothstep(0.5, 0.75, fbm2(x * 3 + 1, y * 3 + 4, s ^ 0x4e, 3));
  const thin = smoothstep(0.78, 0.96, 1 - Math.abs(2 * fbm2(x * 15 + 4, y * 15 + 9, s ^ 0x22, 3) - 1));
  const grain = 0.7 + 0.6 * fbm2(x * 26, y * 26, s ^ 0x23, 3);
  const i = (shell * (0.25 + 1.3 * filament + 0.6 * thin) * (0.35 + 0.65 * gaps) + inner * filament * 1.4) * grain + 0.1 * Math.exp(-(((d - 0.64) / 0.2) ** 2)) * ridge * (0.5 + thin);
  const t = smoothstep(0.35, 0.65, (0.68 - d) * 2.2 + 0.5 + (n.tint - 0.5) * 0.4 + 0.5 * (fbm2(x * 2.5 + 2, y * 2.5, s ^ 0x6f, 3) - 0.5));
  return { i, colour: mix(H_ALPHA, O_III, t) };
};

/** The stars lighting a reflection nebula: (x, y, brightness), the first and brightest at its heart. */
const starsOf = new WeakMap<Nebula, [number, number, number][]>();
function reflectionStars(n: Nebula, s: number): [number, number, number][] {
  const known = starsOf.get(n);
  if (known) return known;
  // One to four (the Pleiades' brightest few), by the tint.
  const count = 1 + Math.floor(n.tint * 3.99);
  const stars: [number, number, number][] = [[0.04 * (lattice(1, 0, s) - 0.5), 0.04 * (lattice(2, 0, s) - 0.5), 1]];
  for (let k = 1; k < count; k++) {
    const a = 2 * Math.PI * lattice(k, 1, s);
    const r = 0.15 + 0.2 * lattice(k, 2, s);
    stars.push([r * Math.cos(a), r * Math.sin(a), 0.35 + 0.35 * lattice(k, 3, s)]);
  }
  starsOf.set(n, stars);
  return stars;
}

/**
 * Reflection (the Pleiades' family): a sheet of dust lit by a few bright
 * blue-white stars. The dust lies in long, slightly bent parallel streaks
 * with dark gaps between them (the striations round Merope); it shows only
 * where a star lights it, brightest close to each star, so the streaks
 * fade out with the light. The stars are sharp, white, with diffraction
 * spikes; the haze is blue.
 */
const reflection: Shader = (x, y, n, s) => {
  // The streaks' direction, one per nebula.
  const angle = Math.PI * lattice(0, 9, s);
  const ca = Math.cos(angle);
  const sa = Math.sin(angle);
  const u = x * ca + y * sa;
  // Across the streaks, bent by a slow noise: they curve a little and are not ruled lines.
  const v = -x * sa + y * ca + 0.3 * (fbm2(x * 1.4 + 3, y * 1.4, s ^ 0x91, 2) - 0.5);
  // Stretched noise (slow along, fast across): long streaks with dark gaps, and thin bright filaments in them.
  const streak = smoothstep(0.4, 0.7, fbm2(u * 1.3 + 7, v * 10, s ^ 0x92, 3));
  const filament = smoothstep(0.75, 0.97, ridged(u * 1.1 + 2, v * 18, s ^ 0x93, 3));
  const clump = 0.6 + 0.8 * fbm2(x * 3.5 + 1, y * 3.5, s ^ 0x94, 2);
  const dust = (0.06 + 0.9 * streak + 0.7 * filament * (0.3 + streak)) * clump;
  let light = 0;
  let star = 0;
  for (const [sx, sy, b] of reflectionStars(n, s)) {
    const d = Math.hypot(x - sx, y - sy);
    // The light on the dust falls off with the distance from each star.
    light += b * (Math.exp(-d / 0.16) + 0.08 * Math.exp(-d / 0.45));
    const core = b * Math.exp(-((d / 0.022) ** 2));
    const dx = Math.abs(x - sx);
    const dy = Math.abs(y - sy);
    const spikes = b * b * 0.6 * (Math.exp(-((dx / 0.006) ** 2)) * Math.exp(-dy * 7) + Math.exp(-((dy / 0.006) ** 2)) * Math.exp(-dx * 7));
    star += 2.2 * core + spikes;
  }
  // An irregular outline: the dust sheet is patchy, never a disc.
  const sheet = smoothstep(0.3, 0.65, fbm2(x * 1.8 + 5, y * 1.8 - 2, s ^ 0x95, 3) + 0.35 * (1 - Math.hypot(x, y)));
  const haze = 0.55 * dust * light * sheet;
  const blue = mix(BLUE, [0.55, 0.72, 1], n.tint);
  // White at the stars, blue in the dust (a little paler close to the stars).
  const white = Math.min(1, star / Math.max(1e-6, star + haze));
  return { i: haze + star, colour: mix(mix(blue, [0.7, 0.82, 1], Math.min(1, 0.3 * light)), WHITE, white) };
};

const SHADERS: Record<Exclude<NebulaKind, 'dark'>, Shader> = { planetary, emission, pillars, remnant, reflection };

/** Where a texture fades to nothing: no texel beyond 0.9 of the half size has any light. */
const fade = (rr: number) => 1 - smoothstep(0.74, 0.9, rr);

/**
 * The planet view's field of view and the canvas height we size for: a
 * 900 px tall view at a pixel ratio of 1, so that a texel is about a
 * screen pixel there (two device pixels on a pixel ratio of 2: the
 * nebulae are soft enough). Sizing for 1800 px and rounding up to powers
 * of two baked 10 million texels for a sky near the core (about 6 s).
 */
const VIEW_FOV_DEG = 40;
const VIEW_HEIGHT = 900;
/** Sizes are multiples of this (WebGL 2 needs no power of two), between the two bounds. */
const SIZE_STEP = 32;
const MIN_TEXTURE = 64;
const MAX_TEXTURE = 768;

/** Texture size by the nebula's angular radius: texels about as wide as a screen pixel at the planet view (see VIEW_HEIGHT). */
export function nebulaTextureSize(radius: number): number {
  const pixels = ((2 * radius * 180) / Math.PI) * (VIEW_HEIGHT / VIEW_FOV_DEG);
  return Math.min(MAX_TEXTURE, Math.max(MIN_TEXTURE, Math.ceil(pixels / SIZE_STEP) * SIZE_STEP));
}

const encode = (linear: number): number => {
  const v = linear <= 0.0031308 ? 12.92 * linear : 1.055 * linear ** (1 / 2.4) - 0.055;
  return Math.round(255 * Math.min(1, Math.max(0, v)));
};

function bake(n: Nebula, exposure: SkyExposure): NebulaTexture {
  const size = nebulaTextureSize(n.radius);
  const rgba = new Uint8Array(size * size * 4);
  const ex = EXPOSURES[exposure];
  const s = subSeed(n.seed, `texture:${n.kind}`);
  const dark = n.kind === 'dark';
  const gain = ex.level * (exposure === 'eye' ? EYE_KIND_GAIN[n.kind] : 1);
  const shader = dark ? null : SHADERS[n.kind as Exclude<NebulaKind, 'dark'>];
  for (let row = 0; row < size; row += 1) {
    const y = 1 - (2 * (row + 0.5)) / size;
    for (let col = 0; col < size; col += 1) {
      const x = (2 * (col + 0.5)) / size - 1;
      const at = (row * size + col) * 4;
      if (dark) {
        rgba[at + 3] = Math.round(255 * ex.dark * darkOpacity(n, x, y));
        continue;
      }
      const w = fade(Math.hypot(x, y));
      if (w <= 0) continue;
      const t = shader!(x, y, n, s);
      let [r, g, b] = t.colour.map((c) => c * t.i * w * gain) as Rgb;
      const lum = 0.2126 * r + 0.7152 * g + 0.0722 * b;
      r = Math.max(0, lum + (r - lum) * ex.saturation);
      g = Math.max(0, lum + (g - lum) * ex.saturation);
      b = Math.max(0, lum + (b - lum) * ex.saturation);
      const lr = ex.ceiling * Math.tanh(r / ex.ceiling);
      const lg = ex.ceiling * Math.tanh(g / ex.ceiling);
      const lb = ex.ceiling * Math.tanh(b / ex.ceiling);
      const br = encode(lr);
      const bg = encode(lg);
      const bb = encode(lb);
      rgba[at] = br;
      rgba[at + 1] = bg;
      rgba[at + 2] = bb;
      // Premultiplied: the colour never exceeds the alpha.
      rgba[at + 3] = Math.max(br, bg, bb, Math.round((255 * Math.max(lr, lg, lb)) / ex.ceiling));
    }
  }
  return { size, rgba, blend: dark ? 'darken' : 'add' };
}

const bakes = new Map<string, NebulaTexture>();

/** The nebula's texture in an exposure; memoised (the last 96 are kept). */
export function bakeNebula(n: Nebula, exposure: SkyExposure): NebulaTexture {
  const key = [exposure, n.kind, n.seed, n.radius, n.tint].join('|');
  const hit = bakes.get(key);
  if (hit) {
    bakes.delete(key);
    bakes.set(key, hit);
    return hit;
  }
  const texture = bake(n, exposure);
  bakes.set(key, texture);
  if (bakes.size > 96) bakes.delete(bakes.keys().next().value!);
  return texture;
}
