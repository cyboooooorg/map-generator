// What a giant's rings and its ring moons are, for a body's card (backlog
// item 18; crates/mapgen/src/rings.rs): the rings the system drew for the
// giant, and the part a ring moon plays in them.
import type { Body } from './api/types/Body';
import type { DustRing } from './api/types/DustRing';
import type { RingRole } from './api/types/RingRole';
import type { RingSystem } from './api/types/RingSystem';
import type { Vec3 } from './geo';
import { DUST_MARCH_STEPS, DUST_SCALE_HEIGHT } from './viewer/dustPhase';

/** The rings the system drew for `body`, if a giant with rings. */
export function ringsOf(body: Body): RingSystem | null {
  return body.params.given_rings?.rings ?? null;
}

/** The dust rings the system drew for `body`, if a giant whose moons feed some. */
export function dustOf(body: Body): DustRing[] {
  return body.params.given_rings?.dust ?? [];
}

/** What the giant's view draws of a faint ring: a dust ring's edges, peak, opacity and colour. */
export type FaintRing = Pick<DustRing, 'inner' | 'outer' | 'peak' | 'opacity' | 'colour'>;

/** Most dust rings a moon's sky draws (the sky ring shader's array size). */
export const SKY_DUST_MAX = 4;

/** The dust rings of a host as the sky ring shader takes them (see skyRingDensityGlsl). */
export interface SkyDustUniforms {
  /** Inner edge, peak and outer edge of each ring, in the host's radii: 3 per ring. */
  radii: Float32Array;
  /** Face-on opacity of each ring. */
  opacity: Float32Array;
  /** Linear colour of each ring: 3 per ring. */
  colour: Float32Array;
  /** How many rings are filled in. */
  count: number;
}

function srgbToLinear(c: number): number {
  return c <= 0.04045 ? c / 12.92 : Math.pow((c + 0.055) / 1.055, 2.4);
}

/**
 * Packs the `SKY_DUST_MAX` dust rings of highest `opacity x (outer - inner)`
 * (the ones that show most) for the sky ring shader, zero-padded; colours
 * go from sRGB (the data) to linear.
 */
export function skyDustUniforms(rings: readonly FaintRing[]): SkyDustUniforms {
  const kept = [...rings].sort((a, b) => b.opacity * (b.outer - b.inner) - a.opacity * (a.outer - a.inner)).slice(0, SKY_DUST_MAX);
  const radii = new Float32Array(3 * SKY_DUST_MAX);
  const opacity = new Float32Array(SKY_DUST_MAX);
  const colour = new Float32Array(3 * SKY_DUST_MAX);
  kept.forEach((ring, k) => {
    radii.set([ring.inner, ring.peak, ring.outer], 3 * k);
    opacity[k] = ring.opacity;
    colour.set(ring.colour.map(srgbToLinear), 3 * k);
  });
  return { radii, opacity, colour, count: kept.length };
}

/** Colour of a D-like ring: grey-brown dust (sRGB). */
const INNER_RING_COLOUR: [number, number, number] = [0.55, 0.5, 0.45];

/** The faint ring inside the bright rings of `body` (like Saturn's D ring), drawn as a smooth dust ring, or none. */
export function innerRingOf(body: Body): FaintRing[] {
  const d = ringsOf(body)?.d_ring;
  return d ? [{ inner: d.inner, outer: d.outer, peak: (d.inner + d.outer) / 2, opacity: d.opacity, colour: INNER_RING_COLOUR }] : [];
}

/** The dust ring `moon` feeds round `planet`, if any. */
export function dustFedBy(moon: Body, planet: Body | null): DustRing | null {
  return (planet ? dustOf(planet) : []).find((d) => d.source === moon.id) ?? null;
}

/** A ring moon's part in its planet's rings: its role, and the width of the gap it opens, km. */
export interface RingPart {
  role: RingRole;
  gapKm: number | null;
}

/** The part `moon` plays in the rings of `planet`, or null for a moon without one. */
export function ringPart(moon: Body, planet: Body | null): RingPart | null {
  if (!moon.ring_role) return null;
  const rings = planet ? ringsOf(planet) : null;
  const index = rings?.gap_moons?.indexOf(moon.id) ?? -1;
  const gap = index >= 0 ? rings?.gaps[index] : undefined;
  return {
    role: moon.ring_role,
    gapKm: gap && planet ? gap[1] * planet.physical.radius_km : null,
  };
}

/**
 * The optical depth of a dust ring along a view ray (host radii, y up the
 * ring's axis): the ring's density is its face-on `opacity` times its
 * triangular radial profile, spread over a Gaussian of the height with
 * sigma `DUST_SCALE_HEIGHT x r` and normalised so that a vertical crossing
 * gives the face-on value. Marched over the part of the ray inside the
 * slab of 3 sigma and the ring's outer radius, `DUST_MARCH_STEPS` midpoint
 * samples, as the sky ring shader does; up to `tMax` along `dir` (unit).
 * Finite with the eye in the ring plane, inside the ring or outside.
 */
export function dustColumn(eye: Vec3, dir: Vec3, ring: FaintRing, tMax = Infinity): number {
  const slab = 3 * DUST_SCALE_HEIGHT * ring.outer;
  let t0 = 0;
  let t1 = tMax;
  if (Math.abs(dir[1]) > 1e-6) {
    const a = (-slab - eye[1]) / dir[1];
    const b = (slab - eye[1]) / dir[1];
    t0 = Math.max(t0, Math.min(a, b));
    t1 = Math.min(t1, Math.max(a, b));
  } else if (Math.abs(eye[1]) >= slab) return 0;
  const a = dir[0] * dir[0] + dir[2] * dir[2];
  const b = eye[0] * dir[0] + eye[2] * dir[2];
  const c = eye[0] * eye[0] + eye[2] * eye[2] - ring.outer * ring.outer;
  if (a < 1e-12) {
    if (c > 0) return 0;
  } else {
    const disc = b * b - a * c;
    if (disc < 0) return 0;
    const root = Math.sqrt(disc);
    t0 = Math.max(t0, (-b - root) / a);
    t1 = Math.min(t1, (-b + root) / a);
  }
  if (!(t1 > t0)) return 0;
  const ds = (t1 - t0) / DUST_MARCH_STEPS;
  let column = 0;
  for (let k = 0; k < DUST_MARCH_STEPS; k += 1) {
    const t = t0 + (k + 0.5) * ds;
    const x = eye[0] + t * dir[0];
    const y = eye[1] + t * dir[1];
    const z = eye[2] + t * dir[2];
    const r = Math.hypot(x, z);
    if (r < ring.inner || r > ring.outer) continue;
    const side = r < ring.peak ? (ring.peak - r) / Math.max(ring.peak - ring.inner, 1e-4) : (r - ring.peak) / Math.max(ring.outer - ring.peak, 1e-4);
    const x01 = Math.min(Math.max(side, 0), 1);
    const profile = ring.opacity * (1 - x01 * x01 * (3 - 2 * x01));
    const h = DUST_SCALE_HEIGHT * r;
    column += (profile * Math.exp(-0.5 * (y / h) ** 2) / (h * Math.sqrt(2 * Math.PI))) * ds;
  }
  return column;
}
