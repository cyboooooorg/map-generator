// The moons of a world as bodies the viewer can show: a moon is a world of
// its own, generated with its planet in `moons/<n>/` of its directory, and
// named `<world>/moons/<n>` (n from 1, nearest first) wherever an id goes.
import type { Moon } from './api/types/Moon';
import type { WorldMetadata } from './api/types/WorldMetadata';

/** The body id of moon `n` (from 1) of world `worldId`. */
export function moonBody(worldId: string, n: number): string {
  return `${worldId}/moons/${n}`;
}

/** The world and the moon (from 1, null for the world itself) of a body id. */
export function parseBody(id: string): { world: string; moon: number | null } {
  const [, world, moon] = /^(.+)\/moons\/(\d+)$/.exec(id) ?? [];
  return world && moon ? { world, moon: Number(moon) } : { world: id, moon: null };
}

/** The moons of a world, nearest first (none for a world made before them). */
export function moonsOf(meta: WorldMetadata | null): readonly Moon[] {
  return meta?.moons?.moons ?? [];
}

/** A file name for a body id: the slashes of a moon's id would make folders. */
export function bodyFileName(id: string): string {
  return id.replaceAll('/', '-');
}
