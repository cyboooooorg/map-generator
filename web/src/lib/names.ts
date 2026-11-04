// The names of a world's system, as the server gives them in its summary
// (see `WorldNames`): scientific designations, and the names given to the
// world and its moons.
import type { GivenNames } from './api/types/GivenNames';
import type { WorldNames } from './api/types/WorldNames';

/** Longest name a world or a moon can be given (the server's `MAX_NAME_CHARS`). */
export const MAX_NAME_CHARS = 60;

/** The world's name: the one given to it, else its designation. */
export function worldName(names: WorldNames): string {
  return names.name ?? names.designation;
}

/** Moon `n`'s name (1 is the closest): the one given to it, else its designation. */
export function moonName(names: WorldNames, n: number): string | null {
  const moon = names.moons[n - 1];
  return moon ? (moon.name ?? moon.designation) : null;
}

/** The designation under a given name, to show beside it; null when the name is the designation. */
export function designationBeside(name: string | null, designation: string): string | null {
  return name && name !== designation ? designation : null;
}

/**
 * The names to send to the server from edited fields: each trimmed, and
 * those left empty or equal to their designation left out (back to it).
 */
export function givenNames(names: WorldNames, world: string, moons: readonly string[]): GivenNames {
  const kept = (value: string | undefined, designation: string): string | null => {
    const trimmed = (value ?? '').trim();
    return trimmed === '' || trimmed === designation ? null : trimmed;
  };
  const name = kept(world, names.designation);
  const given: GivenNames = { moons: {} };
  if (name !== null) given.name = name;
  names.moons.forEach((moon, k) => {
    const value = kept(moons[k], moon.designation);
    if (value !== null) given.moons[String(k + 1)] = value;
  });
  return given;
}
