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
  host: HostMode;
  massRandom: boolean;
  /** A giant world's mass, Earth masses. */
  mass: number;
}

/** The choice a form starts from: drawn, or a varied world's values. */
export function initialGiantChoice(initial: WorldRequest | null | undefined): GiantChoice {
  return {
    metallicityRandom: initial?.metallicity === undefined,
    metallicity: initial?.metallicity ?? 0,
    giants: initial?.giants ?? 'auto',
    host: initial?.host ?? 'auto',
    massRandom: initial?.mass_earth === undefined,
    mass: initial?.mass_earth ?? 318,
  };
}

/**
 * Writes the choice into a request: a giant world takes only its mass
 * (it has no giant neighbours and is nobody's moon).
 */
export function applyGiantChoice(request: WorldRequest, choice: GiantChoice, giantWorld: boolean): void {
  // A field emptied by the user holds no number: drawn, as when random.
  if (!choice.metallicityRandom && Number.isFinite(choice.metallicity)) request.metallicity = choice.metallicity;
  if (giantWorld) {
    if (!choice.massRandom && Number.isFinite(choice.mass) && choice.mass > 0) request.mass_earth = choice.mass;
    return;
  }
  if (choice.giants !== 'auto') request.giants = choice.giants;
  if (choice.host !== 'auto') request.host = choice.host;
}

/**
 * Reads a remembered choice, keeping only well-formed values: each one
 * missing or malformed takes the drawn default of `initialGiantChoice`.
 */
export function parseGiantChoice(raw: unknown): GiantChoice {
  const choice = initialGiantChoice(null);
  if (typeof raw !== 'object' || raw === null) return choice;
  const r = raw as Record<string, unknown>;
  const finite = (v: unknown): v is number => typeof v === 'number' && Number.isFinite(v);
  if (typeof r.metallicityRandom === 'boolean') choice.metallicityRandom = r.metallicityRandom;
  if (finite(r.metallicity)) choice.metallicity = r.metallicity;
  if (r.giants === 'auto' || r.giants === 'none') choice.giants = r.giants;
  if (r.host === 'auto' || r.host === 'none' || r.host === 'giant') choice.host = r.host;
  if (typeof r.massRandom === 'boolean') choice.massRandom = r.massRandom;
  if (finite(r.mass) && r.mass > 0) choice.mass = r.mass;
  return choice;
}
