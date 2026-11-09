// Bodies whose files were evicted after a while unopened (docs/system-spec.md,
// *Storage*): not generated, like a body never generated, but opening
// their view generates them again, and the user is told once when the
// current generator draws them differently.
import type { GeneratorNote } from './api/types/GeneratorNote';

/** What identifies a body in the sets below: `<system>/<body>`. */
export function evictedKey(system: string, body: string): string {
  return `${system}/${body}`;
}

/**
 * The key of the body whose generation the view being opened must queue:
 * `body` of `system` when it is evicted and was not asked for yet in this
 * session (`asked`), else null. A body never generated stays for the
 * user to generate from its card.
 */
export function regenerationToQueue(
  system: string | null,
  body: string | null,
  evicted: readonly string[],
  asked: ReadonlySet<string>,
): string | null {
  if (!system || !body || !evicted.includes(body)) return null;
  const key = evictedKey(system, body);
  return asked.has(key) ? null : key;
}

/**
 * Whether the user is to be told that `key` is made again by another
 * generator: the server says so (`note`) on every answer while the job
 * runs, and the user hears it once (`told`, which this adds `key` to).
 */
export function shouldTell(note: GeneratorNote | null, key: string, told: Set<string>): boolean {
  if (!note || told.has(key)) return false;
  told.add(key);
  return true;
}
