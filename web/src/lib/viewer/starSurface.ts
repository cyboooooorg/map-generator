/**
 * The living surface of a star, shared by every view that draws one: the
 * system view's star (StarView), the star view and a planet's sky disc.
 *
 * - `starLook` turns a star and its `StarActivity` into what the shader
 *   needs (limb darkening, granulation, rotation).
 * - `STAR_SURFACE_GLSL` is a GLSL chunk with its uniforms and
 *   `vec3 starSurfaceColour(vec3 n, float mu)`; a material includes it,
 *   defines `STAR_REGIONS` (the length of the region arrays) and, for a
 *   disc too small to show granules, `STAR_NO_GRANULATION`. The
 *   granulation is moving cellular noise (`starCells`): bright cells,
 *   dark lanes between them, whose
 *   centres wander with the granule phase.
 * - `StarSurfaceState` keeps the uniforms in step with the clock: it asks
 *   the region timeline (lib/starActivity.ts) for the regions alive at a
 *   day and packs them into three small uniform arrays, again only when
 *   the set changes. The shader rebuilds each region's area and longitude
 *   from its packed birth, so between repacks only `uLocalDays` moves.
 *
 * Time: the shader sees only `uLocalDays`, days since the last repack
 * (under 21), and the granule phase reduced in JS: never an absolute day.
 * The phase wraps at 289 lifetimes, and every motion on it turns a whole
 * number of times per wrap (`granuleMotionHarmonics`), so the cells do
 * not jump there.
 * The surface at a day depends only on the activity (which carries the
 * seed), the day, and the nominal speed of the clock (the fades), never
 * on frame history or wall time.
 *
 * Cost: detail falls with the star's size on screen (`surfaceDetail`) and
 * with the clock's speed (`exposureFade`); the shader skips the region
 * loop and the granulation behind uniform branches when they are faded
 * out, so a small or fast-turning star costs only its limb darkening.
 * Where it is drawn, the granulation is the costliest term: two passes
 * over 20 cells (the second reuses the first's `rs[]`) and the warp's
 * 8 hashes, 28 hashes per pixel, one more in the lanes. The supergranule
 * network, drawn over the whole disc from afar, is kept much cheaper:
 * 8 cells and, from afar only, one octave of value noise (16 hashes);
 * up close, where the granules are drawn, the noise is skipped and the
 * 8 cells remain.
 *
 * The science (spot structure, facular contrast, limb law) is in
 * docs/science/star-activity.md.
 */
import { type IUniform, Vector2, Vector3, Vector4 } from 'three';
import type { StarActivity } from '../api/types/StarActivity';
import type { StarInfo } from '../api/types/StarInfo';
import {
  type ActiveRegion,
  hash32,
  MAX_LIFETIME_DAYS,
  rankRegions,
  regionNow,
  regionsBornIn,
  type RegionNow,
  rotationPhase,
} from '../starActivity';

const TWO_PI = 2 * Math.PI;

/** What the shader needs of a star, from its activity (or a fallback). */
export interface StarLook {
  /** The star's light, linear RGB (`info.light`). */
  colour: [number, number, number];
  /** Quadratic limb darkening `[u1, u2]`: `I(μ) = 1 − u1 (1 − μ) − u2 (1 − μ)²`. */
  limb: [number, number];
  /** Granule size, star radii. */
  granuleSize: number;
  /** Granule intensity contrast; 0 when radiative. */
  granuleContrast: number;
  granuleLifetimeDays: number;
  /** Equatorial angular velocity, rad/day. */
  omegaEq: number;
  /** Differential rotation: Ω(λ) = Ω_eq (1 − shear sin²λ). */
  shear: number;
  /** Facula area over spot area. */
  faculaRatio: number;
}

/** The Sun's granule size, 1.3 Mm over its 695.7 Mm radius (Trampedach et al. 2013). */
const SUN_GRANULE_SIZE = 1.3 / 695.7;
/** The Sun's granule lifetime, 8 minutes (Title et al. 1989). */
export const SUN_GRANULE_LIFETIME_DAYS = 8 / 1440;

/**
 * The look of `info`'s surface. Without an activity (a system file read
 * raw, from before activity existed), the old ad-hoc limb darkening
 * `[clamp(0.6 + (5772 − T)/5000, 0.3, 0.85), 0.2]`, the Sun's granules
 * (none above 7 500 K), a 25-day rotation without shear, and no regions.
 */
export function starLook(info: StarInfo, activity: StarActivity | null): StarLook {
  const colour: [number, number, number] = [info.light[0], info.light[1], info.light[2]];
  if (activity) {
    return {
      colour,
      limb: [activity.limb[0], activity.limb[1]],
      granuleSize: activity.granule_size,
      granuleContrast: activity.granule_contrast,
      granuleLifetimeDays: activity.granule_lifetime_days,
      omegaEq: TWO_PI / activity.rotation_days,
      shear: activity.shear,
      faculaRatio: activity.facula_ratio,
    };
  }
  const t = info.teff_k;
  return {
    colour,
    limb: [Math.min(0.85, Math.max(0.3, 0.6 + (5772 - t) / 5000)), 0.2],
    granuleSize: SUN_GRANULE_SIZE,
    granuleContrast: t > 7500 ? 0 : 0.12,
    granuleLifetimeDays: SUN_GRANULE_LIFETIME_DAYS,
    omegaEq: TWO_PI / 25,
    shear: 0,
    faculaRatio: 10,
  };
}

/**
 * How much of a feature that changes over `timescaleDays` survives a
 * frame lasting `daysPerFrame`: 1 when the frame is shorter, falling as
 * sqrt(timescale/step) beyond, as a long exposure averages it out (the
 * contrast of N independent looks averaged falls as 1/√N). Views pass
 * the nominal frame (days per second / 30), not the measured one, so
 * the surface stays a function of the clock alone.
 */
export function exposureFade(daysPerFrame: number, timescaleDays: number): number {
  if (!(daysPerFrame > timescaleDays)) return 1;
  if (!(timescaleDays > 0)) return 0;
  return Math.sqrt(timescaleDays / daysPerFrame);
}

const ramp = (x: number, lo: number, hi: number): number => Math.min(1, Math.max(0, (x - lo) / (hi - lo)));

/**
 * The Sun's supergranules, the cells of the chromospheric network, are
 * about 30 Mm across (Leighton, Noyes & Simon 1962, ApJ 135, 474;
 * Rieutord & Rincon 2010, Living Rev. Sol. Phys. 7, 2).
 */
export const SUPERGRANULE_SIZE_MM = 30;
/** The Sun's granules, 1.3 Mm (Trampedach et al. 2013), as `SUN_GRANULE_SIZE`. */
const SUN_GRANULE_SIZE_MM = 1.3;
/**
 * A supergranule lives about a day (Hart 1956, MNRAS 116, 38; Rieutord &
 * Rincon 2010, Living Rev. Sol. Phys. 7, 2): the network's own clock.
 */
export const SUPERGRANULE_LIFETIME_DAYS = 1;
/**
 * **Calibration**: the network's brightening at the Sun's
 * `granule_contrast` of 0.15, a little stronger than the real network
 * (prototype D's thin lanes drew 0.07; the soft ribbons need 0.15 to be
 * seen at the disc centre from the star view's opening distance): the
 * disc centre about 2 % brighter on average with a 2.3 % rms, the limb
 * about 5 % brighter with a 5 % rms (the figures after the limb cap, as in
 * the science doc). Times `min(1, granule_contrast / 0.15)`: 0 on a radiative
 * star.
 */
const NETWORK_CONTRAST = 0.15;
/**
 * **Calibration**: the half-width of the network's ribbons in F2 − F1
 * (cells), before the junctions widen them and the mottle varies them:
 * soft and wide, a picture of the network's faculae seen from afar
 * rather than thin cell outlines.
 */
const NETWORK_LANE_WIDTH = 0.6;
/** The plages' brightening at the limb, 0.25 (1 − μ) at most (see STAR_SURFACE_GLSL). */
const PLAGE_LIMB_CONTRAST = 0.25;
/**
 * **Calibration**: a network ribbon's peak is at most this share of a
 * full plage's at the same μ (away from the disc centre), so that the
 * active regions' faculae stay the brightest features at the limb.
 */
const NETWORK_PLAGE_SHARE = 0.8;
/** The Sun's granule contrast, against which the network's is scaled. */
const SUN_GRANULE_CONTRAST = 0.15;

/**
 * The network's cell size in star radii, for granules of `granuleSize`:
 * supergranules scale with the granules, `SUPERGRANULE_SIZE_MM` over the
 * Sun's 1.3 Mm granules (both are set by the convection's scale; a
 * **calibration** for other stars, as no survey measures their network).
 */
export function networkSize(granuleSize: number): number {
  return (granuleSize * SUPERGRANULE_SIZE_MM) / SUN_GRANULE_SIZE_MM;
}

/** The network's contrast for a granule contrast: `min(1, c / 0.15) · 0.15`, 0 when radiative. */
export function networkContrast(granuleContrast: number): number {
  if (!(granuleContrast > 0)) return 0;
  return Math.min(1, granuleContrast / SUN_GRANULE_CONTRAST) * NETWORK_CONTRAST;
}

/**
 * How much surface detail a star `radiusPx` pixels in radius shows:
 * granulation fades in from 2.5 to 5 pixels per granule (`granuleSize`
 * in star radii) and is off below, where its cells could not be told
 * apart and would cost the most for nothing; the active regions fade in from 6 to
 * 12 pixels of star radius and are off below; the supergranule network
 * (cells `networkSize(granuleSize)` across) fades in from 3 to 8 pixels
 * per cell, so that it shows on the whole disc of the star view (about
 * 20 px per cell for the Sun 500 px in radius) and is off on a small star.
 */
