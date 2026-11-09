// Bodies whose files were evicted after a while unopened (docs/system-spec.md,
// *Storage*): not generated, like a body never generated, but opening
// their view generates them again (the Viewer queues the generation of any
// body it shows that is not generated, see `generateShown`), and the user
// is told once when the current generator draws them differently.
import type { GeneratorNote } from './api/types/GeneratorNote';
import type { System } from './api/types/System';

/** What identifies a body in the set below: `<system>/<body>`. */
export function evictedKey(system: string, body: string): string {
  return `${system}/${body}`;
}

/** The bodies the user was told of in this session, by `evictedKey`. */
export const toldOlder = new Set<string>();

/**
 * Whether the user is to be told that `key` is made again by another
 * generator: the server says so (`note`) on every answer while the job
 * runs, and the user hears it once (`told`, which this adds `key` to).
 */
export function shouldTell(note: GeneratorNote | null, key: string, told: Set<string> = toldOlder): boolean {
  if (!note || told.has(key)) return false;
  told.add(key);
  return true;
}

/** Forgets what the user was told of `system`'s bodies: a later eviction tells again. */
export function forgetTold(system: string, told: Set<string> = toldOlder): void {
  for (const key of [...told]) {
    if (key.startsWith(`${system}/`)) told.delete(key);
  }
}

/**
 * How the user knows a body: the name given to it, else its designation,
 * else its id (the system is not loaded).
 */
export function bodyTitle(system: Pick<System, 'bodies'> | null, names: Record<string, string>, body: string): string {
  return names[body] ?? system?.bodies.find((b) => b.id === body)?.designation ?? body;
}
