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
  if (!choice.metallicityRandom) request.metallicity = choice.metallicity;
  if (giantWorld) {
    if (!choice.massRandom) request.mass_earth = choice.mass;
    return;
  }
  if (choice.giants !== 'auto') request.giants = choice.giants;
  if (choice.host !== 'auto') request.host = choice.host;
}
