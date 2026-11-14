// The logic of the planet view's display switches that come in two parts
// (see components/dock/ToggleBar.svelte): a switch on something both the
// body shown and its host planet have (atmosphere, grid, spin axis, …),
// whose main button sets both and whose two squares set one each; and a
// switch on something system-wide (labels, orbits, small bodies), whose
// two squares set the local group (the host planet and its moons) and the
// rest of the star system (stellar). The sky switches (labels, small
// bodies, spin axes and arrows) have more parts, one square each (see
// `Part`).

import type { SkyChoice } from './viewer/skyBackdrop';

/** A switch's state on the body shown (a moon, say) and on its host planet. */
export interface BodyPair {
  body: boolean;
  host: boolean;
}

/** A system-wide switch's state on the local group and on the rest of the star system. */
export interface ScopePair {
  local: boolean;
  stellar: boolean;
}

/**
 * The main button of a per-body switch: both bodies take the opposite of
 * the state of the body shown, which the button itself displays.
 */
export function pressBody(pair: BodyPair): BodyPair {
  const on = !pair.body;
  return { body: on, host: on };
}

/** Whether a scoped switch counts as on: any of its scopes is. */
export function scopeOn(pair: ScopePair): boolean {
  return pair.local || pair.stellar;
}

/** The main button of a scoped switch: both scopes take the opposite of what it displays (on when either is). */
export function pressScope(pair: ScopePair): ScopePair {
  const on = !scopeOn(pair);
  return { local: on, stellar: on };
}

/** The bodies of a system, as far as the local group needs them. */
export interface GroupBody {
  id: string;
  parent: string | null;
}

/**
 * The local group of body `shown`: its planet (the body itself when it has
 * no parent) and the moons of that planet, the body shown left out (it is
 * not in its own sky). Empty for a planet without moons.
 */
export function localGroup(bodies: readonly GroupBody[], shown: string): Set<string> {
  const self = bodies.find((b) => b.id === shown);
  if (!self) return new Set();
  const planet = self.parent ?? self.id;
  const ids = bodies.filter((b) => (b.id === planet || b.parent === planet) && b.id !== shown).map((b) => b.id);
  return new Set(ids);
}

/**
 * The bodies of the sky of body `shown`, as the sky switches split them:
 * its host planet (a moon's; null for a planet) and the moons of its
 * planet, the body shown left out (it is not in its own sky). Every other
 * body of the system is the rest of the star system.
 */
export function skyGroups(bodies: readonly GroupBody[], shown: string): { host: string | null; moons: Set<string> } {
  const self = bodies.find((b) => b.id === shown);
  if (!self) return { host: null, moons: new Set() };
  const planet = self.parent ?? self.id;
  return { host: self.parent, moons: new Set(bodies.filter((b) => b.parent === planet && b.id !== shown).map((b) => b.id)) };
}

/**
 * One part of a switch with several squares: its state, and whether it has
 * anything to draw from the body shown (its square shows only then).
 */
export interface Part {
  on: boolean;
  shown: boolean;
}

/**
 * Whether a switch with parts reads as on: by its first part when `lead`
 * (a switch on the body shown, whose other parts follow it), else when any
 * part shown is.
 */
export function partsOn(parts: readonly Part[], lead: boolean): boolean {
  if (lead) return parts[0]?.on ?? false;
  return parts.some((p) => p.shown && p.on);
}

/** The main button of a switch with parts: every part shown takes the opposite of what the button displays. */
export function pressParts(parts: readonly Part[], lead: boolean): boolean {
  return !partsOn(parts, lead);
}

/**
 * Whether something tied to body `id` is shown under `scope`: the local
 * group's things by `local`, those of any other body by `stellar`.
 */
export function inScope(scope: ScopePair, group: ReadonlySet<string>, id: string): boolean {
  return group.has(id) ? scope.local : scope.stellar;
}

/**
 * Display preferences saved before the switches split: the new keys take
 * the value of the key they came from, so that nothing the user had
 * switched on or off changes (`spinAxes` was the axes, the arrows and the
 * link to the host together; the labels, orbits and small bodies were
 * system-wide; the host's clouds drifted with the body's own).
 */
export function migrateDisplay<T extends object>(stored: unknown, current: T): T {
  if (typeof stored !== 'object' || stored === null) return current;
  const old = stored as Record<string, unknown>;
  const out = { ...current } as Record<string, unknown>;
  const copies: [string, string][] = [
    ['spinDirection', 'spinAxes'],
    ['hostLink', 'spinAxes'],
    ['hostSpinAxes', 'spinAxes'],
    ['hostSpinDirection', 'spinAxes'],
    ['skyLabelsLocal', 'skyLabels'],
    ['orbitLinesLocal', 'orbitLines'],
    ['smallBodiesLocal', 'smallBodies'],
    ['hostCloudDrift', 'cloudDrift'],
    // The second split (#430): the local labels into the moons and the
    // host planet, the small bodies into the planet's own trojans, the
    // other planets' and the belts, and the other bodies of the sky out of
    // the spin axes and arrows (from a planet, the body's switch drew its
    // moons' too).
    ['skyLabelsMoons', 'skyLabelsLocal'],
    ['skyLabelsHost', 'skyLabelsLocal'],
    ['trojansOwn', 'smallBodiesLocal'],
    ['trojansOther', 'smallBodies'],
    ['skyBelts', 'smallBodies'],
    ['otherSpinAxes', 'spinAxes'],
    ['otherSpinDirection', 'spinDirection'],
    // The pull and velocity arrows came with the link to the host.
    ['vectors', 'hostLink'],
  ];
  // In order, each copy seeing the ones before: a key from the first split
  // passes its value on to those split from it since.
  const seen: Record<string, unknown> = { ...old };
  for (const [key, from] of copies) {
    if (seen[key] === undefined && typeof seen[from] === 'boolean') out[key] = seen[key] = seen[from];
  }
  // The stars switch became the three-way sky choice (#429): a boolean
  // maps to the system's sky or none, anything else to the system's sky.
  if ('sky' in current) out.sky = parseSky(old.sky ?? old.stars);
  return out as T;
}

/** The sky choices, in the order the `B` key cycles them. */
export const SKY_CHOICES: readonly SkyChoice[] = ['system', 'fixed', 'off'];

/** A stored sky choice: a known one as it is, a boolean (the old stars switch) as the system's sky or none, else the system's. */
export function parseSky(stored: unknown): SkyChoice {
  if (stored === false) return 'off';
  return SKY_CHOICES.find((choice) => choice === stored) ?? 'system';
}

/** The choice after `choice` in the cycle system, fixed, off. */
export function nextSky(choice: SkyChoice): SkyChoice {
  return SKY_CHOICES[(SKY_CHOICES.indexOf(choice) + 1) % SKY_CHOICES.length]!;
}
