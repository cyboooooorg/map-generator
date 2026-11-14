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
 * Giants the asked moons add to the system: the server gives each moon its
 * own host, so one per asked moon whose host archetype is not asked (two
 * titan-like with no saturn-like add two). A host that is asked is taken to
 * carry its moons; the server's `too_many_planets` error is the fallback.
 * `hostOf` maps a moon archetype to its host's.
 */
export function hostsAdded(moons: Record<string, number>, planets: Record<string, number>, hostOf: Record<string, string>): number {
  return asked(moons)
    .filter(([name]) => {
      const host = hostOf[name];
      return host !== undefined && (planets[host] ?? 0) <= 0;
    })
    .reduce((sum, [, n]) => sum + n, 0);
}
