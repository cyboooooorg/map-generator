// The main menu's list of systems: names, and the order with pins.
import type { SystemSummary } from './api/types/SystemSummary';

export type Sort = 'newest' | 'oldest' | 'planets' | 'seed';
export const SORTS: Sort[] = ['newest', 'oldest', 'planets', 'seed'];

/** The body id a system is named by: its star (the server's `STAR`). */
export const SYSTEM_NAME_KEY = 'star';

/** A system's name: the one given to its star, else its designation. */
export const systemName = (s: Pick<SystemSummary, 'names' | 'designation'>): string =>
  s.names[SYSTEM_NAME_KEY] ?? s.designation;

/**
 * The names to send to rename a system: `names` with the system's name set
 * to `value`. An empty value, or the designation itself, removes it, so
 * the default comes back; the names of the bodies stay.
 */
export function withSystemName(names: Record<string, string>, value: string, designation: string): Record<string, string> {
  const next = { ...names };
  const trimmed = value.trim();
  if (trimmed === '' || trimmed === designation) delete next[SYSTEM_NAME_KEY];
  else next[SYSTEM_NAME_KEY] = trimmed;
  return next;
}

const planetCount = (s: SystemSummary) => s.bodies.filter((b) => b.kind === 'planet').length;
const COMPARE: Record<Sort, (a: SystemSummary, b: SystemSummary) => number> = {
  newest: (a, b) => b.modified_ms - a.modified_ms,
  oldest: (a, b) => a.modified_ms - b.modified_ms,
  planets: (a, b) => planetCount(b) - planetCount(a) || b.modified_ms - a.modified_ms,
  seed: (a, b) => a.seed - b.seed,
};

/** The systems in the order of `sort`, the pinned ones first (in that same order). Does not change `list`. */
export function sortSystems(list: SystemSummary[], sort: Sort): SystemSummary[] {
  return [...list].sort((a, b) => Number(b.pinned) - Number(a.pinned) || COMPARE[sort](a, b));
}
