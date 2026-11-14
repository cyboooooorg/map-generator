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
import { pillarAt, pillarColumns, pillarMask, type Nebula, type NebulaKind } from './skyNebulae';
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
const PINK: Rgb = [1, 0.32, 0.42];
const BLUE: Rgb = [0.3, 0.5, 1];
const WHITE: Rgb = [1, 0.95, 0.9];

/** Per-exposure peak linear level of a unit-intensity texel, soft ceiling and colour saturation. */
const EXPOSURES: Record<SkyExposure, { level: number; ceiling: number; saturation: number; dark: number }> = {
  eye: { level: 0.06, ceiling: CLOUD_CEILING, saturation: 0.45, dark: 0.6 },
  photo: { level: 0.42, ceiling: 0.3, saturation: 1, dark: 1 },
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

const smoothstep = (a: number, b: number, x: number) => {
  const t = Math.min(1, Math.max(0, (x - a) / (b - a)));
  return t * t * (3 - 2 * t);
};
const mix = (a: Rgb, b: Rgb, t: number): Rgb => [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t];

function lattice(ix: number, iy: number, seed: number): number {
  let h = Math.imul(ix, 0x27d4eb2d) ^ Math.imul(iy, 0x165667b1) ^ seed;
  h = Math.imul(h ^ (h >>> 15), 0x85ebca6b);
  h ^= h >>> 13;
  h = Math.imul(h, 0xc2b2ae35);
  h ^= h >>> 16;
  return (h >>> 0) / 4_294_967_296;
}

function value2(x: number, y: number, seed: number): number {
  const xi = Math.floor(x);
  const yi = Math.floor(y);
  const fx = x - xi;
  const fy = y - yi;
  const u = fx * fx * (3 - 2 * fx);
  const v = fy * fy * (3 - 2 * fy);
  const a = lattice(xi, yi, seed);
  const b = lattice(xi + 1, yi, seed);
  const c = lattice(xi, yi + 1, seed);
  const d = lattice(xi + 1, yi + 1, seed);
  return a + u * (b - a) + v * (c - a) + u * v * (a - b - c + d);
}

/** Fractal noise in [0, 1). */
function fbm2(x: number, y: number, seed: number, octaves = 4): number {
  let sum = 0;
  let amp = 0.5;
  let norm = 0;
  let fx = x;
  let fy = y;
  for (let i = 0; i < octaves; i += 1) {
    sum += amp * value2(fx, fy, seed + i * 977);
    norm += amp;
    amp *= 0.5;
    fx = fx * 2.03 + 17.1;
    fy = fy * 2.03 + 9.7;
  }
  return sum / norm;
}

/** A texel's glow: an intensity (about 0 to 1) and its colour; `opacity` for dark nebulae. */
interface Texel {
  i: number;
  colour: Rgb;
}

type Shader = (x: number, y: number, n: Nebula, s: number) => Texel;

/** Planetary: a ring (or two lobes) with an O III core and an H-alpha rim, and a central star. */
const planetary: Shader = (x, y, n, s) => {
  const knots = 0.55 + 0.9 * fbm2(x * 5 + 3, y * 5 + 1, s, 3);
  const star = 0.9 * Math.exp(-((x * x + y * y) / (0.03 * 0.03)));
  if (n.tint > 0.6) {
    // Bipolar: two shells joined at a pinched waist, along the vertical.
    let i = 0;
    let rim = 0;
    for (const side of [-1, 1]) {
      const d = Math.hypot(x / (0.3 - 0.04 * n.tint), (y - side * 0.36) / 0.38);
      const shell = Math.exp(-(((d - 0.9) / 0.17) ** 2)) * knots;
      i += shell + 0.28 * Math.exp(-d * d * 1.6);
      rim = Math.max(rim, smoothstep(0.7, 1.15, d));
    }
    return { i: i + star, colour: mix(star > 0.1 ? WHITE : O_III, H_ALPHA, rim) };
  }
  const rho = 0.5;
  const thick = 0.095 + 0.04 * lattice(n.seed & 255, 3, s);
  const rr = Math.hypot(x, y * 1.1);
  const ring = Math.exp(-(((rr - rho) / thick) ** 2)) * knots;
  const inner = 0.2 * (1 - smoothstep(0.1, rho, rr)) * smoothstep(0.02, 0.1, rr);
  const halo = 0.13 * Math.exp(-(((rr - rho - 0.17) / 0.12) ** 2)) * (0.5 + fbm2(x * 3, y * 3, s + 5, 3));
  const rimMix = smoothstep(rho - 0.8 * thick, rho + 0.9 * thick, rr);
  return { i: ring + inner + halo + star, colour: star > 0.15 ? WHITE : mix(O_III, H_ALPHA, rimMix) };
};

/** Glow of an emission complex at a texel: warped fractal noise under an irregular envelope. */
function emissionGlow(x: number, y: number, s: number): { glow: number; q: number } {
  const q1 = fbm2(x * 1.6 + 3.1, y * 1.6 + 1.7, s ^ 0x51, 3) - 0.5;
  const q2 = fbm2(x * 1.6 + 7.7, y * 1.6 - 4.2, s ^ 0xa3, 3) - 0.5;
  const f = fbm2(x * 2.4 + 2.6 * q1, y * 2.4 + 2.6 * q2, s, 5);
  const env = 1 - smoothstep(0.3, 0.9, Math.hypot(x, y) + 0.5 * (fbm2(x * 1.3, y * 1.3, s ^ 0x17, 3) - 0.5));
  return { glow: smoothstep(0.34, 0.74, f) ** 1.4 * env * (0.55 + 0.7 * fbm2(x * 5, y * 5, s ^ 0x29, 3)), q: q1 + q2 };
}

/** Emission: a red-pink glow torn by dark lanes, with brighter ionised knots. */
const emission: Shader = (x, y, n, s) => {
  const { glow, q } = emissionGlow(x, y, s);
  const l = fbm2(x * 2.2 + 4 * q, y * 2.2 - 4 * q, s ^ 0x77, 4);
  const lane = 1 - 0.95 * Math.exp(-(((l - 0.5) / 0.075) ** 2));
  const i = glow * lane;
  const teal = smoothstep(0.45, 0.7, fbm2(x * 1.4 + 11, y * 1.4, s ^ 0x3b, 3)) * n.tint * 0.7;
  const hot = smoothstep(0.55, 1, i);
  return { i, colour: mix(mix(PINK, O_III, teal), WHITE, hot * 0.45) };
};

/** Pillars: emission background with dark tapering columns, their rims lit. */
const pillars: Shader = (x, y, n, s) => {
  const { glow } = emissionGlow(x * 0.9, y * 0.9, s);
  const columns = pillarColumns(n);
  const mask = pillarMask(columns, x, y);
  const lit = 0.45 + 0.55 * smoothstep(-0.9, 0.7, y);
  let rim = 0;
  for (const p of columns) {
    const at = pillarAt(p, Math.min(y, p.top));
    if (!at) continue;
    const dy = Math.max(0, y - p.top);
    const d = Math.hypot(Math.max(0, Math.abs(x - at.cx) - at.half * 0.85), dy);
    rim = Math.max(rim, Math.exp(-((d / 0.035) ** 2)));
  }
  const env = 1 - smoothstep(0.55, 0.9, Math.hypot(x, y));
  const background = (0.12 + 1.1 * glow) * lit * env;
  const i = (1 - mask) * (background + 0.75 * rim * (0.6 + 0.4 * lit) * env);
  return { i, colour: mix(mix(PINK, O_III, 0.25 * n.tint), WHITE, Math.min(1, rim * 0.6)) };
};

/** Remnant: a thin, broken, filamentary shell, red and blue-green. */
const remnant: Shader = (x, y, n, s) => {
  const d = Math.hypot(x, y) + 0.45 * (fbm2(x * 2 + 4, y * 2, s, 3) - 0.5);
  const shell = Math.exp(-(((d - 0.7) / 0.055) ** 2));
  const ridge = 1 - Math.abs(2 * fbm2(x * 6 + 9, y * 6 - 3, s ^ 0x21, 4) - 1);
  const filament = smoothstep(0.55, 0.95, ridge);
  const gaps = smoothstep(0.25, 0.55, fbm2(x * 1.8 + 9, y * 1.8, s ^ 0x4d, 3));
  const i = shell * (0.2 + 0.9 * filament) * gaps + 0.07 * Math.exp(-(((d - 0.7) / 0.2) ** 2)) * ridge;
  const t = smoothstep(0.4, 0.6, fbm2(x * 2.5 + 2, y * 2.5, s ^ 0x6f, 3) + (n.tint - 0.5) * 0.4);
  return { i, colour: mix(H_ALPHA, O_III, t) };
};

/** Reflection: a blue haze round a bright star. */
const reflection: Shader = (x, y, n, s) => {
  const rr = Math.hypot(x, y);
  const haze = Math.exp(-rr * rr * 3.4) * (0.55 + 0.9 * fbm2(x * 3, y * 3, s, 4));
  const star = 0.9 * Math.exp(-((rr / 0.05) ** 2));
  const colour = mix(BLUE, [0.55, 0.72, 1], n.tint);
  return { i: haze * 0.85 + star, colour: mix(colour, WHITE, Math.min(1, star + Math.exp(-rr * rr * 30) * 0.5)) };
};

const SHADERS: Record<Exclude<NebulaKind, 'dark'>, Shader> = { planetary, emission, pillars, remnant, reflection };

/** Opacity 0..1 of a dark nebula at a texel: a soft-edged patch with a mottled density. */
function darkOpacity(x: number, y: number, s: number): number {
  const rr = Math.hypot(x, y) + 0.1 * (fbm2(x * 3, y * 3, s, 3) - 0.5);
  return Math.min(1, (1 - smoothstep(0.4, 0.9, rr)) * (0.9 + 0.3 * fbm2(x * 4 + 6, y * 4, s ^ 0x5, 3)));
}

/** Where a texture fades to nothing: no texel beyond 0.9 of the half size has any light. */
const fade = (rr: number) => 1 - smoothstep(0.74, 0.9, rr);

/** Texture size by the nebula's angular radius. */
export function nebulaTextureSize(radius: number): number {
  const deg = (radius * 180) / Math.PI;
  return deg < 1 ? 128 : deg < 4 ? 256 : 512;
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
      const w = fade(Math.hypot(x, y));
      if (w <= 0) continue;
      const at = (row * size + col) * 4;
      if (dark) {
        rgba[at + 3] = Math.round(255 * ex.dark * w * darkOpacity(x, y, s));
        continue;
      }
      const t = shader!(x, y, n, s);
      let [r, g, b] = t.colour.map((c) => c * t.i * w * gain) as Rgb;
      const lum = 0.2126 * r + 0.7152 * g + 0.0722 * b;
      r = lum + (r - lum) * ex.saturation;
      g = lum + (g - lum) * ex.saturation;
      b = lum + (b - lum) * ex.saturation;
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
