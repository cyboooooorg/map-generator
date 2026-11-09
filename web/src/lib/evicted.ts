// Bodies whose files were evicted after a while unopened (docs/system-spec.md,
// *Storage*): not generated, like a body never generated, but opening
// their view generates them again (the Viewer queues the generation of any
// body it shows that is not generated, see `generateShown`), and the user
// is told once per body when the current generator draws them differently.
import type { RegeneratedBody } from './api/types/RegeneratedBody';
import type { System } from './api/types/System';

/** What identifies a body in the set below: `<system>/<body>`. */
export function evictedKey(system: string, body: string): string {
  return `${system}/${body}`;
}

/** The bodies the user was told of in this session, by `evictedKey`. */
export const toldOlder = new Set<string>();

/** The evicted bodies whose opening the user was told of, by `evictedKey`. */
export const toldEvicted = new Set<string>();

/**
 * Whether the user is to be told that the evicted `body`, opened now, is
 * made again: once per body until `forgetTold` (its generation ended).
 */
export function toTellEvicted(system: string, body: string, told: Set<string> = toldEvicted): boolean {
  const key = evictedKey(system, body);
  if (told.has(key)) return false;
  told.add(key);
  return true;
}

/**
 * The bodies of `regenerated` (what the server says a job makes again by
 * another generator, see `GenerateStarted`) that the user is to be told
 * of: the server says so on every answer while the job runs, and the user
 * hears it once per body (`told`, which this adds them to).
 */
export function toTell(system: string, regenerated: RegeneratedBody[], told: Set<string> = toldOlder): string[] {
  const fresh: string[] = [];
  for (const { body } of regenerated) {
    const key = evictedKey(system, body);
    if (told.has(key)) continue;
    told.add(key);
    fresh.push(body);
  }
  return fresh;
}

/** A toast about a body made again: which message, for which body. */
export interface RemadeToast {
  kind: 'olderWorld' | 'evictedRemade' | 'evictedRemadeOlder';
  body: string;
}

/**
 * The toasts for the answer to the generation of `body` (the one opened):
 * `wasEvicted` says it lost its files, read before the request was sent
 * (the system may be listed again while the request is in flight, and no
 * longer name it); `regenerated` is what the server says the job makes
 * again by another generator. A body evicted and made by an older
 * generator gets one toast saying both.
 */
export function remadeToasts(
  system: string,
  body: string,
  wasEvicted: boolean,
  regenerated: RegeneratedBody[],
  toldOlderSet: Set<string> = toldOlder,
  toldEvictedSet: Set<string> = toldEvicted,
): RemadeToast[] {
  const remade = wasEvicted && toTellEvicted(system, body, toldEvictedSet);
  const older = toTell(system, regenerated, toldOlderSet);
  const out: RemadeToast[] = older.map((named) => ({
    kind: remade && named === body ? 'evictedRemadeOlder' : 'olderWorld',
    body: named,
  }));
  if (remade && !older.includes(body)) out.push({ kind: 'evictedRemade', body });
  return out;
}

/** Forgets what the user was told of `system`'s bodies: a later eviction tells again. */
export function forgetTold(system: string, ...sets: Set<string>[]): void {
  for (const told of sets.length > 0 ? sets : [toldOlder, toldEvicted]) {
    for (const key of [...told]) {
      if (key.startsWith(`${system}/`)) told.delete(key);
    }
  }
}

/**
 * How the user knows a body: the name given to it, else its designation,
 * else its id (the system is not loaded).
 */
export function bodyTitle(system: Pick<System, 'bodies'> | null, names: Record<string, string>, body: string): string {
  return names[body] ?? system?.bodies.find((b) => b.id === body)?.designation ?? body;
}
