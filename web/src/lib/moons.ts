// Moons in the generation form and in a world's metadata, and file names
// for body ids (a body of a system is `systems/<system>/bodies/<body>`).
import type { Moon } from './api/types/Moon';
import type { MoonsMode } from './api/types/MoonsMode';
import type { WorldMetadata } from './api/types/WorldMetadata';

/** The moons of a world, nearest first (none for a world made before them). */
export function moonsOf(meta: WorldMetadata | null): readonly Moon[] {
  return meta?.moons?.moons ?? [];
}

/** A file name for a body id: the slashes of a moon's id would make folders. */
export function bodyFileName(id: string): string {
  return id.replaceAll('/', '-');
}

/** The moons the generation form asks for: drawn by the seed, none, or a number. */
export type MoonChoice = 'auto' | 'none' | number;

/** The form's choice for a request's `moons` (absent: drawn by the seed). */
export function moonChoiceOf(moons: MoonsMode | undefined | null): MoonChoice {
  if (moons === 'none') return 'none';
  if (moons && typeof moons === 'object') return moons.count;
  return 'auto';
}

/** A request's `moons` for the form's choice (absent for `auto`, the default). */
export function moonsRequest(choice: MoonChoice): MoonsMode | undefined {
  if (choice === 'auto') return undefined;
  if (choice === 'none') return 'none';
  return { count: choice };
}

/** A remembered choice, if well formed. */
export function parseMoonChoice(raw: unknown, max: number): MoonChoice {
  if (raw === 'none') return 'none';
  if (typeof raw === 'number' && Number.isInteger(raw) && raw >= 1 && raw <= max) return raw;
  return 'auto';
}
