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
