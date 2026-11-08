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
import { libration, librationAmplitude, placeSystem, systemSky } from './systemSky';
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
  /** The star's height above the equator, radians, −π/2 to π/2 (see the dial's vertical half-circle). */
  sunAltitude: number;
  /** The other bodies of the planet's system in the sky: from a moon its host and the other moons, from a planet its moons (see DialBody). */
  bodies: DialBody[];
}

/** A body of the sky on the dial: a quarter-circle from the ring up or down to its height. */
export interface DialBody {
  id: string;
  /** Its direction projected on the equator (unit), null when within 1° of the spin axis. */
  bearing: Vec3 | null;
  /** Its height above the equator, radians. */
  altitude: number;
  /** The planet the body shown orbits. */
  host: boolean;
}

export interface DialFigures {
  siderealHours: number;
  /** Null when the sun stands still in the body's sky (a planet locked to its star). */
  solarHours: number | null;
  /** Local time at the pointer's meridian, in local hours [0, 24): noon when the sun crosses it. */
  localHours: number;
  /** Synchronous bodies only (null otherwise): the libration now and its amplitude (about 2e, exact), radians. */
  libration: number | null;
  librationAmplitude: number | null;
  /** Held by its star's tides in the 3:2 spin-orbit resonance (a free dial, three turns every two orbits). */
  resonant: boolean;
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
/** A direction's height above the equatorial plane, radians. */
export const altitudeOf = (v: Vec3) => Math.asin(Math.max(-1, Math.min(1, v[1] / (Math.hypot(...v) || 1))));

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
    zero = (parent ? host : sun) ?? [0, 0, -1];
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
  const solarHours = !parent && locked ? null : Math.abs(rate) < STILL ? null : 1 / Math.abs(rate);
  const localHours = wrap(12 + (24 * (pointerLongitude - bearing(sky.sun))) / TAU, 24);

  return {
    state: {
      locked,
      zero,
      pointerLongitude,
      reading,
      sun,
      host: parent ? host : null,
      sunAltitude: altitudeOf(sky.sun),
      // The planet's own system: from a moon, its planet and the other
      // moons; from a planet, its moons. Other planets are left out.
      bodies: sky.bodies.flatMap((body, k) => {
        const id = sky.ids[k] ?? '';
        const other = system.bodies.find((b) => b.id === id);
        const local = parent ? id === parent.id || other?.parent === parent.id : other?.parent === self.id;
        return local
          ? [{ id, bearing: onEquator(body.direction), altitude: altitudeOf(body.direction), host: id === parent?.id }]
          : [];
      }),
    },
    figures: {
      siderealHours,
      solarHours,
      localHours,
      libration: locked ? libration(self.orbit, days) : null,
      librationAmplitude: locked ? librationAmplitude(self.orbit.eccentricity) : null,
      resonant: !locked && (self.spin.spin_orbit_3_2 ?? false),
    },
  };
}

const dot = (a: Vec3, b: Vec3) => a[0] * b[0] + a[1] * b[1] + a[2] * b[2];

/**
 * The host planet's compass as `moon` sees it: the host's own dial (its
 * ring graduated against its equinox or its star, the star's half-circle,
 * in the host's frame) with the moon as the only body on it, at its
 * bearing and height over the host's equator; null for a body without a
 * host. The host's sky is read only for its directions: no maps, coarse
 * orbit lines.
 */
export function hostCompass(
  system: System,
  moon: string,
  days: number,
): { state: DialState; figures: DialFigures } | null {
  const host = system.bodies.find((b) => b.id === moon)?.parent;
  if (!host) return null;
  const dial = rotationDial(system, host, days, 0, systemSky(system, host, days, () => null, 0, 8));
  if (!dial) return null;
  return { ...dial, state: { ...dial.state, bodies: dial.state.bodies.filter((b) => b.id === moon) } };
}

