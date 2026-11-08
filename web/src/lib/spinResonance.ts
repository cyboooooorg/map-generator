/**
 * The spin-orbit resonance a planet is held in by its star's tides: Mercury's
 * 3:2, or the 2:1 that large eccentricities favour (see
 * docs/science/tidal-locking.md). Worlds and systems made before the others
 * were drawn have `spin_orbit_3_2` only.
 */
export interface ResonantSpin {
  locked?: boolean | undefined;
  spin_orbit_3_2?: boolean | undefined;
  resonance_turns?: number | undefined;
}

/** Turns per orbit of a planet in a resonance (1.5, 2 or 2.5), 0 for none. */
export function resonanceTurns(spin: ResonantSpin): number {
  if (spin.locked) return 0;
  if (spin.resonance_turns && spin.resonance_turns > 0) return spin.resonance_turns;
  return spin.spin_orbit_3_2 ? 1.5 : 0;
}

/** `3:2`, `2:1` or `5:2` for 1.5, 2 or 2.5 turns per orbit. */
export function resonanceRatio(turns: number): string {
  const twice = Math.round(turns * 2);
  return twice % 2 === 0 ? `${twice / 2}:1` : `${twice}:2`;
}

/** The whole turns and orbits of a ratio such as `3:2` (3 turns every 2 orbits) or `2:1`. */
export function resonanceCycle(ratio: string): { turns: number; orbits: number } {
  const [turns, orbits] = ratio.split(':').map(Number);
  return { turns: turns ?? 0, orbits: orbits ?? 1 };
}
