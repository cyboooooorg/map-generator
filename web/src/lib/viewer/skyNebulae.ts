// The nebulae of a system's sky: a seeded catalogue of objects, in the
// families of real nebulae (never copies): planetary (the Helix's family),
// emission complexes with dark lanes (Carina's), pillars (the Eagle's),
// supernova remnants, reflection nebulae and dark nebulae.
//
// All directions are in the system frame. Each kind draws from its own
// sub-stream of the sky seed, so adding a kind later reshuffles no other.
// Dark nebulae and pillar silhouettes also dim what is behind them: see
// `nebulaDustAt`, which `dustAt` (skyClouds) adds to the band's dust.

import type { Vec3 } from '../skyBodies';
import { fbm2, smoothstep } from './nebulaNoise';
import { bandFrame, type SkyLook } from './skyLook';
import { random, subSeed } from './skyRandom';

export type NebulaKind = 'planetary' | 'emission' | 'pillars' | 'remnant' | 'reflection' | 'dark';

export interface Nebula {
  kind: NebulaKind;
  /** Centre, unit, system frame. */
  dir: Vec3;
  /** Angular radius, radians: the texture's quad spans plus and minus this around `dir`. */
  radius: number;
  /** Orientation about `dir`, radians (see `nebulaBasis`). */
  roll: number;
  /** Its own shape seed. */
  seed: number;
  /** Colour-mix parameter within the kind, 0 to 1. */
  tint: number;
}

interface KindSpec {
  kind: NebulaKind;
  /** Count at the Sun's place (inclusive range), scaled with the look's brightness. */
  count: [number, number];
  /** Angular radius range, degrees (log-uniform). */
  size: [number, number];
  /** Share drawn about the band (the rest is uniform on the sphere). */
  bandShare: number;
  /** Latitude spread of the band-drawn ones, in band widths. */
  spread: number;
  /** Hard limit of the latitude, degrees. */
  maxLatitude: number;
}

const KINDS: KindSpec[] = [
  { kind: 'planetary', count: [3, 8], size: [0.5, 3], bandShare: 0.35, spread: 2, maxLatitude: 90 },
  { kind: 'emission', count: [2, 5], size: [3, 12], bandShare: 1, spread: 0.8, maxLatitude: 15 },
  { kind: 'pillars', count: [1, 3], size: [1.5, 5], bandShare: 1, spread: 0.6, maxLatitude: 15 },
  { kind: 'remnant', count: [1, 3], size: [2, 8], bandShare: 0.5, spread: 1.5, maxLatitude: 90 },
  { kind: 'reflection', count: [2, 6], size: [0.5, 3], bandShare: 1, spread: 1, maxLatitude: 15 },
  { kind: 'dark', count: [4, 10], size: [1.5, 7], bandShare: 1, spread: 0.4, maxLatitude: 8 },
];

/** The size table, degrees, for the tests and the texture resolution. */
export const NEBULA_SIZE_DEG: Record<NebulaKind, [number, number]> = Object.fromEntries(
  KINDS.map((k) => [k.kind, k.size]),
) as Record<NebulaKind, [number, number]>;

const RAD = Math.PI / 180;

const dot = (a: Vec3, b: Vec3) => a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
const cross = (a: Vec3, b: Vec3): Vec3 => [
  a[1] * b[2] - a[2] * b[1],
  a[2] * b[0] - a[0] * b[2],
  a[0] * b[1] - a[1] * b[0],
];
const unit = (v: Vec3): Vec3 => {
  const l = Math.hypot(v[0], v[1], v[2]);
  return l > 0 ? [v[0] / l, v[1] / l, v[2] / l] : [1, 0, 0];
};

function gauss(next: () => number): number {
  return Math.sqrt(-2 * Math.log(Math.max(1e-9, next()))) * Math.cos(2 * Math.PI * next());
}

/** The roll that turns the texture's x axis along the band's great circle at the nebula's centre. */
function alongBandRoll(n: Nebula, north: Vec3): number {
  const { right, up } = nebulaBasis({ ...n, roll: 0 });
  const tangent = unit(cross(north, n.dir));
  return Math.atan2(dot(tangent, up), dot(tangent, right));
}

