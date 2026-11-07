// The light of a night side, from the sources in the sky, and the display
// curve that maps it to a brightness on screen.
//
// Physics: a night side is lit by the light the other bodies of the system
// reflect (a host planet on its moon, a moon on its planet, a giant's rings),
// and by starlight and airglow. All are measured against the star's
// illuminance on a surface facing it at the body shown, so the result does not
// depend on the star: the light of a Lambert sphere of radius R and geometric
// albedo p, at distance D and phase angle alpha, is
//
//   E / E_star = p * Phi(alpha) * (R / D)^2 * insolation,
//   Phi(alpha) = (sin a + (pi - a) cos a) / pi            (Phi(0) = 1)
//
// (Russell 1916; the form used for planetary phase curves, e.g. in Seager,
// Exoplanet Atmospheres, 2010). `insolation` is the star's flux at that body
// over its flux at the body shown. Check against the full Moon seen from
// Earth: p = 0.12 (geometric albedo, NASA Moon fact sheet), R / D = 1737 /
// 384 400 = 4.5e-3, so 0.12 * 2.0e-5 = 2.4e-6 of the Sun's 1.3e5 lux, i.e.
// 0.3 lux, the top of the 0.1 to 0.3 lux usually given for the full Moon. The
// Earth seen from the Moon: p = 0.43 (NASA Earth fact sheet), R / D = 0.0166,
// 1.2e-4 at full Earth (the measured earthshine is within a factor 3 of it).
// A starlit, moonless clear night with airglow is 1e-3 to 2e-3 lux, about
// 1e-8 of daylight (the tables of typical illuminances, e.g. Allen's
// Astrophysical Quantities, or "Orders of magnitude (illuminance)").
//
// Display: those numbers span seven orders of magnitude, where the eye adapts
// (scotopic vision, Ferwerda et al. 1996, and the log-luminance tone mapping
// of Ward 1994 and Reinhard et al. 2002) and a screen shows a ratio of about
// 100 to 1. So the night side is shown on a logarithmic curve between two
// ratios, with a floor so that the shape of the body can always be seen. The
// curve's ends and the floor are display choices, not physics: they decide
// how bright a night looks, not how bright it is.
import type { SkyBody, Vec3 } from './skyBodies';

/** Illuminance ratio shown as the floor: below it nothing is seen (display choice). */
export const NIGHT_RATIO_MIN = 1e-7;
/** Illuminance ratio shown as the brightest night: a giant as seen from Io, about 1e-2 (display choice). */
export const NIGHT_RATIO_MAX = 2e-2;
/** Brightness of a night side lit by nothing: kept from before, a display choice, not physics. */
export const NIGHT_FLOOR = 0.07;
/** Brightness of a night side lit by a full giant, a display choice (the day side is 1): well under the day's, so the terminator stays obvious. */
export const NIGHT_CEILING = 0.18;

/** Starlight and airglow, relative to the Sun's illuminance on Earth: 2e-3 lux over 1.3e5 lux. */
export const STARLIGHT_RATIO = 1.5e-8;

/** Geometric albedo of rings, bright and icy (Saturn's B ring, 0.5 or so; the faint ones are darker). */
const RING_ALBEDO = 0.4;
/** Geometric albedo limits taken from a body's colour: a lump of coal to fresh ice. */
const ALBEDO_MIN = 0.05;
const ALBEDO_MAX = 0.8;

/** Most sources the surface shader takes. */
export const MAX_NIGHT_SOURCES = 4;

/** A light of the night: where it is and how much it gives. */
export interface NightSource {
  /** Unit direction, in the frame of the body shown. */
  direction: Vec3;
  /** Sine of its angular radius: a disk above the horizon, not a point. */
  half: number;
  /** Illuminance on a surface facing it, over the star's on one facing the star. */
  ratio: number;
}

/** Fraction of a Lambert sphere's light sent back at phase angle `alpha` (0 full, pi new), radians. */
export function lambertPhase(alpha: number): number {
  const a = Math.min(Math.max(alpha, 0), Math.PI);
  return (Math.sin(a) + (Math.PI - a) * Math.cos(a)) / Math.PI;
}

