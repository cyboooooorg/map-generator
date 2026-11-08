// What the Light and season panel reads of the orbit clock.

/** The day and the length of the orbit the panel states. */
export interface OrbitReadout {
  /** Days into the orbit. */
  day: number;
  /** Days of the whole orbit. */
  period: number;
}

/**
 * "Day D of a P-day orbit". A moon states its own orbit (the clock's days
 * past the season); a planet states its year, the day of it being the
 * clock's (days since its spring equinox, see seasonClock.ts). Never the slider's span
 * (a moon's orbit, or ten days of a planet without moons), which is not an
 * orbit of the body shown.
 */
export function orbitReadout(body: { parent?: string | null; orbitPeriodDays: number }, yearDay: number, orbitDays: number): OrbitReadout | null {
  const period = body.orbitPeriodDays;
  if (!(period > 0)) return null;
  const days = body.parent ? orbitDays : yearDay;
  return { day: ((days % period) + period) % period, period };
}
