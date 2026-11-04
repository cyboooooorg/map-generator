// The moons of a world as bodies the viewer can show: a moon is a world of
// its own, generated with its planet in `moons/<n>/` of its directory, and
// named `<world>/moons/<n>` (n from 1, nearest first) wherever an id goes.
import type { Moon } from './api/types/Moon';
import type { MoonsMode } from './api/types/MoonsMode';
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

/**
 * A body of a world's system the planet view can show: the world itself,
 * one of its moons, or, when the world is the moon of a giant, that giant
 * (`host/` of its directory) or one of the giant's other moons
 * (`host/siblings/<n>/`, n from 1 in the order of `host.siblings`).
 */
export type Body = { kind: 'world' } | { kind: 'moon'; n: number } | { kind: 'host' } | { kind: 'sibling'; n: number };

/** The world itself. */
export const WORLD_BODY: Body = { kind: 'world' };

/** The body id of `body` of world `worldId`, as file and API paths take it. */
export function bodyId(worldId: string, body: Body): string {
  switch (body.kind) {
    case 'world':
      return worldId;
    case 'moon':
      return moonBody(worldId, body.n);
    case 'host':
      return `${worldId}/host`;
    case 'sibling':
      return `${worldId}/host/siblings/${body.n}`;
  }
}

/** Whether two bodies are the same. */
export function sameBody(a: Body, b: Body): boolean {
  return a.kind === b.kind && ('n' in a ? a.n : 0) === ('n' in b ? b.n : 0);
}

/** A body as a short key, for a `<select>` value: `world`, `moon:2`, `host`, `sibling:1`. */
export function bodyKey(body: Body): string {
  return 'n' in body ? `${body.kind}:${body.n}` : body.kind;
}

/** The body of a key made by `bodyKey`, or null when it names none. */
export function parseBodyKey(key: string): Body | null {
  if (key === 'world' || key === 'host') return { kind: key };
  const [, kind, n] = /^(moon|sibling):(\d+)$/.exec(key) ?? [];
  if (!kind || !n || Number(n) < 1) return null;
  return { kind: kind as 'moon' | 'sibling', n: Number(n) };
}

/**
 * The bodies of a world's system the planet view can show, in the order of
 * the system: a giant whose moon the world is, then the giant's moons
 * (the world among its siblings, by distance); else the world, then its
 * moons, nearest first.
 */
export function systemBodies(meta: WorldMetadata | null): Body[] {
  const host = meta?.host;
  if (host) {
    const moons = [
      { body: WORLD_BODY, distance: host.distance_km },
      ...(host.siblings ?? []).map((s, k) => ({ body: { kind: 'sibling', n: k + 1 } as Body, distance: s.distance_km })),
    ].sort((a, b) => a.distance - b.distance);
    return [{ kind: 'host' }, ...moons.map((m) => m.body)];
  }
  return [WORLD_BODY, ...moonsOf(meta).map((_, k): Body => ({ kind: 'moon', n: k + 1 }))];
}

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