export function surfaceDetail(
  radiusPx: number,
  granuleSize: number,
): { granulation: number; regions: number; network: number } {
  return {
    granulation: ramp(radiusPx * granuleSize, 2.5, 5),
    regions: ramp(radiusPx, 6, 12),
    network: ramp(radiusPx * networkSize(granuleSize), 3, 8),
  };
}

/**
 * Period of the granule pattern's phase, in granule lifetimes: the
 * phase is reduced modulo 289 in JS, so every motion the shader drives
 * with it turns a whole number of times per period (see
 * `granuleMotionHarmonics`) and the pattern does not jump at the wrap.
 */
const GRANULE_PHASE_PERIOD = 289;

/**
 * The cells' wander, as whole turns per `GRANULE_PHASE_PERIOD`: the
 * centres move along x, y and z at 97, 103 and 109 turns (2.1 to 2.4
 * rad per lifetime, as prototype D's 2.4; **calibration**), the domain
 * warp drifts round a circle at 2 turns.
 */
const CELL_HARMONICS = [97, 103, 109] as const;
const WARP_HARMONIC = 2;

/**
 * Every angular rate the shader puts on the granule phase is
 * 2π·k/`GRANULE_PHASE_PERIOD` for one of these integers k (the chunk's
 * rates are written from them), so the surface is periodic in the
 * phase's wrap: the three axes of the cells' wander, then the warp's
 * drift.
 */
export function granuleMotionHarmonics(): readonly number[] {
  return [...CELL_HARMONICS, WARP_HARMONIC];
}

/** The angular rate of harmonic k on the granule phase, rad per lifetime, as a GLSL literal. */
const glslRate = (k: number): string => ((TWO_PI * k) / GRANULE_PHASE_PERIOD).toFixed(8);

/**
 * Prototype D's warp drifted 0.08 noise units per lifetime; on a circle
 * turning at the warp's rate, that is its radius.
 */
const WARP_RADIUS = 0.08 / ((TWO_PI * WARP_HARMONIC) / GRANULE_PHASE_PERIOD);

/**
 * Mean and rms of the raw granule brightness up close,
 * measured on disc patches from the centre to 0.6 radii and at several
 * phases (see `starGranules`): they bring it to zero mean and unit rms.
 */
const GRANULE_RAW_MEAN = 0.519;
const GRANULE_RAW_RMS = 0.384;

/**
 * **Calibration**: granules this many pixels across or fewer are shaded
 * as small (a continuous sum of round blobs of more varied tone, see
 * starCellBlobs), and from the second as close up: in between, at
 * 1.1 R, thin crisp lanes round flat cells read as cracked glass, and
 * any drawn edge as a seam across the blobs.
 */
const GRANULE_SMALL_PX = [40, 120] as const;
/** The raw brightness's mean and rms with the small shading, measured as GRANULE_RAW_MEAN. */
const GRANULE_SMALL_RAW_MEAN = 1.323;
const GRANULE_SMALL_RAW_RMS = 0.545;

/**
 * **Calibration**: the drawn granulation's rms over `granule_contrast`,
 * on the brightness b before the soft exposure curve.
 * Prototype D, the approved look, drew 0.62 × the raw rms (about 0.236)
 * on the Sun, whose `granule_contrast` is 0.15: a gain of about 1.6.
 */
const GRANULE_GAIN = 1.6;

/**
 * **Calibration**: the shaped spots' scale, which keeps the area they draw
 * darker than 0.9 of the quiet photosphere within 20 % of a round spot's
 * of the same radius (measured in a harness at ages 0.1 to 0.9, see
 * docs/science/star-activity.md): their torn edges and pores reshape the
 * area, they do not add to it.
 */
const SPOT_SCALE = 1.02;
/**
 * The Evershed flow, about 1–2 km/s outward along the penumbral filaments
 * (Evershed 1909, MNRAS 69, 454): 1.5 km/s on a reference spot of 10 Mm
 * radius (**calibration**) is 1.5 · 86 400 / 10 000 = 13 spot radii a day,
 * three noise units per radius along the filaments.
 */
const EVERSHED_RATE = (3 * 1.5 * 86400) / 10000;
/**
 * The umbral dots twinkle 72 times a day, a period of 20 minutes: dots
 * live minutes to about an hour (Borrero & Ichimoto 2011, Living Rev.
 * Sol. Phys. 8, 4; the period is a **calibration**).
 */
const UMBRAL_DOT_RATE = 1440 / 20;
/**
 * The days over which the spots' moving details change visibly: a quarter
 * of a noise unit of the filaments' drift or of a dot's twinkle, whichever
 * is faster (5 minutes). A clock outrunning it fades them
 * (`exposureFade`), as a long exposure averages them out.
 */
const SPOT_MOTION_DAYS = 0.25 / Math.max(EVERSHED_RATE, UMBRAL_DOT_RATE);
/** **Calibration**: the penumbra's mean brightness (Solanki 2003 gives about 0.75). */
const SPOT_PENUMBRA = 0.77;
/**
 * **Calibration**: the umbra's mean radius in the shape's units, a little
 * above sqrt(1/5) = 0.447 (Solanki 2003's penumbra four times the umbra),
 * so that the dark area on screen matches part 1's round umbra.
 */
const SPOT_UMBRA_RADIUS = 0.47;
/** **Calibration**: the share of the (large) spots that grow a light bridge in mid-life. */
const SPOT_BRIDGE_SHARE = 0.35;
/** **Calibration**: the share of a spot opened into gaps at its death; its pieces spread to keep its area. */
const SPOT_FRAGMENT_LOST = 0.35;
/**
 * **Calibration**: a grown spot's scale is trimmed by this share, as its
 * torn, lobed outline covers more than a round one of the same mean
 * radius (measured with the area, see SPOT_SCALE).
 */
const SPOT_GROWN_TRIM = 0.03;
/** **Calibration**: the share of a spot's area its moat's pores darken (measured with the area, see SPOT_SCALE). */
const SPOT_PORE_SHARE = 0.05;
/**
 * The breakup noise, the mean of starFbm2 (sd 0.138) and one octave of
 * value noise (sd 0.185), is close to normal with mean 0.5 and sd 0.115:
 * a share p of it lies below 0.5 + 0.115 · probit(p), within 0.003 of
 * the quantiles measured on 500 000 samples for p from 0.02 to 0.35.
 */
const SPOT_BREAKUP_MEDIAN = 0.5;
const SPOT_BREAKUP_SD = 0.115;

/** About where the granule clock's days wrap before the phase is computed (keeps it exact in f64). */
const GRANULE_DAYS_WRAP = 1e4;

/**
 * The days at which the granule clock wraps for granules living
 * `lifetimeDays`: the whole number of phase periods (289 lifetimes)
 * nearest `GRANULE_DAYS_WRAP`, at least one, so that the phase runs on
 * across the wrap without a jump.
 */
export function granuleDaysWrap(lifetimeDays: number): number {
  const period = GRANULE_PHASE_PERIOD * lifetimeDays;
  return period * Math.max(1, Math.round(GRANULE_DAYS_WRAP / period));
}

/**
 * The granule phase at `days`, in lifetimes, reduced modulo
 * `GRANULE_PHASE_PERIOD`: continuous in `days` (the day clock wraps at
 * `granuleDaysWrap`, a whole number of periods), and 0 without
 * granule lifetime.
 */
export function granulePhase(days: number, lifetimeDays: number): number {
  if (!(lifetimeDays > 0)) return 0;
  return modulo(modulo(days, granuleDaysWrap(lifetimeDays)) / lifetimeDays, GRANULE_PHASE_PERIOD);
}

/**
 * The network's phase at `days`, in supergranule lifetimes, reduced as
 * the granule phase (`granulePhase`): modulo 289 lifetimes, its day clock
 * wrapped at a whole number of them, so that the network's cells, which
 * move on the same harmonics, do not jump at either wrap.
 */
export function networkPhase(days: number): number {
  return granulePhase(days, SUPERGRANULE_LIFETIME_DAYS);
}

/** The key mixed with a region's id for its spots' seed (`hash32(id, 7)`). */
const SPOT_SEED_KEY = 7;

/** Days from the epoch after which the regions are packed again, to keep `uLocalDays` small. */
const EPOCH_SPAN_DAYS = 20;
/**
 * Half-width of the window of days the cached candidate regions cover:
 * every region alive at some day within it. The timeline is drawn again
 * only when the clock leaves the window.
 */
const WINDOW_DAYS = 21;

