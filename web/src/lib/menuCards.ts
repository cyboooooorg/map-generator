// The main menu's cards: sections by age, filters (star class, habitable
// world), the planet strip and the view setting (components/SystemList.svelte).
import type { BodySummary } from './api/types/BodySummary';
import type { SystemSummary } from './api/types/SystemSummary';
import { canBeHabitable, isGiantType } from './planetClass';
import { loadSetting } from './settings';

// -- Views -------------------------------------------------------------

export type View = 'cards' | 'compact' | 'list';
export const VIEWS: View[] = ['cards', 'compact', 'list'];

/** A stored view, the cards when it is missing or unknown. */
export function parseView(value: unknown): View {
  return VIEWS.includes(value as View) ? (value as View) : 'cards';
}

/** The view the menu opens in: the one remembered in the `systemList` setting. */
export function loadView(): View {
  return parseView(loadSetting('systemList', { view: 'cards' as string }).view);
}

/** The Compact button: cards <-> compact cards. It does not apply to the list. */
export const toggleCompact = (view: View): View => (view === 'compact' ? 'cards' : 'compact');

// -- Sections ----------------------------------------------------------

export type Section = 'pinned' | 'today' | 'older';
export const SECTIONS: Section[] = ['pinned', 'today', 'older'];

/** Local midnight of the day `now` falls in, ms since the epoch. */
export function startOfDay(now: number): number {
  const d = new Date(now);
  d.setHours(0, 0, 0, 0);
  return d.getTime();
}

/**
 * The section of a system: "pinned" if pinned, else "today" when it was
 * drawn since the last local midnight (a system drawn at 23:50 is "older"
 * ten minutes after midnight), else "older". A date in the future (clock
 * skew) is "today".
 */
export function sectionOf(system: Pick<SystemSummary, 'pinned' | 'modified_ms'>, now: number): Section {
  if (system.pinned) return 'pinned';
  return system.modified_ms >= startOfDay(now) ? 'today' : 'older';
}

/** The non-empty sections in order, each keeping the order of `list`. */
export function groupSections<T extends Pick<SystemSummary, 'pinned' | 'modified_ms'>>(
  list: T[],
  now: number,
): { section: Section; items: T[] }[] {
  return SECTIONS.map((section) => ({ section, items: list.filter((s) => sectionOf(s, now) === section) })).filter(
    (g) => g.items.length > 0,
  );
}

// -- Stars -------------------------------------------------------------

/** The spectral letters, hottest first. */
export type StarLetter = 'O' | 'B' | 'A' | 'F' | 'G' | 'K' | 'M';
/**
 * The filter's star classes: M, K, G and F as they are, and "white" for
 * the hot stars (O, B and A: white or blue-white).
 */
export type StarClass = 'M' | 'K' | 'G' | 'F' | 'white';
/** In the chips' order: the commonest first. */
export const STAR_CLASSES: StarClass[] = ['M', 'K', 'G', 'F', 'white'];

/** The spectral letter of a type like `K4.5V`, null when it does not start with one. */
export function starLetter(star: string): StarLetter | null {
  const c = star.trim().charAt(0).toUpperCase();
  return c !== '' && 'OBAFGKM'.includes(c) ? (c as StarLetter) : null;
}

/** The filter class of a spectral type, null when unknown. */
export function starClass(star: string): StarClass | null {
  const letter = starLetter(star);
  if (letter === null) return null;
  return letter === 'O' || letter === 'B' || letter === 'A' ? 'white' : letter;
}

/** The classes found in `systems`, in the chips' order. */
export function presentClasses(systems: Pick<SystemSummary, 'star'>[]): StarClass[] {
  const found = new Set(systems.map((s) => starClass(s.star)));
  return STAR_CLASSES.filter((c) => found.has(c));
}

// -- Habitable worlds --------------------------------------------------

/**
 * Whether a body is a planet of a type that can be habitable (`terran`, `earth-like`,
 * or the classes `rocky`, `super-earth` and `ocean-world`; see `canBeHabitable`;
 * the summary carries no model, so the real water is not known here) whose semi-major axis lies
 * inside the zone [inner, outer] AU. Only planets count: a moon of a giant
 * in the zone does not (its `orbit_au` is round its planet), and a planet
 * on an eccentric orbit that only passes through the zone is judged by its
 * semi-major axis.
 */
