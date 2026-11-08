// The logic of the planet view's display switches that come in two parts
// (see components/dock/ToggleBar.svelte): a switch on something both the
// body shown and its host planet have (atmosphere, grid, spin axis, …),
// whose main button sets both and whose two squares set one each; and a
// switch on something system-wide (labels, orbits, small bodies), whose
// two squares set the local group (the host planet and its moons) and the
// rest of the star system (stellar).

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
  ];
  for (const [key, from] of copies) {
    if (old[key] === undefined && typeof old[from] === 'boolean') out[key] = old[from];
  }
  return out as T;
}
