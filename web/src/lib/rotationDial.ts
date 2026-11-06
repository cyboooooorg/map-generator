// The figures of a body's rotation dial (the ring drawn round the planet
// view): where the pointer reads on the ring, which way the star and the
// host lie on the body's equator, and the lengths of its days.
//
// A free body's ring is graduated against the stars: 0° at its equinox (the
// direction its equatorial frame is built from, see systemSky's `equator`),
// the pointer starting from its longitude 0 and reading how far the body
// has turned. A locked body shows no turn against the stars worth reading,
// so its ring is graduated against its host (the planet a moon orbits, the
// star for a planet), the pointer starting from the face turned to the host
// (longitude 180°) and reading the angle the libration swings it by.
//
// Everything is in the shown body's frame, `[cos lat sin lon, sin lat, cos
// lat cos lon]`, y along its spin axis; angles about +y run from +z to +x,
// as a longitude does.
import type { System } from './api/types/System';
import { libration, placeSystem } from './systemSky';
import type { SkyBody, Vec3 } from './skyBodies';

export interface DialState {
  /** Synchronous: the ring is graduated against the host, the pointer starts from the face turned to it. */
  locked: boolean;
  /** The ring's 0°, a unit vector in the body's equatorial plane (its frame, y the spin axis). */
  zero: Vec3;
  /** Longitude the pointer starts from, radians: 0 for a free body, π for a locked one. */
  pointerLongitude: number;
  /** The angle the pointer reads on the ring, radians: [0, 2π) for a free body, (−π, π] for a locked one. */
  reading: number;
  /** The star's direction projected on the equator (unit), null when within 1° of the spin axis. */
  sun: Vec3 | null;
  /** The planet a moon orbits, projected on the equator (unit), null for a planet. */
  host: Vec3 | null;
}

export interface DialFigures {
  siderealHours: number;
  /** Null when the sun stands still in the body's sky (a planet locked to its star). */
  solarHours: number | null;
  /** Local time at the pointer's meridian, in local hours [0, 24): noon when the sun crosses it. */
  localHours: number;
  /** Synchronous bodies only (null otherwise): the libration now and its amplitude, 2e, radians. */
  libration: number | null;
  librationAmplitude: number | null;
}

const TAU = 2 * Math.PI;
/** A direction this close to the spin axis (sine of 1°) has no meaningful bearing on the equator. */
const POLAR = Math.sin(Math.PI / 180);
/** Below this difference of turns per hour, a day is too long to be worth a figure. */
const STILL = 1e-9;

/** `x` brought into [0, period). */
const wrap = (x: number, period: number) => {
  const r = x - period * Math.floor(x / period);
  // A tiny negative `x` rounds up to `period` itself.
  return r >= period ? 0 : r;
};
/** An angle brought into (−π, π]. */
const wrapSigned = (x: number) => {
  const r = wrap(x + Math.PI, TAU) - Math.PI;
  return r === -Math.PI ? Math.PI : r;
};
/** A direction's projection on the equatorial plane, unit; null when it is along the spin axis. */
const onEquator = (v: Vec3): Vec3 | null => {
  const h = Math.hypot(v[0], v[2]);
  return h < POLAR * Math.hypot(...v) || h === 0 ? null : [v[0] / h, 0, v[2] / h];
};
/** The angle of a direction about +y, from +z towards +x. */
const bearing = (v: Vec3) => Math.atan2(v[0], v[2]);

/**
 * The dial of body `shown` after `days`, `spinOffset` radians of extra turn
 * (see placeSystem), `sky` being its sky (see systemSky); null when the
 * system has no such body.
 */
export function rotationDial(
  system: System,
  shown: string,
  days: number,
  spinOffset: number,
  sky: { sun: Vec3; bodies: SkyBody[]; ids: string[] },
): { state: DialState; figures: DialFigures } | null {
  const self = system.bodies.find((b) => b.id === shown);
  const here = placeSystem(system, days, { id: shown, spinOffset }).get(shown);
  if (!self || !here) return null;
  const parent = self.parent ? system.bodies.find((b) => b.id === self.parent) : undefined;
  const locked = self.spin.locked;

  const sun = onEquator(sky.sun);
  const hostBody = parent ? sky.bodies[sky.ids.indexOf(parent.id)] : undefined;
  const host = hostBody ? onEquator(hostBody.direction) : null;

  let zero: Vec3;
  let pointerLongitude: number;
  let reading: number;
  if (locked) {
    // The host's bearing, or the sun's for a planet; failing both (the host
    // along the axis), the meridian the body would face on average.
    zero = (parent ? host : sun) ?? [0, 0, 1];
    pointerLongitude = Math.PI;
    // The pointer stands at [0, 0, −1]: bearing π.
    reading = wrapSigned(Math.PI - bearing(zero));
  } else {
    const [ex, , ez] = here.equator;
    const z = here.axes[2];
    reading = wrap(Math.atan2(dot(z, ex), dot(z, ez)), TAU);
    zero = [-Math.sin(reading), 0, Math.cos(reading)];
    pointerLongitude = 0;
  }

  // The year is the orbit round the star: the planet's, for a moon.
  const yearHours = (parent ?? self).orbit.period_days * 24;
  const siderealHours = locked ? self.orbit.period_days * 24 : self.spin.period_hours;
  const rate = 1 / siderealHours - 1 / yearHours;
  const solarHours = !parent && locked ? null : Math.abs(rate) < STILL ? null : 1 / rate;
  const localHours = wrap(12 + (24 * (pointerLongitude - bearing(sky.sun))) / TAU, 24);

  return {
    state: { locked, zero, pointerLongitude, reading, sun, host: parent ? host : null },
    figures: {
      siderealHours,
      solarHours,
      localHours,
      libration: locked ? libration(self.orbit, days) : null,
      librationAmplitude: locked ? 2 * self.orbit.eccentricity : null,
    },
  };
}

const dot = (a: Vec3, b: Vec3) => a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