/**
 * The surface's GLSL: its uniforms; the noise helpers, compiled even
 * when `STAR_NO_GRANULATION` is defined, so that a material without
 * granules can still draw cells (`uint starHashU(vec3 c)`, `float
 * starHash(vec3 p)`, `vec3 starHash3(vec3 c)`, `vec3
 * starValueNoise3(vec3 x)`, `vec3 starFbm2x3(vec3 p)`, `float
 * starFbm2(vec3 p)`, `vec3 starWave(vec3 x)`, `vec3
 * starValueNoiseRing(vec2 x, float period, float salt)`, `StarCell
 * starCellDetail(vec3 p, float phase, float blobs)`, `float
 * starCellBlobs(vec3 p, float phase)`, `vec2 starCells(vec3 p, float
 * phase)` and `float starCellLanes(vec3 p, float phase)`); `float
 * starSpotShaped(vec2 local, float rs, float seed, float age, float
 * aa)`, the brightness of one spot; `float starLimb(float mu)`, `float starBrightness(vec3 n, float mu)`, `vec3
 * starSurfaceExposed(vec3 n, float mu)` (for display, see there),
 * `float starDither(vec2)` and `vec3 starSurfaceColour(vec3 n, float
 * mu)`, the linear colour of the photosphere at unit normal `n` in the
 * star's inertial frame (y the spin axis, longitude increasing
 * anticlockwise seen from +y, as the orbits go) and `mu` the cosine of
 * the angle to the line of sight.
 *
 * The photosphere's brightness is the quadratic limb law times the
 * granulation, the network, the spots and the faculae, times
 * `uStarColour`:
 *
 * - Granulation (`starGranules`, prototype D): cellular noise at the
 *   granule size, warped by one octave of value noise that curves the
 *   cells' edges. Each cell is a dome of its own tone, bright at its
 *   centre and falling towards its edge (distances along the surface,
 *   see `starCellDetail`), dimmer while it is born or dies, a few large
 *   ones darker in the middle, its corners rounded; the intergranular
 *   lanes are dark, narrow along long edges and wider at junctions.
 *   Granules are the tops of convective cells: hot upflows, bright,
 *   ringed by cooler, darker downflow lanes (Nordlund, Stein & Asplund
 *   2009, Living Rev. Sol. Phys. 6, 2). The magnetic bright points of
 *   the lanes (Sánchez Almeida et al. 2004, ApJ 609, L91) are not drawn:
 *   cut by the lane they sat in, they read as specks. The cells' centres wander by 0.3 of a cell at 2.1 to 2.4 rad
 *   per lifetime (**calibration**), so cells deform, split and merge over
 *   a few lifetimes. It is brought to zero mean and unit rms, then times
 *   `GRANULE_GAIN` (**calibration**, prototype D's look): the rms of the
 *   brightness b is 1.6 × `granule_contrast`, measured before the soft
 *   exposure curve (`starSurfaceExposed`), which compresses it on
 *   screen. `starGranules` is compiled out under
 *   `STAR_NO_GRANULATION`; the noise helpers are not.
 * - The supergranule network (`uNetwork`): cells of `networkSize` (30 Mm
 *   on the Sun, Leighton, Noyes & Simon 1962) on their own phase, in
 *   supergranule lifetimes (a day, Hart 1956), whose lanes
 *   (`starCellLanes`) are drawn as soft ribbons, `NETWORK_LANE_WIDTH` in
 *   F2 − F1 and wider and brighter at junctions, of a tone varying from
 *   lane to lane, brightened by c (0.35 + 1.4 (1 − μ)), faculae
 *   brightest at the limb (Solanki 2003), capped so that a ribbon's
 *   peak stays at 0.8 of a plage's at the same μ. From afar a mesogranular
 *   mottle (one octave of value noise at 0.35 of the cell size) adds
 *   ±c/4 and varies the ribbons' width and tone along their length; it
 *   fades out as the granules fade in (1 − `uGranule.w`), behind a
 *   uniform branch. c is `networkContrast`, 0.15 on the Sun
 *   (**calibration**, slightly above the real network so that the whole
 *   disc is not smooth) and 0 on a radiative star. The network is
 *   sampled in the star's turning frame: n's longitude minus the
 *   equatorial rotation phase (`uNetworkSpin`, `rotationPhase` reduced
 *   in JS), with no shear over its day of life. Not compiled out by
 *   `STAR_NO_GRANULATION`: the sky disc keeps it off with its fade
 *   (`hostStarDetail`).
 * - Each region: a leader (0.6 of the area, **calibration**) and a
 *   follower spot, at ±separation/2 along the region's tilt axis (the
 *   leader east and equatorward, Joy's law), each a shaped spot
 *   (`starSpotShaped`): an umbra within a penumbra four times its area
 *   (Solanki 2003, A&ARv 11, 153), with filaments drifting outward (the
 *   Evershed flow, Evershed 1909, MNRAS 69, 454), umbral dots, a light
 *   bridge on some, a torn outline that changes with the region's age,
 *   fragments in its decline and pores in its moat, its dark area within
 *   20 % of a round spot's. No granules are drawn (nor computed) inside
 *   it. Around them a facular
 *   plage of 3× the region's spot radius, brightened by 0.25 (1 − μ)
 *   (**calibration**, brightest at the limb, Solanki 2003), with a fill
 *   of facula_ratio / 9 (the plage covers 9 spot areas; the Sun's ratio
 *   about 10 fills it), at most 1.
 *
 * Region arrays: `uRegionA[i] = (latitude, longitude at the epoch, tilt,
 * separation)`, `uRegionB[i] = (born − epoch, growth, lifetime,
 * peakArea)`, radians, days and shares of the hemisphere, and
 * `uRegionC[i] = (seed, 0, 0, 0)`, the seed `hash32(id, 7) / 2^32` in
 * [0, 1) that shapes its spots. The area is
 * rebuilt as a triangle (linear rise over `growth`, linear fall to 0 at
 * `lifetime`) and the longitude as `+ Ω_eq (1 − shear sin²λ) uLocalDays`.
 */
