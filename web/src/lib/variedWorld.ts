// What the generation form keeps of a world it varies (Vary, the drift
// slider): the planet it was asked as, and the inputs it formed with, which
// the form has no fields for until step 5 of #423.
import type { WorldRequest } from './api/types/WorldRequest';

/** The model inputs a world's request pins (`requestFromWorld`, from world.json's `formation`). */
export const MODEL_INPUTS = [
  'mass_earth',
  'core_mass_fraction',
  'water_oceans',
  'envelope_fraction',
  'co2',
  'n2',
  'radiogenic',
  'life',
] as const satisfies readonly (keyof WorldRequest)[];

/**
 * Whether the form starts on "drawn from the star": for a varied world, when
 * it was asked as no archetype (a planet of a drawn system, a world drawn
 * from its context), else as the form memory says.
 */
export function startsDrawn(initial: WorldRequest | null | undefined, remembered: boolean | undefined): boolean {
  return initial ? initial.planet === undefined : remembered === true;
}

/**
 * Copies into `request` the model inputs `initial` pins, while `request`
 * asks for the same planet as the varied world: the same archetype
 * (`initialSpec`, the catalog spec its archetype name resolves to, which a
 * request sends), or none. The planet stays the one varied. A value
 * `request` already sets (a giant's mass typed in the form) wins. Another
 * archetype picked, or one this server no longer offers, drops them all.
 */
export function keepModelInputs(
  request: WorldRequest,
  initial: WorldRequest | null | undefined,
  initialSpec: string | undefined,
): void {
  if (!initial || (initial.planet !== undefined && initialSpec === undefined) || request.planet !== initialSpec) {
    return;
  }
  const kept: WorldRequest = {};
  for (const key of MODEL_INPUTS) {
    if (initial[key] !== undefined && request[key] === undefined) {
      Object.assign(kept, { [key]: initial[key] });
    }
  }
  Object.assign(request, kept);
}

/** Whether a request would carry the varied world's model inputs (see {@link keepModelInputs}). */
export function keepsModelInputs(
  planet: string | undefined,
  initial: WorldRequest | null | undefined,
  initialSpec: string | undefined,
): boolean {
  const probe: WorldRequest = planet === undefined ? {} : { planet };
  keepModelInputs(probe, initial, initialSpec);
  return Object.keys(probe).length > (planet === undefined ? 0 : 1);
}

/** What {@link isGiantWorld} reads of the form. */
export interface GiantWorldInputs {
  /** "Drawn from the star" is checked. */
  randomPlanet: boolean;
  /** The archetype picked in the list (its `giant`, as the server's model derives it). */
  archetype: { giant: boolean } | undefined;
  /** The varied world's request, null for a new world. */
  initial: WorldRequest | null | undefined;
  /** The catalog spec of the varied world's archetype (see {@link keepModelInputs}). */
  initialSpec: string | undefined;
  /** The varied world is a giant (world.json's `giant`). */
  variedGiant: boolean;
}

/**
 * Whether the form's planet takes the giant path (no drift, moons or giant
 * neighbours; a giant's mass instead): the archetype picked says so, or,
 * drawn from the star, the request keeps the inputs of a varied giant, which
 * has no archetype when it comes from a drawn system.
 */
export function isGiantWorld(f: GiantWorldInputs): boolean {
  if (!f.randomPlanet) return f.archetype?.giant === true;
  return f.variedGiant && keepsModelInputs(undefined, f.initial, f.initialSpec);
}
