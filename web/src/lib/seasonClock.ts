// The season slider of a planet of a system, as a time on its orbit.
//
// placeSystem puts a planet at `node cos θ + third sin θ`, θ = periapsis +
// true anomaly, and leans its axis from the orbit's normal by its tilt about
// the node line: the sun's latitude is asin(sin θ sin tilt). The northern
// spring equinox is θ = 0 (the planet on its node line), the summer solstice
// θ = 90°, the autumn equinox θ = 180°, the winter solstice θ = 270°. The
// season `s` (0 to 1) is θ = 360° · s, and its time follows from Kepler's
// equation: the quarters are the equinoxes and solstices, the seasons of an
// eccentric orbit unequal in days. The system's epoch is at an arbitrary θ,
// so days of the year count from the northern spring equinox instead.
import type { Orbit } from './api/types/Orbit';

const DEG = Math.PI / 180;
const TAU = 2 * Math.PI;
const wrap = (x: number, period: number) => x - period * Math.floor(x / period);

const clampEcc = (e: number) => Math.min(Math.max(e, 0), 0.95);

/** The mean anomaly, radians, of true anomaly `nu` on an orbit of eccentricity `e`. */
function meanOfTrue(e: number, nu: number): number {
  const eccentric = 2 * Math.atan2(Math.sqrt(1 - e) * Math.sin(nu / 2), Math.sqrt(1 + e) * Math.cos(nu / 2));
  return eccentric - e * Math.sin(eccentric);
}

/** The true anomaly, radians, of mean anomaly `mean` (Kepler's equation, Newton's method). */
function trueOfMean(e: number, mean: number): number {
  const m = wrap(mean + Math.PI, TAU) - Math.PI;
  let eccentric = e < 0.8 ? m : Math.PI;
  for (let i = 0; i < 12; i += 1) {
    eccentric -= (eccentric - e * Math.sin(eccentric) - m) / (1 - e * Math.cos(eccentric));
  }
  return 2 * Math.atan2(Math.sqrt(1 + e) * Math.sin(eccentric / 2), Math.sqrt(1 - e) * Math.cos(eccentric / 2));
}

/** Days from the system's epoch, in [0, period), when the planet crosses angle `theta` (radians) of its orbit from the node. */
function timeAt(orbit: Orbit, theta: number): number {
  const e = clampEcc(orbit.eccentricity);
  const mean = meanOfTrue(e, theta - orbit.periapsis_deg * DEG);
  const turns = (mean - orbit.mean_anomaly_deg * DEG) / TAU;
  return wrap(turns, 1) * orbit.period_days;
}

/** Days from the epoch of the northern spring equinox (θ = 0) of a planet's year, in [0, period). */
export function equinoxTime(orbit: Orbit): number {
  return timeAt(orbit, 0);
}

/** Days since the northern spring equinox at season `season` (0 to 1: θ = 360° · season), in [0, period). */
export function yearDayOfSeason(orbit: Orbit, season: number): number {
  return wrap(timeAt(orbit, TAU * season) - equinoxTime(orbit), orbit.period_days);
}

/** The season (0 to 1, from the spring equinox) `yearDay` days after the northern spring equinox. */
export function seasonOfYearDay(orbit: Orbit, yearDay: number): number {
  const e = clampEcc(orbit.eccentricity);
  const days = equinoxTime(orbit) + yearDay;
  const mean = orbit.mean_anomaly_deg * DEG + (TAU * days) / Math.max(orbit.period_days, 1e-6);
  return wrap((orbit.periapsis_deg * DEG + trueOfMean(e, mean)) / TAU, 1);
}

/** The clock time (days from the system's epoch) at season `season`, `past` days later. */
export function clockDays(orbit: Orbit, season: number, past = 0): number {
  return equinoxTime(orbit) + yearDayOfSeason(orbit, season) + past;
}

/** The day of the year (days since the northern spring equinox) the clock shows at `days` from the epoch. */
export function yearDayAt(orbit: Orbit, days: number): number {
  return wrap(days - equinoxTime(orbit), orbit.period_days);
}