/** Geometric albedo from a surface colour (linear RGB): its luminance, a proxy kept within real bounds. */
export function albedoOf(colour: Vec3): number {
  const lum = 0.2126 * colour[0] + 0.7152 * colour[1] + 0.0722 * colour[2];
  return Math.min(Math.max(lum, ALBEDO_MIN), ALBEDO_MAX);
}

const dot = (a: Vec3, b: Vec3) => a[0] * b[0] + a[1] * b[1] + a[2] * b[2];

/**
 * The light one body of the sky gives, over the star's on the body shown.
 * Its phase comes from `phaseCos` when the system gave it (the angle at the
 * body between the star and the body shown), else from the sun's direction
 * `sun` as seen from the body shown (the star far behind the observer: full).
 * Its rings add their lit area, seen from the body shown, to its disk.
 */
export function reflectedRatio(body: SkyBody, sun: Vec3): number {
  const cos = body.phaseCos ?? -dot(body.direction, sun);
  const phase = lambertPhase(Math.acos(Math.min(Math.max(cos, -1), 1)));
  const half = Math.sin((body.angularDiameterDeg * Math.PI) / 360);
  let area = albedoOf(body.colour);
  if (body.rings) {
    const axis = body.axes?.[1] ?? [0, 1, 0];
    const { inner, outer, opacity } = body.rings;
    // Rings seen edge on give nothing; their area counts against the disk's pi R^2.
    area += RING_ALBEDO * (outer * outer - inner * inner) * opacity * Math.abs(dot(body.direction, axis));
  }
  return area * phase * half * half * (body.insolation ?? 1);
}

/**
 * The sources to light the night side with: the brightest bodies of the
 * sky, at most `MAX_NIGHT_SOURCES`, each as a direction, a size and a ratio.
 */
export function nightSources(bodies: readonly SkyBody[], sun: Vec3): NightSource[] {
  return bodies
    .map((body) => ({
      direction: body.direction,
      half: Math.sin((body.angularDiameterDeg * Math.PI) / 360),
      ratio: reflectedRatio(body, sun),
    }))
    .filter((s) => s.ratio > 0)
    .sort((a, b) => b.ratio - a.ratio)
    .slice(0, MAX_NIGHT_SOURCES);
}

/**
 * Brightness (the day side is 1) of a night-side point lit by `ratio` of the
 * star's illuminance: a log curve from `NIGHT_FLOOR` at `NIGHT_RATIO_MIN` to
 * `NIGHT_CEILING` at `NIGHT_RATIO_MAX`. The surface shader computes the same.
 */
export function nightBrightness(ratio: number): number {
  const lo = Math.log10(NIGHT_RATIO_MIN);
  const t = (Math.log10(Math.max(ratio, 1e-30)) - lo) / (Math.log10(NIGHT_RATIO_MAX) - lo);
  return NIGHT_FLOOR + (NIGHT_CEILING - NIGHT_FLOOR) * Math.min(Math.max(t, 0), 1);
}

/**
 * The night brightness at a point of the surface with outward normal `normal`.
 * The log curve sets the brightest the night gets, from the sum of the
 * sources' ratios (`peak`); the light at the point, each source's ratio by how
 * high it stands over the horizon there (a disk, so one just under the horizon
 * still gives a little) plus starlight, scales it linearly between the floor
 * and that. Putting the log on the peak only, not on each point, keeps the
 * edge of a source's reach soft: a log of the cosine law would drop from half
 * brightness to nothing within a degree of the horizon, a hard line on the
 * ground (a display choice).
 */
export function nightAt(normal: Vec3, sources: readonly NightSource[], starlight = STARLIGHT_RATIO): number {
  let peak = starlight;
  let total = starlight;
  for (const s of sources) {
    peak += s.ratio;
    total += s.ratio * Math.max((dot(normal, s.direction) + s.half) / (1 + s.half), 0);
  }
  return NIGHT_FLOOR + (nightBrightness(peak) - NIGHT_FLOOR) * (total / peak);
}