export const STAR_SURFACE_GLSL = /* glsl */ `
#ifndef STAR_REGIONS
#error STAR_REGIONS must be defined by the material (see starSurfaceUniforms)
#endif
uniform vec3 uStarColour;
uniform vec2 uLimb;
/** Granules: size (star radii), contrast, phase (lifetimes, mod 289), fade. */
uniform vec4 uGranule;
/** A granule's size on screen, pixels (the star's radius in pixels times the granule size): its shading follows it. */
uniform float uGranulePx;
/** The supergranule network: cell size (star radii), contrast, phase (lifetimes, mod 289), fade. */
uniform vec4 uNetwork;
/** The star's equatorial rotation phase (rad, in [0, 2π)): the network turns with it. */
uniform float uNetworkSpin;
uniform float uRegionFade;
uniform int uRegionCount;
uniform vec4 uRegionA[STAR_REGIONS];
uniform vec4 uRegionB[STAR_REGIONS];
/** Per region: (seed in [0, 1), 0, 0, 0); the seed shapes its spots. */
uniform vec4 uRegionC[STAR_REGIONS];
/** The spots' moving details (filaments, umbral dots): 1, fading as the clock outruns them. */
uniform float uSpotMotion;
/** Days since the regions were packed. */
uniform float uLocalDays;
uniform float uOmegaEq;
uniform float uShear;
uniform float uFacula;

/** Angular rates of the cells' wander along x, y, z on a phase, rad per lifetime: 2π·k/289. */
const vec3 STAR_CELL_RATES = vec3(${CELL_HARMONICS.map(glslRate).join(', ')});
/** The granule warp's drift round a circle: its rate (2π·k/289) and radius, in noise units. */
const float STAR_WARP_RATE = ${glslRate(WARP_HARMONIC)};
const float STAR_WARP_RADIUS = ${WARP_RADIUS.toFixed(6)};
/** The network's ribbons: half-width in F2 − F1 (cells) and the junction's reach (**calibrations**, NETWORK_LANE_WIDTH). */
const float STAR_NETWORK_WIDTH = ${NETWORK_LANE_WIDTH.toFixed(4)};
const float STAR_NETWORK_JUNCTION = 0.6;
/** The network's limb slope at most: its peak, c times this times (1 − μ), at most NETWORK_PLAGE_SHARE of a plage's 0.25 (1 − μ). */
const float STAR_NETWORK_LIMB_CAP = ${((NETWORK_PLAGE_SHARE * PLAGE_LIMB_CONTRAST) / NETWORK_CONTRAST).toFixed(4)};
/** The warp's frequency (per cell) and amplitude (cells, **calibration**): enough to curve the cells' edges. */
const float STAR_WARP_SCALE = 0.3;
const float STAR_WARP_AMPLITUDE = 1.0;
/** The spots' shape: the scale that keeps their dark area a round spot's, and the drifts of the filaments and umbral dots, noise units per day (SPOT_SCALE, EVERSHED_RATE, UMBRAL_DOT_RATE). */
const float STAR_SPOT_SCALE = ${SPOT_SCALE.toFixed(4)};
const float STAR_EVERSHED_RATE = ${EVERSHED_RATE.toFixed(4)};
const float STAR_UMBRAL_DOT_RATE = ${UMBRAL_DOT_RATE.toFixed(4)};

/**
 * A 32-bit hash of the integer point c (lowbias32, Wellons 2018, of the
 * coordinates' weighted sum). The offset keeps the coordinates positive
 * for |c| < 32768, i.e. cells larger than 3e-5 star radii.
 */
uint starHashU(vec3 c) {
  uvec3 v = uvec3(ivec3(c) + 32768);
  uint h = v.x * 0x8da6b343u + v.y * 0xd8163841u + v.z * 0xcb1ab31fu;
  h ^= h >> 16u;
  h *= 0x7feb352du;
  h ^= h >> 15u;
  h *= 0x846ca68bu;
  h ^= h >> 16u;
  return h;
}

/** A value in [0, 1) for the unit cell holding p. */
float starHash(vec3 p) {
  return float(starHashU(floor(p)) >> 8u) * (1.0 / 16777216.0);
}

/** Three values in [0, 1) for the integer point c, ten bits each. */
vec3 starHash3(vec3 c) {
  uint h = starHashU(c);
  return vec3(uvec3(h, h >> 10u, h >> 20u) & 1023u) * (1.0 / 1024.0);
}

/** Three channels of value noise in [0, 1], one hash per lattice corner. */
vec3 starValueNoise3(vec3 x) {
  vec3 i = floor(x);
  vec3 f = x - i;
  f = f * f * (3.0 - 2.0 * f);
  return mix(
    mix(mix(starHash3(i), starHash3(i + vec3(1.0, 0.0, 0.0)), f.x),
        mix(starHash3(i + vec3(0.0, 1.0, 0.0)), starHash3(i + vec3(1.0, 1.0, 0.0)), f.x), f.y),
    mix(mix(starHash3(i + vec3(0.0, 0.0, 1.0)), starHash3(i + vec3(1.0, 0.0, 1.0)), f.x),
        mix(starHash3(i + vec3(0.0, 1.0, 1.0)), starHash3(i + vec3(1.0, 1.0, 1.0)), f.x), f.y),
    f.z);
}

/** Two octaves of value noise, three channels in [0, 1] about 0.5 (prototype D's cheap fbm). */
vec3 starFbm2x3(vec3 p) {
  return (0.5 * starValueNoise3(p) + 0.25 * starValueNoise3(p * 2.03 + 1.7)) * (4.0 / 3.0);
}

/** One channel of starFbm2x3. */
float starFbm2(vec3 p) {
  return starFbm2x3(p).x;
}

/**
 * Three channels of 2D value noise on a ring: x wraps every \`period\`
 * cells (a whole number), so that noise taken at (period · turns, ·) has
 * no seam where the angle wraps. \`salt\` (a whole number) picks the
 * pattern. One hash per lattice corner, four in all.
 */
vec3 starValueNoiseRing(vec2 x, float period, float salt) {
  vec2 i = floor(x);
  vec2 f = x - i;
  f = f * f * (3.0 - 2.0 * f);
  float i0 = mod(i.x, period);
  float i1 = mod(i.x + 1.0, period);
  return mix(mix(starHash3(vec3(i0, i.y, salt)), starHash3(vec3(i1, i.y, salt)), f.x),
             mix(starHash3(vec3(i0, i.y + 1.0, salt)), starHash3(vec3(i1, i.y + 1.0, salt)), f.x), f.y);
}

/** sin(2π x) within 0.056, a parabola per half turn: smooth, and cheaper than sin. */
vec3 starWave(vec3 x) {
  vec3 f = 1.0 - 2.0 * fract(x);
  return 4.0 * f * (1.0 - abs(f));
}

/**
 * The 20 cells searched round p's cell, as steps towards p's side of
 * its cell on each axis (1), none (0) or away (−1): the 27 neighbours
 * but the 7 away on two or three axes.
 */
const vec3 STAR_CELL_STEPS[20] = vec3[20](
  vec3(0.0, 0.0, 0.0), vec3(1.0, 0.0, 0.0), vec3(0.0, 1.0, 0.0), vec3(0.0, 0.0, 1.0),
  vec3(1.0, 1.0, 0.0), vec3(1.0, 0.0, 1.0), vec3(0.0, 1.0, 1.0), vec3(1.0, 1.0, 1.0),
  vec3(-1.0, 0.0, 0.0), vec3(-1.0, 1.0, 0.0), vec3(-1.0, 0.0, 1.0), vec3(-1.0, 1.0, 1.0),
  vec3(0.0, -1.0, 0.0), vec3(1.0, -1.0, 0.0), vec3(0.0, -1.0, 1.0), vec3(1.0, -1.0, 1.0),
  vec3(0.0, 0.0, -1.0), vec3(1.0, 0.0, -1.0), vec3(0.0, 1.0, -1.0), vec3(1.0, 1.0, -1.0));


/**
 * The small granules' blobs: exp(−d² · this), a Gaussian of
 * σ = 1 / sqrt(2 · 5.5) ≈ 0.30 cell, depth counting for 0.4 of its square
 * (**calibrations**).
 */
const float STAR_BLOB_SHARPNESS = 5.5;

/** What starCellDetail finds at a point, distances in cells. */
struct StarCell {
  /** Distance to the nearest centre (3D). */
  float f1;
  /** Distance along the surface to the cell's nearest edge, and to its second-nearest edge line. */
  float edge;
  float edge2;
  /** Distance along the surface to the cell's centre, and the centre's depth below or above the surface. */
  float centre;
  float depth;
  /** A value in [0, 1) for the cell. */
  float id;
  /** starCellBlobs's sum when asked for (blobs > 0), else 0. */
  float blobs;
};

/**
 * Cellular noise on the surface through p (the sphere of radius |p|,
 * one cell centre per unit cube). Each centre wanders about its cube's
 * middle by 0.3 · starWave(rate · phase / 2π + h) per axis, h the
 * cube's hash and rate STAR_CELL_RATES, so the cells deform, split and
 * merge as the phase runs, and come back the same after 289.
 *
 * The edges are measured along the surface, in the tangent plane: the
 * distance to the bisector with centre k is (d_k − d_1) / (2 |t_k −
 * t_1|), t the centres projected on the plane. The slice of a Voronoi
 * diagram by a plane is a power diagram in it, and a cell is convex, so
 * the nearest of these is the exact, continuous distance to the cell's
 * edge. Plain 3D Worley F2 − F1 stays small wherever the sphere runs
 * nearly parallel to a face between two centres stacked in depth, and
 * drew flat grey patches with hard, axis-aligned edges.
 *
 * edge2, the distance to the second-nearest edge line, is small near
 * a junction of three cells; it may differ on the two sides of an edge,
 * where edge is 0.
 *
 * It takes two passes over the 20 cubes of STAR_CELL_STEPS: the nearest
 * centre, then the nearest edges. With the wander bounded by 0.3 the
 * result matched a 125-cube search on every one of 4.3 million test
 * points. A 27-cube search, on an M1, cost two to four times as much.
 */
StarCell starCellDetail(vec3 p, float phase, float blobs) {
  vec3 i = floor(p);
  vec3 f = p - i;
  vec3 axis = normalize(p);
  vec3 side = sign(f - 0.5);
  side += 1.0 - abs(side);
  vec3 turns = fract(STAR_CELL_RATES * (phase * 0.15915494));
  // The centres are kept from the first pass for the second.
  vec3 rs[20];
  float d1 = 1e4;
  vec3 r1 = vec3(0.0);
  float h1 = 0.0;
  float sum = 0.0;
  for (int k = 0; k < 20; k++) {
    vec3 g = side * STAR_CELL_STEPS[k];
    vec3 hv = starHash3(i + g);
    vec3 r = g + 0.5 + 0.3 * starWave(turns + hv) - f;
    rs[k] = r;
    float d = dot(r, r);
    // Callers pass blobs as a literal, so the compiler drops this when 0.
    if (blobs > 0.0) {
      float depth = dot(r, axis);
      sum += (0.45 + fract(hv.x * 13.7)) * exp(-(d - 0.6 * depth * depth) * blobs);
    }
    bool nearer = d < d1;
    d1 = nearer ? d : d1;
    r1 = nearer ? r : r1;
    h1 = nearer ? hv.x : h1;
  }
  float edge = 1e4;
  float edge2 = 1e4;
  for (int k = 0; k < 20; k++) {
    vec3 r = rs[k];
    vec3 dr = r - r1;
    float along = dot(dr, axis);
    float apart = dot(dr, dr) - along * along;
    // The nearest centre itself (apart 0) is no edge.
    float e = apart > 1e-8 ? 0.5 * (dot(r, r) - d1) * inversesqrt(apart) : 1e4;
    bool nearer = e < edge;
    edge2 = min(edge2, max(edge, e));
    edge = nearer ? e : edge;
  }
  StarCell cell;
  cell.f1 = sqrt(d1);
  cell.edge = edge;
  cell.edge2 = edge2;
  float depth = dot(r1, axis);
  cell.depth = abs(depth);
  cell.centre = length(r1 - depth * axis);
  cell.id = h1;
  cell.blobs = sum;
  return cell;
}

/**
 * (F1, F2) of starCellDetail, F2 = F1 + 2 · edge: F2 − F1 is 0 on a
 * cell's edge and grows into the cell, as the lane measure of a 2D cell
 * pattern.
 */
vec2 starCells(vec3 p, float phase) {
  StarCell cell = starCellDetail(p, phase, 0.0);
  return vec2(cell.f1, cell.f1 + 2.0 * cell.edge);
}

/**
 * The network's lanes, drawn over the whole disc: (F2 − F1, the same
 * measure for the second-nearest edge line, a value in [0, 1) for the
 * pair of cells across the nearest edge). F2 − F1 = 2 · edge as
 * starCells measures it (the same cells, wandering the same way, the
 * edges along the surface), but searched among the 8 cells of the
 * 2×2×2 block on p's side only, in one pass whose centres are kept for
 * the edges: about a third of starCells' cost. Against the 20-cell
 * search it misses a short stub of lane near a junction on about 0.1 %
 * of the points, which the network's soft, wide ribbons hide.
 */
vec3 starCellLanes(vec3 p, float phase) {
  vec3 i = floor(p);
  vec3 f = p - i;
  vec3 axis = normalize(p);
  vec3 side = sign(f - 0.5);
  side += 1.0 - abs(side);
  vec3 turns = fract(STAR_CELL_RATES * (phase * 0.15915494));
  vec3 rs[8];
  float hs[8];
  float d1 = 1e4;
  vec3 r1 = vec3(0.0);
  float h1 = 0.0;
  // STAR_CELL_STEPS starts with the block's 8 cells.
  for (int k = 0; k < 8; k++) {
    vec3 g = side * STAR_CELL_STEPS[k];
    vec3 hv = starHash3(i + g);
    vec3 r = g + 0.5 + 0.3 * starWave(turns + hv) - f;
    rs[k] = r;
    hs[k] = hv.x;
    float d = dot(r, r);
    bool nearer = d < d1;
    d1 = nearer ? d : d1;
    r1 = nearer ? r : r1;
    h1 = nearer ? hv.x : h1;
  }
  float edge = 1e4;
  float edge2 = 1e4;
  float across = 0.0;
  for (int k = 0; k < 8; k++) {
    vec3 r = rs[k];
    vec3 dr = r - r1;
    float along = dot(dr, axis);
    float apart = dot(dr, dr) - along * along;
    float e = apart > 1e-8 ? 0.5 * (dot(r, r) - d1) * inversesqrt(apart) : 1e4;
    bool nearer = e < edge;
    edge2 = min(edge2, max(edge, e));
    edge = nearer ? e : edge;
    across = nearer ? hs[k] : across;
  }
  return vec3(2.0 * edge, 2.0 * edge2, fract(h1 + across));
}

/**
 * The cells of starCellDetail as a sum of round blobs: over the same 20
 * cells, each centre's tone (0.45 to 1.45, from its value) times
 * exp(−d² · STAR_BLOB_SHARPNESS), d² the squared distance along the
 * surface plus 0.4 of the squared depth (a centre deep below or above
 * the surface is dimmer). It is continuous everywhere: no edge is drawn,
 * and the dips between centres are the lanes, wide where centres are far
 * apart. One pass, for granules too small on screen for their edges.
 */
float starCellBlobs(vec3 p, float phase) {
  vec3 i = floor(p);
  vec3 f = p - i;
  vec3 axis = normalize(p);
  vec3 side = sign(f - 0.5);
  side += 1.0 - abs(side);
  vec3 turns = fract(STAR_CELL_RATES * (phase * 0.15915494));
  float sum = 0.0;
  for (int k = 0; k < 20; k++) {
    vec3 g = side * STAR_CELL_STEPS[k];
    vec3 hv = starHash3(i + g);
    vec3 r = g + 0.5 + 0.3 * starWave(turns + hv) - f;
    float along = dot(r, axis);
    sum += (0.45 + fract(hv.x * 13.7)) * exp(-(dot(r, r) - 0.6 * along * along) * STAR_BLOB_SHARPNESS);
  }
  return sum;
}

#ifndef STAR_NO_GRANULATION
/**
 * Granulation at n, zero-mean with rms ${GRANULE_GAIN} (the gain, see
 * GRANULE_GAIN): cells of the granule size in a space warped by one
 * octave of value noise (which curves their edges), shaded as
 * prototype D's look and real granules:
 *
 * - each cell a dome, bright at its centre and falling towards its edge
 *   with s = centre / (centre + edge), its tone varying from cell to
 *   cell, dimmer while its centre lies deep (a granule being born or
 *   dying), and a darker middle in a few large cells (exploding
 *   granules);
 * - its corners rounded by a faster fall where s nears 1;
 * - dark intergranular lanes with soft edges, narrow along long edges
 *   and widening into pools at junctions.
 *
 * That is the look from 120 px per granule (uGranulePx). At 40 px or
 * fewer, the cells are a continuous sum of round blobs (starCellBlobs),
 * with no edge drawn, so no seam crosses them; in between the two
 * blend, every lane edge at least 2.5 px soft (GRANULE_SMALL_PX), the
 * blobs summed in the same pass as the cells.
 *
 * The shapes and weights are **calibrations** matched by eye to
 * prototype D.
 */
float starGranules(vec3 n) {
  float phase = uGranule.z;
  vec3 q = n / uGranule.x;
  float drift = STAR_WARP_RATE * phase;
  // One octave of warp, as D's second octave cost a sixth of the term.
  q += STAR_WARP_AMPLITUDE * (starValueNoise3(STAR_WARP_SCALE * q + STAR_WARP_RADIUS * vec3(cos(drift), 0.0, sin(drift))) - 0.5);
  // How small the granules are on screen: 1 up to ${GRANULE_SMALL_PX[0]} px, 0 from
  // ${GRANULE_SMALL_PX[1]} px. Each look is computed only while it shows
  // (uniform branches): the cells' edges up close, the blobs when small.
  float small = 1.0 - smoothstep(${GRANULE_SMALL_PX[0].toFixed(1)}, ${GRANULE_SMALL_PX[1].toFixed(1)}, uGranulePx);
  float lit = 0.0;
  float blob = 0.0;
  if (small >= 1.0) {
    blob = starCellBlobs(q, phase);
  } else {
    StarCell cell;
    // Between the two looks, the blobs are summed in the same pass.
    if (small > 0.0) cell = starCellDetail(q, phase, STAR_BLOB_SHARPNESS);
    else cell = starCellDetail(q, phase, 0.0);
    blob = cell.blobs;
    // About 2.5 px of softness, in cells; at most 0.1 cell.
    float soft = min(2.5 / max(uGranulePx, 1.0), 0.1);
    float s = cell.centre / (cell.centre + cell.edge + 1e-4);
    float tone = 0.75 + 0.45 * fract(cell.id * 13.7);
    tone *= 1.0 - 0.35 * smoothstep(0.15, 0.5, cell.depth);
    // D's dome: bright at the centre, falling towards the edge, and
    // faster near it, which rounds the cell's corners (where s nears 1
    // first) into a curved outline.
    float dome = 1.2 - 0.75 * s * s - 0.35 * smoothstep(0.75, 1.0, s);
    // A dark middle in a few large cells.
    float large = smoothstep(0.45, 0.7, cell.centre + cell.edge) * step(0.75, fract(cell.id * 7.31));
    dome -= 0.35 * large * (1.0 - smoothstep(0.0, 0.35, s));
    // Only the width varies (edge2 may differ across a lane, where edge
    // is 0). Lanes widen into soft pools at the junctions, and every lane
    // edge is at least about 2.5 px soft.
    float junction = 1.0 - smoothstep(0.0, 0.55, cell.edge2);
    float width = mix(0.05, 0.26, junction * junction) + soft;
    float lane = 1.0 - smoothstep(0.0, width, cell.edge);
    lit = tone * dome * (1.0 - lane);
  }
  // Small on screen, a sum of round blobs (starCellBlobs).
  float raw = mix(lit, 1.6 * blob + 0.3, small);
  // Brought to zero mean and unit rms at either end of the size range.
  float mean = mix(${GRANULE_RAW_MEAN.toFixed(4)}, ${GRANULE_SMALL_RAW_MEAN.toFixed(4)}, small);
  float rms = mix(${GRANULE_RAW_RMS.toFixed(4)}, ${GRANULE_SMALL_RAW_RMS.toFixed(4)}, small);
  return ${GRANULE_GAIN.toFixed(4)} * (raw - mean) / rms;
}
#endif

/** Brightness of a round spot of radius rs at distance d: umbra 0.3, penumbra 0.75 (Solanki 2003), part 1's spot. */
float starSpotRound(float d, float rs, float aa) {
  float ru = rs * 0.4472136; // umbra: a fifth of the spot's area
  float penumbra = 1.0 - smoothstep(rs - max(0.15 * rs, aa), rs + max(0.15 * rs, aa), d);
  float umbra = 1.0 - smoothstep(ru - max(0.2 * ru, aa), ru + max(0.2 * ru, aa), d);
  return mix(mix(1.0, 0.75, penumbra), 0.3, umbra);
}

/**
 * Days since the birth of the region whose spots starSpotShaped draws.
 * starBrightness sets it before each call: the Evershed flow and the
 * umbral dots move on it (the brief keeps starSpotShaped's signature to
 * the region's age as a share of its life).
 */
float starSpotDays = 0.0;

/**
 * The inverse of the standard normal distribution at p in (0, 0.5]: the
 * rational approximation of Abramowitz & Stegun 26.2.23 (error under
 * 4.5e-4). ${SPOT_BREAKUP_MEDIAN} + ${SPOT_BREAKUP_SD} · this is close to the level
 * below which a share p of one octave of value noise lies.
 */
float starProbit(float p) {
  float t = sqrt(-2.0 * log(clamp(p, 1e-6, 0.5)));
  return -(t - (2.515517 + 0.802853 * t + 0.010328 * t * t) / (1.0 + t * (1.432788 + t * (0.189269 + 0.001308 * t))));
}

/**
 * Brightness factor of a shaped spot (prototype D's live path, scaled from
 * its RMAX to rs): \`local\` in radians on the tangent plane, centred on the
 * spot; rs the radius of a disc of the spot's area; seed in [0, 1) the
 * region's (the follower's is fract(seed + 0.5)); age the region's age as
 * a share of its life; aa about a pixel, radians. 1 outside.
 *
 * - The shape lives in spot radii, so it grows and shrinks with the
 *   region's area, and evolves with age (its noise moves 2 units over a
 *   life). Young, it is compact: a nearly round edge and short filaments.
 *   Mature (from about a quarter of its life), the edge is torn and the
 *   penumbra frayed by its filaments.
 * - The penumbra's filaments: value noise on a ring round the spot
 *   (starValueNoiseRing, no seam), stretched along the radius: faint
 *   bundles of 88 cells a turn, and filaments of 251 and 565 cells a turn
 *   (prototype D's circles of radius 40 and 90 noise units) drifting
 *   outward with the Evershed flow.
 * - The outline: its second and third angular harmonics and two octaves
 *   of noise, so that it stays curved (lattice noise alone draws straight
 *   runs).
 * - The penumbra about ${SPOT_PENUMBRA} (filaments 0.7 to 0.88), the umbra about 0.3,
 *   as part 1's (Solanki 2003): on screen only the umbra and the pores
 *   read as dark, as with part 1's spots.
 * - Umbral dots (computed only inside the umbra) and, on about a third of
 *   the large spots between 0.30 and 0.80 of the life, a light bridge
 *   across the umbra.
 * - From 0.45 of its life its penumbra fragments: a share of it, up to
 *   ${SPOT_FRAGMENT_LOST} of the spot at its death, opens into gaps of photosphere,
 *   and the spot spreads by 1 / sqrt(1 − 0.78 share) so that its pieces
 *   keep its area; the umbra keeps its size and is not cut.
 * - Pores in the moat, 1.05 to 2.6 rs from its centre (computed only
 *   there), on a jittered grid of half spot radii; they take their area
 *   from the spot (SPOT_PORE_SHARE).
 *
 * Every edge is at least about a pixel soft, and every detail fades out
 * before it is smaller than a couple of pixels, so a small spot is a soft,
 * irregular blot; under about a pixel of radius it is part 1's round spot
 * (starSpotRound). The moving details (filaments, dots) fade with the
 * clock's speed (uSpotMotion), as a long exposure averages them out.
 * The shape's numbers are **calibrations**, matched by eye to prototype D
 * and scaled so that the drawn dark area stays within 20 % of a round
 * spot's (STAR_SPOT_SCALE).
 */
float starSpotShaped(vec2 local, float rs, float seed, float age, float aa) {
  // Under about a pixel the shape cannot be seen: the round spot of part 1,
  // blended into the shaped one from 0.7 to 1.5 px of radius, so that a
  // spot being born, dying or seen small keeps its area and does not blink.
  float detail = smoothstep(0.7, 1.5, rs / aa);
  float roundSpot = starSpotRound(length(local), rs, aa);
  if (detail <= 0.0) return roundSpot;
  float radius = max(rs, 1e-7);
  vec2 sp = local / radius;
  float rho = length(sp);
  // About a pixel, in spot radii (at most 1 / 0.7 here).
  float px = aa / radius;
  // Compact while growing, spread out from about a quarter of the life.
  float grown = smoothstep(0.02, 0.25, age);
  // The share of the spot opened into gaps in the decline, and the spread
  // that keeps the pieces' area.
  // Only gaps wide enough to be seen (the breakup's features, a quarter
  // of a spot radius, over 1.5 to 5 px) are opened: a blurred gap would
  // not lighten the spot, and its spread would add area.
  float gapPx = 0.2 * radius * STAR_SPOT_SCALE / aa;
  float lost = ${SPOT_FRAGMENT_LOST.toFixed(2)} * smoothstep(0.45, 1.0, age) * smoothstep(1.5, 5.0, gapPx);
  // The umbra (a fifth of the area, held at its size) is not cut.
  float spread = inversesqrt(1.0 - 0.78 * lost);
  // The torn, lobed outline of a grown spot covers a little more than its
  // mean radius: brought back.
  float scale = STAR_SPOT_SCALE * spread * (1.0 - ${SPOT_GROWN_TRIM.toFixed(3)} * grown);
  // The moat's pores (below) darken about ${SPOT_PORE_SHARE} of the spot's area
  // once they show: the spot gives it up, so that the region's dark area
  // stays its own.
  float moatAge = smoothstep(0.15, 0.4, age);
  float cellPx = 0.5 * radius / aa;
  scale *= sqrt(1.0 - ${SPOT_PORE_SHARE.toFixed(3)} * moatAge * smoothstep(1.0, 2.0, 0.12 * cellPx));
  // The shape reaches 2 of its scale plus two pixels of softness (its
  // edge at most about 1.85 of it, where the lobes and the noise both pull
  // rd in), the moat 2.6 spot radii; the round spot is inside both. At
  // the largest scale, 1.19, that is 1.84 r for the leader (0.775 r), inside
  // the region's reach of 2.02 r.
  float core = 2.0 * scale + 2.0 * px;
  if (rho >= max(core, 2.6)) return 1.0;
  vec3 so = vec3(173.0, 91.0, 57.0) * seed;
  float spot = 1.0;
  if (rho < core) {
    // Shape coordinates: 1 at the (mean) outer edge of the penumbra.
    vec2 q = sp / scale;
    float r = rho / scale;
    float pxQ = px / scale;
    // The torn edges, the umbra's edge and the coarse breakup: three
    // channels of two octaves of noise, evolving with age.
    vec3 shape = starFbm2x3(vec3(q * 2.0, 2.0 * age) + so);
    // The angle round the spot, in turns, twisted with the radius by a
    // curvature that varies round the edge (the outline's noise).
    float turns = atan(q.y, q.x + 1e-12) * 0.15915494 + (0.0286 + 0.08 * (shape.x - 0.5)) * r;
    // Filaments: noise periodic round the spot (no seam), stretched along
    // the radius. Three sets: bundles (88 cells a turn), which change only
    // with age and show from afar; filaments (251 and 565 cells a turn, as
    // prototype D's circles of radius 40 and 90), which drift outward
    // with the Evershed flow, each computed only once a cell spans about
    // two pixels (at 0.7 of the radius) and faded with the clock's speed.
    float width = 0.7 * 6.2831853 * radius * scale / aa;
    float salt = floor(seed * 997.0);
    float bundle = starValueNoiseRing(vec2(88.0 * (turns + seed), r * 1.2 + 4.0 * age), 88.0, salt).z - 0.5;
    float flow = STAR_EVERSHED_RATE * starSpotDays;
    float fade1 = smoothstep(1.5, 3.5, width / 251.0);
    float fade2 = smoothstep(1.5, 3.5, width / 565.0);
    float fine = 0.0;
    // The bundles shift the filaments' phase round the spot, so their
    // width varies (squeezed here, spread there); the radial frequency
    // varies with the outline's noise, so their length does.
    float stretch = 1.0 + 0.8 * (shape.y - 0.5);
    if (fade1 > 0.0) fine += 0.6 * fade1 * (starValueNoiseRing(vec2(251.0 * turns + 2.5 * bundle, r * 3.0 * stretch - flow), 251.0, salt + 1.0).x - 0.5);
    if (fade2 > 0.0) fine += 0.4 * fade2 * (starValueNoiseRing(vec2(565.0 * turns + 5.5 * bundle, r * 5.0 * stretch - 1.3 * flow), 565.0, salt + 2.0).y - 0.5);
    fine *= uSpotMotion;
    // Bundles faint, filaments strong; the young penumbra is fainter in
    // structure.
    float filament = 0.5 + mix(0.6, 1.0, grown) * (0.18 * bundle + 0.9 * fine);
    // The outline's lobes: its second and third harmonics in angle, of
    // phases set by the seed and turning slowly with age. Smooth curves,
    // where a contour of lattice noise alone draws straight runs.
    float lobes = cos(12.566371 * turns - 6.2831853 * fract(seed * 7.13) - 1.3 * age)
      + 0.7 * cos(18.849556 * turns - 6.2831853 * fract(seed * 3.71) + 1.7 * age);
    // Compact while young (a little torn), lobed and torn when grown, the
    // edge frayed a little by the bundles and filaments.
    float rd = r * (1.0 + mix(0.05, 0.12, grown) * lobes + mix(0.35, 0.5, grown) * (shape.x - 0.5));
    // Each filament ends at its own radius: the edge frays into the
    // granules (they come back between the filaments' ends).
    float outer = 1.0 + mix(0.3, 0.6, grown) * (0.05 * bundle + 0.6 * fine);
    // At least a pixel soft on either side, about the same midpoint.
    float pen = 1.0 - smoothstep(outer - 0.05 - pxQ, outer + pxQ, rd);
    // The umbra, about a fifth of the area (${SPOT_UMBRA_RADIUS} of the radius), follows
    // the outline at half its distortion, so that its own area varies
    // little from spot to spot: it is what reads as dark on screen.
    // A light bridge across the umbra, in mid-life, on about a third of
    // the spots (a draw of their own) and only on large ones (from about
    // 0.008 to 0.012 star radii, 6 to 8 Mm on the Sun). The umbra widens a
    // little round it, so that it keeps its area.
    float bridgeOn = step(fract(seed * 7.31), ${SPOT_BRIDGE_SHARE.toFixed(2)}) * smoothstep(0.008, 0.012, rs)
      * smoothstep(0.30, 0.40, age) * (1.0 - smoothstep(0.70, 0.80, age));
    // The umbra keeps its size as the spot spreads in its decline: its
    // pieces are the penumbra's.
    float ru = ${SPOT_UMBRA_RADIUS.toFixed(3)} * (1.0 + 0.08 * bridgeOn) / spread * (1.0 + 0.3 * (shape.y - 0.5) - 0.06 * lobes);
    float ra = 0.5 * (r + rd);
    float umb = 1.0 - smoothstep(ru - 0.04 - pxQ, ru + 0.02 + pxQ, ra * (1.0 + 0.1 * (filament - 0.5)));
    // Dark and bright filaments about the penumbra's ${SPOT_PENUMBRA.toFixed(2)} (Solanki
    // 2003's 0.75), between 0.7 and 0.88: no filament as dark as the
    // umbra, none as bright as the photosphere.
    float penB = clamp(${SPOT_PENUMBRA.toFixed(2)} + 0.6 * (filament - 0.5), 0.7, 0.88);
    // The umbra about 0.3 (Solanki 2003), varying smoothly over it: no
    // gradient that follows its outline, which banded into terraces.
    float umbB = 0.3 + 0.06 * (shape.z - 0.5);
    if (umb > 0.0) {
      // Umbral dots: round, soft points on a jittered grid of 0.07 spot
      // radii turned by the seed, one in about a third of the cells, 0.12
      // to 0.2 cell in radius, its centre within 0.2 of the cell's middle
      // and its edge at most 0.08 soft, so that it stays inside its cell
      // (the grid never cuts one); each twinkles on its own phase with a
      // period of 20 minutes.
      float turn = 6.2831853 * seed;
      vec2 dq = mat2(cos(turn), sin(turn), -sin(turn), cos(turn)) * q * 14.0 + so.yz;
      vec2 cell = floor(dq);
      vec3 h = starHash3(vec3(cell, floor(seed * 991.0) + 3.0));
      // 0.07 spot radii a cell: shown once a dot spans about 1.5 px.
      float cellPx = radius * scale / (14.0 * aa);
      float dr = 0.12 + 0.08 * fract(h.z * 7.0);
      float dotFade = h.z > 0.65 ? smoothstep(1.0, 2.0, dr * cellPx) * uSpotMotion : 0.0;
      if (dotFade > 0.0) {
        float softD = min(max(0.4 * dr, 1.0 / cellPx), 0.08);
        float d = length(dq - cell - 0.5 - 0.4 * (h.xy - 0.5));
        float twinkle = 0.6 + 0.4 * sin(6.2831853 * fract(STAR_UMBRAL_DOT_RATE * starSpotDays + h.z * 13.0));
        // A soft blob, brightest at its centre: no rim.
        umbB += 0.25 * dotFade * twinkle * (1.0 - smoothstep(0.0, dr + softD, d));
      }
    }
    float bridge = 0.0;
    if (bridgeOn > 0.0) {
      float angle = 12.566371 * seed;
      float dl = abs(dot(q, vec2(-sin(angle), cos(angle))) + 0.12 * (shape.x - 0.5));
      bridge = bridgeOn * (1.0 - smoothstep(0.03 + pxQ, 0.09 + 0.04 * bridgeOn + pxQ, dl));
    }
    float inner = mix(penB, umbB, umb);
    // The bridge is bright as the penumbra's mean, with no filaments (they
    // meet at the centre).
    inner = mix(inner, 0.8 + 0.1 * (shape.y - 0.5), bridge * umb);
    // Fragmentation: gaps where the breakup noise (about a fifth of a spot
    // radius) falls below the level of the share lost.
    if (lost > 1e-4) {
      // Two lattices, the second finer and turned by 37°, so that the
      // gaps' outlines follow neither: one octave alone cuts blocky,
      // axis-aligned holes.
      vec2 qt = mat2(0.7986355, 0.6018150, -0.6018150, 0.7986355) * q;
      float breakup = 0.5 * (shape.z + starValueNoise3(vec3(qt * 5.0, 2.0 * age) + so.zxy).x);
      float level = ${SPOT_BREAKUP_MEDIAN.toFixed(3)} + ${SPOT_BREAKUP_SD.toFixed(3)} * starProbit(lost);
      float w = max(0.06, 1.2 * pxQ);
      pen = max(pen * smoothstep(level - w, level + w, breakup), umb);
    }
    spot = mix(1.0, inner, pen);
  }
  // Pores in the moat, from about a fifth of the life: on a grid of half
  // spot radii, a pore in an eighth of the cells, 0.12 to 0.2 cell in
  // radius, its centre within 0.2 of the cell's middle and its soft edge
  // at most 0.09 cell wide, so that it stays inside its cell (the grid
  // never cuts one).
  float moat = smoothstep(1.05, 1.3, rho) * (1.0 - smoothstep(1.9, 2.6, rho)) * moatAge;
  if (moat > 0.0) {
    vec2 c = sp * 2.0 + so.xy;
    vec2 cell = floor(c);
    vec3 h = starHash3(vec3(cell, floor(seed * 997.0)));
    float pr = 0.12 + (h.z - 0.875) * 0.64;
    // A pore shows once it is about 1.5 px in radius.
    float poreFade = h.z > 0.875 ? smoothstep(1.0, 2.0, pr * cellPx) : 0.0;
    if (poreFade > 0.0) {
      float soft = min(max(0.4 * pr, 1.0 / cellPx), 0.09);
      float d = length(c - cell - 0.5 - 0.4 * (h.xy - 0.5));
      spot *= 1.0 - 0.5 * moat * poreFade * (1.0 - smoothstep(pr - soft, pr + soft, d));
    }
  }
  return mix(roundSpot, spot, detail);
}

/** The quadratic limb law, I(μ) = 1 − u1 (1 − μ) − u2 (1 − μ)². */
float starLimb(float mu) {
  float edge = 1.0 - mu;
  return 1.0 - uLimb.x * edge - uLimb.y * edge * edge;
}

/** Brightness of the photosphere at n before the limb law: 1 quiet, less in spots, more in faculae and bright granules. */
float starBrightness(vec3 n, float mu) {
  float edge = 1.0 - mu;
  float brightness = 1.0;
  if (uNetwork.w > 0.0) {
    // The supergranule network, sampled in the star's turning frame
    // (longitude minus the equatorial rotation phase, no shear: a cell
    // lives a day). Its lanes are soft, wide ribbons of faculae,
    // stronger towards the limb, wider and brighter at the junctions,
    // of a tone varying from one lane to the next; from afar a
    // mesogranular mottle a third of the cell's size varies them along
    // their length and mottles the cells. The mottle fades out as the
    // granules, which cover its scale, fade in. All calibrations,
    // slightly exaggerated so that the whole disc is not smooth.
    float spinC = cos(uNetworkSpin);
    float spinS = sin(uNetworkSpin);
    vec3 q = vec3(n.x * spinC - n.z * spinS, n.y, n.x * spinS + n.z * spinC) / uNetwork.x;
    vec3 lanes = starCellLanes(q, uNetwork.z);
    float junction = 1.0 - smoothstep(0.0, STAR_NETWORK_JUNCTION, lanes.y);
    float width = STAR_NETWORK_WIDTH * (0.7 + 0.6 * junction);
    // Each lane's own tone, blended to a common one at the junctions,
    // where lanes of different tones meet.
    float tone = mix(0.6 + 0.4 * fract(lanes.z * 7.31), 0.8, junction) * (0.8 + 0.4 * junction);
    float mottle = 0.0;
#ifdef STAR_NO_GRANULATION
    float mottleFade = 1.0;
#else
    float mottleFade = 1.0 - uGranule.w;
#endif
    if (mottleFade > 0.0) {
      vec3 v = starValueNoise3(q * (1.0 / 0.35));
      mottle = mottleFade * 0.5 * (v.x - 0.5);
      width *= mix(1.0, 0.5 + v.y, mottleFade);
      tone *= mix(1.0, 0.3 + 1.4 * v.z, mottleFade);
    }
    float ribbon = 1.0 - smoothstep(0.0, width, lanes.x);
    float net = min(tone * ribbon, 1.0);
    // Brighter towards the limb, but capped so that a ribbon's peak stays
    // below a plage's at the same μ (STAR_NETWORK_LIMB_CAP), except near
    // the disc centre, where plages vanish and the network keeps 0.35.
    float limb = max(0.35, min(0.35 + 1.4 * edge, STAR_NETWORK_LIMB_CAP * edge));
    brightness += uNetwork.w * uNetwork.y * (net * limb + mottle);
  }
  float spot = 1.0;
  float facula = 0.0;
  if (uRegionFade > 0.0 && uRegionCount > 0) {
    float latitude = asin(clamp(n.y, -1.0, 1.0));
    // About a pixel on the sphere, to keep small spots from aliasing; a
    // material calling this in non-uniform control flow, where fwidth is
    // undefined, defines STAR_PIXEL_ANGLE as the angle it measured outside.
#ifdef STAR_PIXEL_ANGLE
    float aa = STAR_PIXEL_ANGLE;
#else
    float aa = length(fwidth(n));
#endif
    for (int i = 0; i < STAR_REGIONS; i++) {
      if (i >= uRegionCount) break;
      vec4 a = uRegionA[i];
      vec4 b = uRegionB[i];
      float t = uLocalDays - b.x;
      if (t <= 0.0 || t >= b.z) continue;
      float area = b.w * (t < b.y ? t / max(b.y, 1e-6) : (b.z - t) / max(b.z - b.y, 1e-6));
      // Radius of a disc of the region's spot area (2π · area steradians).
      float r = sqrt(2.0 * area);
      float plage = 3.0 * r;
      // The leader's moat reaches 2.6 of its radius (0.775 r): 2.02 r; its
      // shape, two pixels of softness more.
      float reach = max(plage, 0.5 * a.w + 2.02 * r + 2.0 * aa) + aa;
      if (abs(latitude - a.x) > reach) continue;
      float sinLat = sin(a.x);
      float cosLat = cos(a.x);
      float longitude = a.y + uOmegaEq * (1.0 - uShear * sinLat * sinLat) * uLocalDays;
      float sinLon = sin(longitude);
      float cosLon = cos(longitude);
      vec3 centre = vec3(cosLat * cosLon, sinLat, -cosLat * sinLon);
      vec3 offset = n - centre;
      if (dot(offset, offset) > reach * reach || dot(n, centre) < 0.0) continue;
      // Local coordinates on the tangent plane: east, north (radians).
      vec2 local = vec2(dot(n, vec3(-sinLon, 0.0, -cosLon)), dot(n, vec3(-sinLat * cosLon, cosLat, sinLat * sinLon)));
      // The bipole's axis, the leader east and equatorward (Joy's law).
      vec2 axis = vec2(cos(a.z), -sign(a.x) * sin(a.z)) * (0.5 * a.w);
      float seed = uRegionC[i].x;
      float age = t / b.z;
      starSpotDays = t;
      spot = min(spot, starSpotShaped(local - axis, r * 0.7745967, seed, age, aa));
      spot = min(spot, starSpotShaped(local + axis, r * 0.6324555, fract(seed + 0.5), age, aa));
      facula = max(facula, 1.0 - smoothstep(0.6 * plage, plage + aa, length(local)));
    }
  }
#ifndef STAR_NO_GRANULATION
  // No granules inside a spot or a pore: they fade out across the
  // spot's soft edge and its brightest filaments (0.8 to 0.98), and are
  // not computed where they are gone, which pays for the spot's own
  // detail.
  float bare = mix(1.0, smoothstep(0.8, 0.98, spot), uRegionFade);
  if (uGranule.w > 0.0 && bare > 0.0) brightness += bare * uGranule.y * uGranule.w * starGranules(n);
#endif
  brightness *= mix(1.0, spot, uRegionFade);
  float fill = min(uFacula / 9.0, 1.0);
  brightness += uRegionFade * ${PLAGE_LIMB_CONTRAST.toFixed(2)} * edge * fill * facula * smoothstep(0.75, 1.0, spot);
  return brightness;
}

/** The photosphere's linear colour: its light times the brightness and the limb law. */
vec3 starSurfaceColour(vec3 n, float mu) {
  return uStarColour * starBrightness(n, mu) * starLimb(mu);
}

/**
 * The photosphere for display: the soft exposure 1 − exp(−1.6 b) on the
 * brightness only, then the limb law and the star's light. The curve
 * keeps bright faculae and granules from clipping, while the limb law
 * and the colour stay linear: applied to the whole colour, the curve
 * flattens the limb and pulls every channel towards white.
 */
vec3 starSurfaceExposed(vec3 n, float mu) {
  return uStarColour * (1.0 - exp(-1.6 * starBrightness(n, mu))) * starLimb(mu);
}

/** A screen-space dither in [−0.5, 0.5) (interleaved gradient noise, Jimenez 2014), against 8-bit bands on a smooth disc. */
float starDither(vec2 fragCoord) {
  return fract(52.9829189 * fract(dot(fragCoord, vec2(0.06711056, 0.00583715)))) - 0.5;
}
`;