function draw(look: SkyLook, spec: KindSpec): Nebula[] {
  const next = random(subSeed(look.seed, `nebula:${spec.kind}`));
  const scale = Math.min(1.8, Math.max(0.5, Math.sqrt(look.brightness)));
  const [lo, hi] = spec.count;
  const base = lo + Math.floor(next() * (hi - lo + 1));
  const count = Math.max(1, Math.round(base * scale));
  const frame = bandFrame(look);
  const centreBias = 0.25 + 0.5 * look.bulge;
  const out: Nebula[] = [];
  for (let i = 0; i < count; i += 1) {
    let dir: Vec3;
    if (next() < spec.bandShare) {
      const limit = spec.maxLatitude * RAD;
      const lat = Math.max(-limit, Math.min(limit, gauss(next) * look.bandWidth * spec.spread));
      const lon = next() < centreBias ? gauss(next) * 0.9 : (next() * 2 - 1) * Math.PI;
      const cl = Math.cos(lat);
      dir = [
        cl * (Math.cos(lon) * frame.along[0] + Math.sin(lon) * frame.across[0]) + Math.sin(lat) * frame.north[0],
        cl * (Math.cos(lon) * frame.along[1] + Math.sin(lon) * frame.across[1]) + Math.sin(lat) * frame.north[1],
        cl * (Math.cos(lon) * frame.along[2] + Math.sin(lon) * frame.across[2]) + Math.sin(lat) * frame.north[2],
      ];
    } else {
      const z = next() * 2 - 1;
      const a = next() * 2 * Math.PI;
      const r = Math.sqrt(1 - z * z);
      dir = [r * Math.cos(a), r * Math.sin(a), z];
    }
    const [smin, smax] = spec.size;
    const radius = smin * (smax / smin) ** next() * RAD;
    const nebula: Nebula = {
      kind: spec.kind,
      dir: unit(dir),
      radius,
      roll: next() * 2 * Math.PI,
      seed: Math.floor(next() * 4_294_967_296) >>> 0,
      tint: next(),
    };
    // Dark nebulae are elongated and lie along the band: their long axis follows the plane.
    if (spec.kind === 'dark') nebula.roll = alongBandRoll(nebula, frame.north) + (next() - 0.5) * 0.5;
    out.push(nebula);
  }
  return out;
}

const byObject = new WeakMap<SkyLook, Nebula[]>();
const byKey = new Map<string, Nebula[]>();

/** The look's nebulae; memoised per look. */
export function skyNebulae(look: SkyLook): Nebula[] {
  const hit = byObject.get(look);
  if (hit) return hit;
  const key = [look.seed, look.brightness, look.bandWidth, look.bulge, ...look.north, ...look.centre].join('|');
  let list = byKey.get(key);
  if (!list) {
    list = KINDS.flatMap((spec) => draw(look, spec));
    if (byKey.size >= 16) byKey.clear();
    byKey.set(key, list);
  }
  byObject.set(look, list);
  return list;
}

/** The right and up unit vectors of a nebula's texture on the sky (tangent to the sphere at `dir`, turned by `roll`). */
export function nebulaBasis(n: Nebula): { right: Vec3; up: Vec3 } {
  const ref: Vec3 = Math.abs(n.dir[1]) < 0.9 ? [0, 1, 0] : [1, 0, 0];
  const right0 = unit(cross(ref, n.dir));
  const up0 = cross(n.dir, right0);
  const c = Math.cos(n.roll);
  const s = Math.sin(n.roll);
  return {
    right: [c * right0[0] + s * up0[0], c * right0[1] + s * up0[1], c * right0[2] + s * up0[2]],
    up: [-s * right0[0] + c * up0[0], -s * right0[1] + c * up0[1], -s * right0[2] + c * up0[2]],
  };
}

/** A dark column of a pillars nebula, in the texture's frame (x right, y up, both in [-1, 1]). */
export interface Pillar {
  x: number;
  /** Half width at the base. */
  width: number;
  /** Height of the tip. */
  top: number;
  lean: number;
  phase: number;
}

const pillarCache = new WeakMap<Nebula, Pillar[]>();

/** One to three dark columns, rising from the bottom edge and tapering, drawn from the nebula's seed. */
export function pillarColumns(n: Nebula): Pillar[] {
  const hit = pillarCache.get(n);
  if (hit) return hit;
  const next = random(subSeed(n.seed, 'pillars'));
  const count = 2 + Math.floor(next() * 2);
  const columns: Pillar[] = [];
  for (let i = 0; i < count; i += 1) {
    columns.push({
      x: (count === 1 ? 0 : -0.5 + (i / (count - 1)) * 1) * 0.9 + (next() - 0.5) * 0.15,
      width: 0.17 + next() * 0.1,
      top: -0.2 + next() * 0.6,
      lean: (next() - 0.5) * 0.3,
      phase: next() * 6.28,
    });
  }
  pillarCache.set(n, columns);
  return columns;
}