/** The unit words of a day length (`h`, `min`, `d` in English). */
export interface DayUnits {
  hour: string;
  minute: string;
  day: string;
}

/**
 * How a figure's numbers are written: the space between a number and its
 * unit, and a number with `digits` decimals (the locale's, see lib/format).
 */
export interface FigureFormat {
  space?: string;
  number?: (value: number, digits: number) => string;
}

/** `toFixed`, but a negative number rounded to zero prints 0, never "-0.0". */
const plainNumber = (value: number, digits: number) => value.toFixed(digits).replace(/^-(0(\.0*)?)$/, '$1');

/**
 * A local time in hours [0, 24) as `HH:MM`: rounded to the second, then
 * the minutes floored as a clock shows them, never `24:00`.
 */
export function formatLocalTime(hours: number): string {
  const minutes = Math.min(Math.floor(Math.round(wrap(hours, 24) * 3600) / 60), 24 * 60 - 1);
  const pad = (n: number) => String(n).padStart(2, '0');
  return `${pad(Math.floor(minutes / 60))}:${pad(minutes % 60)}`;
}

/**
 * The length of a day, `hours` long: under 48 h in hours and minutes to the
 * nearest minute (`10 h 53 min`, `53 min` under an hour), else in days
 * with two decimals (`2.49 d`).
 */
export function formatDayLength(hours: number, units: DayUnits, format: FigureFormat = {}): string {
  const { space = ' ', number = plainNumber } = format;
  const minutes = Math.round(hours * 60);
  if (minutes >= 48 * 60) return `${number(hours / 24, 2)}${space}${units.day}`;
  const h = Math.floor(minutes / 60);
  const m = `${minutes % 60}${space}${units.minute}`;
  return h > 0 ? `${h}${space}${units.hour} ${m}` : m;
}

/**
 * An angle in degrees, from radians: whole degrees in [0, 360) (`137°`),
 * or signed with one decimal (`+0.3°`, `−0.3°`) for a locked body's
 * reading and libration.
 */
export function formatDialAngle(radians: number, signed: boolean, number: (value: number, digits: number) => string = plainNumber): string {
  const degrees = (radians * 180) / Math.PI;
  if (!signed) return `${number(Math.round(wrap(degrees, 360)) % 360, 0)}°`;
  const tenths = Math.round(degrees * 10) / 10;
  return `${tenths < 0 ? '−' : '+'}${number(Math.abs(tenths), 1)}°`;
}

/** The parameters of the dial's two lines of figures, written out (see the `dial…` messages). */
export type DialFigureText =
  | { locked: false; resonant: boolean; angle: string; time: string; sidereal: string; solar: string | null }
  | { locked: true; libration: string; amplitude: string; time: string; solar: string | null };

/**
 * The dial's figures written out for the panel. A locked body's libration
 * is the pointer's swing round 0°, its reading, so that the panel and the
 * pointer's label agree in sign (the host swings the other way in its
 * sky: `figures.libration`).
 */
export function dialFigureText(
  dial: { state: DialState; figures: DialFigures },
  units: DayUnits,
  format: FigureFormat = {},
): DialFigureText {
  const { state, figures } = dial;
  const number = format.number ?? plainNumber;
  const length = (hours: number | null) => (hours === null ? null : formatDayLength(hours, units, format));
  const time = formatLocalTime(figures.localHours);
  if (state.locked) {
    const amplitude = ((figures.librationAmplitude ?? 0) * 180) / Math.PI;
    return {
      locked: true,
      libration: formatDialAngle(state.reading, true, number),
      amplitude: `${number(amplitude, 1)}°`,
      time,
      solar: length(figures.solarHours),
    };
  }
  return {
    locked: false,
    resonant: figures.resonant,
    angle: formatDialAngle(state.reading, false, number),
    time,
    sidereal: formatDayLength(figures.siderealHours, units, format),
    solar: length(figures.solarHours),
  };
}