/**
 * The uniforms of `STAR_SURFACE_GLSL`, the region arrays `maxRegions`
 * long (the material's `STAR_REGIONS`): uStarColour, uLimb (vec2),
 * uGranule (vec4: size, contrast, phase, fade), uGranulePx, uNetwork (vec4: cell
 * size, contrast, phase, fade), uNetworkSpin, uRegionFade,
 * uRegionCount, uRegionA, uRegionB and uRegionC (vec4 each, packed in a
 * Float32Array), uSpotMotion, uLocalDays,
 * uOmegaEq, uShear, uFacula.
 */
export function starSurfaceUniforms(maxRegions: number): Record<string, IUniform> {
  return {
    uStarColour: { value: new Vector3(1, 1, 1) },
    uLimb: { value: new Vector2(0.6, 0.2) },
    uGranule: { value: new Vector4(SUN_GRANULE_SIZE, 0, 0, 0) },
    uGranulePx: { value: 0 },
    uNetwork: { value: new Vector4(networkSize(SUN_GRANULE_SIZE), 0, 0, 0) },
    uNetworkSpin: { value: 0 },
    uRegionFade: { value: 0 },
    uRegionCount: { value: 0 },
    uRegionA: { value: new Float32Array(4 * maxRegions) },
    uRegionB: { value: new Float32Array(4 * maxRegions) },
    uRegionC: { value: new Float32Array(4 * maxRegions) },
    uSpotMotion: { value: 1 },
    uLocalDays: { value: 0 },
    uOmegaEq: { value: TWO_PI / 25 },
    uShear: { value: 0 },
    uFacula: { value: 10 },
  };
}