/** Pillar parameters at height `y`: centre and half width, or null above the tip. */
export function pillarAt(p: Pillar, y: number): { cx: number; half: number } | null {
  if (y > p.top) return null;
  const t = Math.max(0, (y + 1) / (p.top + 1));
  // A knobbly shaft, wide at the foot, with a rounded dome at the tip.
  const knobs = 1 + 0.12 * Math.sin(7 * y + p.phase);
  const dome = Math.sqrt(1 - (1 - Math.min(1, (p.top - y) / (0.9 * p.width))) ** 2);
  return {
    cx: p.x + p.lean * t + 0.02 * Math.sin(9 * y + p.phase),
    half: p.width * (1 - 0.45 * t ** 1.3) * knobs * dome,
  };
}

/** Opacity 0..1 of the columns at a point of the texture's frame, soft-edged. */
export function pillarMask(columns: Pillar[], x: number, y: number): number {
  let mask = 0;
  for (const p of columns) {
    const at = pillarAt(p, y);
    if (!at) continue;
    if (at.half < 1e-4) continue;
    mask = Math.max(mask, 1 - smoothstep(at.half * 0.8, at.half, Math.abs(x - at.cx)));
  }
  return mask;
}

/**
 * Opacity 0..1 of a dark nebula at a point of its texture's frame (x right,
 * y up, both in [-1, 1]): an elongated, bent patch (long along x) with
 * warped, ragged edges and a few dark tendrils, zero beyond 0.9 of the half
 * size. The texture and `nebulaDustAt` both read it.
 */
export function darkOpacity(n: Nebula, x: number, y: number): number {
  const s = subSeed(n.seed, 'texture:dark');
  const rr = Math.hypot(x, y);
  if (rr >= 0.9) return 0;
  const aspect = 0.3 + 0.25 * fbm2(n.tint * 9, 1.5, s, 1);
  const bend = 0.22 * Math.sin(2.1 * x + n.tint * 6);
  const u = x + 0.3 * (fbm2(x * 2 + 5, y * 2, s ^ 0x31, 3) - 0.5);
  const v = y - bend + 0.22 * (fbm2(x * 2, y * 2 + 8, s ^ 0x32, 3) - 0.5);
  const body = 1 - smoothstep(0.3, 1, Math.hypot(u / 0.95, v / (0.95 * aspect)));
  const ragged = fbm2(x * 5 + 2, y * 5 - 1, s, 4) - 0.5;
  const core = smoothstep(0.18, 0.5, body + 0.55 * ragged);
  const ridge = 1 - Math.abs(2 * fbm2(x * 3.2 + 4, y * 3.2 + 1, s ^ 0x33, 3) - 1);
  const tendril = 0.7 * smoothstep(0.78, 0.95, ridge) * (1 - smoothstep(0.1, 0.9, Math.hypot(u / 1.4, v / (1.4 * aspect * 1.6))));
  const mottle = 0.8 + 0.3 * fbm2(x * 4 + 6, y * 4, s ^ 0x5, 3);
  return Math.min(1, Math.max(core * mottle, tendril)) * (1 - smoothstep(0.74, 0.9, rr));
}

/** Strongest extinction of a dark nebula or pillar silhouette at its centre. */
const DARK_OPACITY = 0.85;
const PILLAR_OPACITY = 0.75;

interface Occluder {
  nebula: Nebula;
  cosRadius: number;
  tanRadius: number;
  right: Vec3;
  up: Vec3;
}

const occluderCache = new WeakMap<Nebula[], Occluder[]>();

function occludersOf(look: SkyLook): Occluder[] {
  const list = skyNebulae(look);
  let found = occluderCache.get(list);
  if (!found) {
    found = list
      .filter((n) => n.kind === 'dark' || n.kind === 'pillars')
      .map((nebula) => ({
        nebula,
        cosRadius: Math.cos(nebula.radius),
        tanRadius: Math.tan(nebula.radius),
        ...nebulaBasis(nebula),
      }));
    occluderCache.set(list, found);
  }
  return found;
}

/**
 * Extinction 0..1 of the dark nebulae and pillar silhouettes toward a unit
 * direction; the textures draw the same shapes.
 */
export function nebulaDustAt(look: SkyLook, dir: Vec3): number {
  let clear = 1;
  for (const o of occludersOf(look)) {
    const c = dot(dir, o.nebula.dir);
    if (c < o.cosRadius) continue;
    const x = dot(dir, o.right) / c / o.tanRadius;
    const y = dot(dir, o.up) / c / o.tanRadius;
    if (o.nebula.kind === 'dark') clear *= 1 - DARK_OPACITY * darkOpacity(o.nebula, x, y);
    else clear *= 1 - PILLAR_OPACITY * pillarMask(pillarColumns(o.nebula), x, y);
  }
  return 1 - clear;
}
