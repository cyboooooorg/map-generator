// What a giant's rings and its ring moons are, for a body's card (backlog
// item 18; crates/mapgen/src/rings.rs): the rings the system drew for the
// giant, and the part a ring moon plays in them.
import type { Body } from './api/types/Body';
import type { DustRing } from './api/types/DustRing';
import type { RingRole } from './api/types/RingRole';
import type { RingSystem } from './api/types/RingSystem';

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
