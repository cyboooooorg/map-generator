import type { Body } from './api/types/Body';

/**
 * The classes drawn through the giant path: the three giants and the
 * mini-Neptune, whose radius is the giant look's (issue #446). The
 * presets a request around a planet names (`terran`, `barren`, …) are
 * never giants.
 */
export const GIANT_CLASSES: ReadonlySet<string> = new Set(['gas-giant', 'ice-giant', 'hot-jupiter', 'mini-neptune']);

/** Whether a body type (a class key or a preset name) is drawn as a giant. */
export const isGiantType = (type: string): boolean => GIANT_CLASSES.has(type);

/**
 * The preset names a request can name for a planet that hold liquid water
 * and life: the Earth-like `terran` and `earth-like`. A planet asked for
 * has no model to read the water from.
 */
const HABITABLE_PRESETS: ReadonlySet<string> = new Set(['terran', 'earth-like']);

/**
 * The classes whose planet can hold liquid water, for a summary that
 * carries the class only (not the model): inside the habitable zone, they
 * are the candidates.
 */
export const HABITABLE_CLASSES: ReadonlySet<string> = new Set(['rocky', 'super-earth', 'ocean-world']);

/** Whether a summary's body type can be a habitable world. */
export const canBeHabitable = (type: string): boolean => HABITABLE_PRESETS.has(type) || HABITABLE_CLASSES.has(type);

/**
 * Whether a planet has liquid water on its surface, as its model says. A
 * planet drawn by a request has no model: its preset name decides. A moon
 * is never counted.
 */
export function isHabitable(body: Body): boolean {
  if (body.kind !== 'planet') return false;
  if (!body.model) return HABITABLE_PRESETS.has(body.type);
  return body.model.surface?.climate.liquid === 'water';
}