/** `x` modulo `m`, in [0, m). */
const modulo = (x: number, m: number): number => {
  const r = ((x % m) + m) % m;
  return r >= m ? 0 : r;
};

/**
 * Keeps a star's surface uniforms in step with its clock: the fades, the
 * granule phase, and the active regions alive at the day.
 */
export class StarSurfaceState {
  private look: StarLook | null = null;
  private activity: StarActivity | null = null;
  /** Ids of the regions packed. */
  private packed = new Set<number>();
  private epoch: number | null = null;
  /** The day the regions were last found for, to find them once per day shown. */
  private lastDays = Number.NaN;
  private drawn: readonly RegionNow[] = [];
  /** Every region alive at some day of [windowFrom, windowTo] (see regionsAt). */
  private candidates: ActiveRegion[] = [];
  private windowFrom = Number.NaN;
  private windowTo = Number.NaN;

  constructor(
    private readonly uniforms: Record<string, IUniform>,
    private readonly maxRegions: number,
  ) {}

  /** The regions drawn now (for the loops), largest first; none while they are faded out. */
  get regions(): readonly RegionNow[] {
    return this.drawn;
  }

  /** Share of the regions' fade due to the star's size on screen (0 below 6 px, 1 from 12). */
  private sizeFade = 0;
  /** The exposure fade of the regions: below 1, the clock is faster than the regions change. */
  private exposure = 1;

