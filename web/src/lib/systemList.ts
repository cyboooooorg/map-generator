// The main menu's list of systems: names, and the order with pins.
import type { SystemSummary } from './api/types/SystemSummary';

export type Sort = 'newest' | 'oldest' | 'planets' | 'seed';
export const SORTS: Sort[] = ['newest', 'oldest', 'planets', 'seed'];

/** The body id a system is named by: its star (the server's `STAR`). */
export const SYSTEM_NAME_KEY = 'star';

/**
 * A system whose `system.json` is of another format: only listed, deleted or drawn again (it has no
 * bodies to show). One outdated for its worlds only is a whole system, openable.
 */
export const isUnreadable = (s: Pick<SystemSummary, 'outdated_reason'>): boolean =>
  s.outdated_reason === 'system_format' || s.outdated_reason === 'newer_system_format';

/** Whether the Regenerate button is offered: not for a system a newer generator made (it would destroy it). */
export const canRegenerate = (s: Pick<SystemSummary, 'outdated_reason'>): boolean =>
  s.outdated_reason === 'system_format' || s.outdated_reason === 'world_format';

/** A system's name: the one given to its star, else its designation. */
export const systemName = (s: Pick<SystemSummary, 'names' | 'designation'>): string =>
  s.names[SYSTEM_NAME_KEY] ?? s.designation;

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
