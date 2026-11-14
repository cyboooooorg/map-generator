// The planets step of the system form: planets and moons are asked in one
// map, as the request has it (`WholeSystemRequest.planets`).

const asked = (counts: Record<string, number>): [string, number][] =>
  Object.entries(counts).filter(([, n]) => Number.isFinite(n) && n > 0);

/** The request's `planets`: the asked planets and moons together; null when nothing is asked. */
export function planetsRequest(planets: Record<string, number>, moons: Record<string, number>): Record<string, number> | null {
  const merged = Object.fromEntries([...asked(planets), ...asked(moons)]);
  return Object.keys(merged).length === 0 ? null : merged;
}

/**
 * Giants the asked moons add to the system: one per host archetype that
 * is not asked itself. `hostOf` maps a moon archetype to its host's.
 */
export function hostsAdded(moons: Record<string, number>, planets: Record<string, number>, hostOf: Record<string, string>): number {
  const hosts = new Set(asked(moons).map(([name]) => hostOf[name]).filter((h): h is string => h !== undefined));
  return [...hosts].filter((host) => (planets[host] ?? 0) <= 0).length;
}
