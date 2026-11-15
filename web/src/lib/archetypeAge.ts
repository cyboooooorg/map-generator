// The age of the system an archetype sets (`inputs.age_gyr`): the Solar
// System's archetypes carry one, the exoplanet ones leave the star's age free.
import type { ArchetypeInfo } from './api/types/ArchetypeInfo';
import { archetypeName } from './i18n/names.svelte';

/** The age (Gyr) an archetype sets: its fixed value, or a range's midpoint for display; null when it sets none. */
export function archetypeAge(archetype: ArchetypeInfo): number | null {
  const age = archetype.inputs.age_gyr;
  if (age === undefined || age === null) return null;
  return typeof age === 'number' ? age : (age[0] + age[1]) / 2;
}

/**
 * The names (localised, in name order, each once) of the archetypes asked
 * (count above 0, by archetype name) that set the system's age.
 */
export function ageSources(archetypes: readonly ArchetypeInfo[], asked: Record<string, number>): string[] {
  const names = archetypes
    .filter((a) => (asked[a.name] ?? 0) > 0 && archetypeAge(a) !== null)
    .map((a) => archetypeName(a));
  return [...new Set(names)].sort((a, b) => a.localeCompare(b));
}