  /**
   * The regions to stand loops on: `regions`, but none while the clock is so
   * fast that the spots are averaged out (a long exposure), when the set
   * changes almost entirely every frame and loops would only pop in and out.
   */
  get loopRegions(): readonly RegionNow[] {
    return this.exposure < 1 ? [] : this.drawn;
  }

  /** The loops' opacity factor: the spots' ramp with the star's size on screen. */
  get loopFade(): number {
    return this.sizeFade;
  }

  /** Shows another star (or the same with another activity). */
  setStar(look: StarLook, activity: StarActivity | null): void {
    this.look = look;
    this.activity = activity;
    const u = this.uniforms;
    (u['uStarColour']!.value as Vector3).set(...look.colour);
    (u['uLimb']!.value as Vector2).set(look.limb[0], look.limb[1]);
    u['uOmegaEq']!.value = look.omegaEq;
    u['uShear']!.value = look.shear;
    u['uFacula']!.value = look.faculaRatio;
    this.candidates = [];
    this.windowFrom = this.windowTo = Number.NaN;
    this.clearRegions();
  }

  /**
   * The regions alive at `days`, largest first, at most `maxRegions`:
   * exactly `activeRegions(activity, days, maxRegions)`, found among the
   * cached candidates, which are drawn from the timeline again only when
   * `days` leaves their window (about 160 day-slots, against a filter of
   * a few hundred regions per frame).
   */
  private regionsAt(activity: StarActivity, days: number): RegionNow[] {
    if (!(days >= this.windowFrom && days <= this.windowTo)) {
      const from = days - WINDOW_DAYS;
      const to = days + WINDOW_DAYS;
      const list: ActiveRegion[] = [];
      for (let n = Math.floor(from - MAX_LIFETIME_DAYS); n <= Math.floor(to); n++) {
        for (const region of regionsBornIn(activity, n)) {
          if (region.born <= to && region.born + region.lifetime > from) list.push(region);
        }
      }
      this.candidates = list;
      this.windowFrom = from;
      this.windowTo = to;
    }
    const alive: RegionNow[] = [];
    for (const region of this.candidates) {
      const now = regionNow(activity, region, days);
      if (now) alive.push(now);
    }
    return rankRegions(alive, this.maxRegions);
  }

