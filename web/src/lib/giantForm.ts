// The generation form's giant-planet fields: the star's metallicity, the
// giant neighbours, whether the world is a giant's moon, and a giant
// world's mass. Each is drawn unless asked for.
import type { GiantsMode } from './api/types/GiantsMode';
import type { HostMode } from './api/types/HostMode';
import type { WorldRequest } from './api/types/WorldRequest';

export interface GiantChoice {
  metallicityRandom: boolean;
  /** `[Fe/H]`, dex. */
  metallicity: number;
  giants: GiantsMode;
  /** A lone planet: its system holds no other planet. */
  alone: boolean;
  host: HostMode;
  massRandom: boolean;
  /** A giant world's mass, Earth masses. */
  mass: number;
}

/** Jupiter's mass, Earth masses: what the giant mass field shows until set. */
const JUPITER_MASS = 318;

/**
 * The choice a form starts from: drawn, or a varied world's values. Its
 * mass fills the giant mass field only when the varied world is a giant
 * (`giant`): a rocky world's mass would turn a giant archetype picked
 * afterwards into a small world.
 */
export function initialGiantChoice(initial: WorldRequest | null | undefined, giant = false): GiantChoice {
  const mass = giant ? initial?.mass_earth : undefined;
  return {
    metallicityRandom: initial?.metallicity === undefined,
    metallicity: initial?.metallicity ?? 0,
    giants: initial?.giants ?? 'auto',
    alone: initial?.alone === true,
    host: initial?.host ?? 'auto',
    massRandom: mass === undefined,
    mass: mass ?? JUPITER_MASS,
  };
}

/**
 * The choice as the form memory keeps it: without the giant's mass, a model
 * input, which the next form must not carry to another planet.
 */
export function rememberedGiantChoice(choice: GiantChoice): GiantChoice {
  return { ...choice, massRandom: true, mass: JUPITER_MASS };
}

/**
 * Writes the choice into a request: a giant world takes only its mass
 * (it has no giant neighbours and is nobody's moon) and whether it is alone.
 */
export function applyGiantChoice(request: WorldRequest, choice: GiantChoice, giantWorld: boolean): void {
  // A field emptied by the user holds no number: drawn, as when random.
  if (!choice.metallicityRandom && Number.isFinite(choice.metallicity)) request.metallicity = choice.metallicity;
  // A lone planet has no giant neighbours: the server overrides `giants`.
  if (choice.alone) request.alone = true;
  if (giantWorld) {
    if (!choice.massRandom && Number.isFinite(choice.mass) && choice.mass > 0) request.mass_earth = choice.mass;
    return;
  }
  if (choice.giants !== 'auto' && !choice.alone) request.giants = choice.giants;
  if (choice.host !== 'auto') request.host = choice.host;
}

/**
 * Reads a remembered choice, keeping only well-formed values: each one
 * missing or malformed takes the drawn default of `initialGiantChoice`. A
 * mass remembered by an older form is dropped (see
 * {@link rememberedGiantChoice}).
 */
export function parseGiantChoice(raw: unknown): GiantChoice {
  const choice = initialGiantChoice(null);
  if (typeof raw !== 'object' || raw === null) return choice;
  const r = raw as Record<string, unknown>;
  const finite = (v: unknown): v is number => typeof v === 'number' && Number.isFinite(v);
  if (typeof r.metallicityRandom === 'boolean') choice.metallicityRandom = r.metallicityRandom;
  if (finite(r.metallicity)) choice.metallicity = r.metallicity;
  if (r.giants === 'auto' || r.giants === 'none') choice.giants = r.giants;
  if (typeof r.alone === 'boolean') choice.alone = r.alone;
  if (r.host === 'auto' || r.host === 'none' || r.host === 'giant') choice.host = r.host;
  return choice;
}