export function isHabitableWorld(body: BodySummary, inner: number, outer: number): boolean {
  return (
    body.kind === 'planet' && canBeHabitable(body.body_type) && outer > 0 && body.orbit_au >= inner && body.orbit_au <= outer
  );
}

/** The worlds of a system that sit in its habitable zone and could hold life. */
export const habitableWorlds = (system: SystemSummary): BodySummary[] =>
  system.bodies.filter((b) => isHabitableWorld(b, system.hz_inner_au, system.hz_outer_au));

export const hasHabitableWorld = (system: SystemSummary): boolean => habitableWorlds(system).length > 0;

// -- Filters and search ------------------------------------------------

/** The selection kept to the systems still shown: what a filter hides is no longer selected. */
export function pruneChosen(chosen: string[], shownIds: Iterable<string>): string[] {
  const shown = new Set(shownIds);
  const kept = chosen.filter((id) => shown.has(id));
  return kept.length === chosen.length ? chosen : kept;
}

/** Ms from `now` to the next local midnight. */
export function msToMidnight(now: number): number {
  const d = new Date(now);
  d.setHours(24, 0, 0, 0);
  return d.getTime() - now;
}

export interface Filters {
  /** Empty for any star. */
  classes: StarClass[];
  habitable: boolean;
}
export const NO_FILTERS: Filters = { classes: [], habitable: false };

/** Whether the text is found in the system's id, designation, seed, names or bodies' designations. */
export function matchesQuery(s: SystemSummary, query: string): boolean {
  const q = query.trim().toLowerCase();
  return (
    q === '' ||
    [s.id, s.designation, String(s.seed), ...Object.values(s.names), ...s.bodies.map((b) => b.designation)].some((text) =>
      text.toLowerCase().includes(q),
    )
  );
}

/** The systems that match the search and every filter (the star classes are alternatives, the habitable world is required). */
export function filterSystems(list: SystemSummary[], query: string, filters: Filters): SystemSummary[] {
  return list.filter((s) => {
    if (!matchesQuery(s, query)) return false;
    if (filters.classes.length > 0) {
      const c = starClass(s.star);
      if (c === null || !filters.classes.includes(c)) return false;
    }
    return !filters.habitable || hasHabitableWorld(s);
  });
}

// -- The planet strip --------------------------------------------------

export interface StripPlanet {
  id: string;
  /** Position along the strip, % of its width. */
  at: number;
  giant: boolean;
  colour: [number, number, number];
  generated: boolean;
}

export interface Strip {
  planets: StripPlanet[];
  /** The habitable zone as a band, % of the strip, null when unknown. */
  zone: { from: number; width: number } | null;
}

/**
 * The planets of a system along a line from the star, by the logarithm of
 * their distance (so that tight M-dwarf systems and wide ones both fill the
 * strip), between `margin` and `100 - margin` %.
 */
export function strip(system: SystemSummary, margin = 6): Strip {
  const planets = system.bodies.filter((b) => b.kind === 'planet' && b.orbit_au > 0);
  const hasZone = system.hz_outer_au > 0 && system.hz_inner_au > 0;
  const dists = planets.map((p) => p.orbit_au);
  if (hasZone) dists.push(system.hz_inner_au, system.hz_outer_au);
  if (dists.length === 0) return { planets: [], zone: null };
  const lo = Math.log(Math.min(...dists) * 0.7);
  const hi = Math.log(Math.max(...dists) * 1.25);
  const span = 100 - 2 * margin;
  const at = (au: number) => margin + (span * (Math.log(au) - lo)) / (hi - lo);
  return {
    planets: planets.map((p) => ({
      id: p.id,
      at: at(p.orbit_au),
      giant: isGiantType(p.body_type),
      colour: p.colour,
      generated: p.generated,
    })),
    zone: hasZone ? { from: at(system.hz_inner_au), width: at(system.hz_outer_au) - at(system.hz_inner_au) } : null,
  };
}