  private clearRegions(): void {
    this.packed = new Set();
    this.epoch = null;
    this.lastDays = Number.NaN;
    this.drawn = [];
    this.uniforms['uRegionCount']!.value = 0;
    this.uniforms['uLocalDays']!.value = 0;
  }

  /**
   * Sets the uniforms for `days` (the system clock), the star `radiusPx`
   * pixels in radius on screen and the clock running `daysPerSecond`;
   * repacks the region arrays only when the drawn set's ids change or
   * |days − epoch| > 20. Returns whether it repacked. `regionDetail`, when
   * given, replaces the regions' ramp with the star's size (see
   * `surfaceDetail`): the sky disc has its own (`hostStarDetail`).
   * `networkDetail`, when given, caps the network's ramp the same way
   * (the sky disc passes `hostStarDetail` too, 0 for its usual 14 px).
   */
  update(
    days: number,
    view: { radiusPx: number; daysPerSecond: number; regionDetail?: number; networkDetail?: number },
  ): boolean {
    const look = this.look;
    if (!look) return false;
    const u = this.uniforms;
    const daysPerFrame = Math.abs(view.daysPerSecond) / 30;
    const detail = surfaceDetail(view.radiusPx, look.granuleSize);

    const granuleFade =
      look.granuleContrast > 0 ? detail.granulation * exposureFade(daysPerFrame, look.granuleLifetimeDays) : 0;
    const phase = granulePhase(days, look.granuleLifetimeDays);
    (u['uGranule']!.value as Vector4).set(look.granuleSize, look.granuleContrast, phase, granuleFade);
    u['uGranulePx']!.value = view.radiusPx * look.granuleSize;

    const rotationDays = TWO_PI / look.omegaEq;
    const rotationExposure = exposureFade(daysPerFrame, rotationDays / 12);
    const contrast = networkContrast(look.granuleContrast);
    // It boils on its own lifetime and turns with the star: either can smear it.
    const networkFade =
      contrast > 0
        ? Math.min(detail.network, view.networkDetail ?? 1) *
          Math.min(exposureFade(daysPerFrame, SUPERGRANULE_LIFETIME_DAYS), rotationExposure)
        : 0;
    (u['uNetwork']!.value as Vector4).set(networkSize(look.granuleSize), contrast, networkPhase(days), networkFade);
    u['uNetworkSpin']!.value = this.activity ? rotationPhase(this.activity, days) : modulo(look.omegaEq * days, TWO_PI);

    this.exposure = rotationExposure;
    this.sizeFade = view.regionDetail ?? detail.regions;
    const regionFade = this.sizeFade * this.exposure;
    u['uRegionFade']!.value = regionFade;
    u['uSpotMotion']!.value = exposureFade(daysPerFrame, SPOT_MOTION_DAYS);

    const activity = this.activity;
    if (!activity || !(activity.region_rate > 0) || !(regionFade > 0)) {
      if (this.epoch !== null || this.drawn.length > 0) this.clearRegions();
      return false;
    }
    if (days === this.lastDays && this.epoch !== null) {
      u['uLocalDays']!.value = days - this.epoch;
      return false;
    }
    this.lastDays = days;
    const now = this.regionsAt(activity, days);
    this.drawn = now;
    const same =
      this.epoch !== null &&
      Math.abs(days - this.epoch) <= EPOCH_SPAN_DAYS &&
      now.length === this.packed.size &&
      now.every((r) => this.packed.has(r.region.id));
    if (!same) this.pack(now, days);
    u['uLocalDays']!.value = days - this.epoch!;
    return !same;
  }

  /** Packs `now` (the regions alive at `days`) with `days` as the epoch. */
  private pack(now: readonly RegionNow[], days: number): void {
    const a = this.uniforms['uRegionA']!.value as Float32Array;
    const b = this.uniforms['uRegionB']!.value as Float32Array;
    const c = this.uniforms['uRegionC']!.value as Float32Array;
    now.forEach(({ region, longitude }, i) => {
      c[4 * i] = hash32(region.id, SPOT_SEED_KEY) / 2 ** 32;
      a[4 * i] = region.latitude;
      a[4 * i + 1] = longitude;
      a[4 * i + 2] = region.tilt;
      a[4 * i + 3] = region.separation;
      b[4 * i] = region.born - days;
      b[4 * i + 1] = region.growth;
      b[4 * i + 2] = region.lifetime;
      b[4 * i + 3] = region.peakArea;
    });
    this.uniforms['uRegionCount']!.value = now.length;
    this.packed = new Set(now.map((r) => r.region.id));
    this.epoch = days;
  }
}
