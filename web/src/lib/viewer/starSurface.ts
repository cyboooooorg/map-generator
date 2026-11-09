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
 *   dark lanes between them and bright points in the lanes, whose
 *   centres wander with the granule phase.
 * - `StarSurfaceState` keeps the uniforms in step with the clock: it asks
 *   the region timeline (lib/starActivity.ts) for the regions alive at a
 *   day and packs them into two small uniform arrays, again only when the
 *   set changes. The shader rebuilds each region's area and longitude
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
 * over 20 cells and one octave of value noise per pixel (48 hashes, one
 * more in the lanes), about four and a half times the two simplex
 * fields it replaced. The supergranule network, drawn over the whole
 * disc from afar, is kept to about a third of that: 8 cells and, from
 * afar only, one octave of value noise (16 hashes); up close, where
 * the granules are drawn, the noise is skipped and the 8 cells remain.
 *
 * The science (spot structure, facular contrast, limb law) is in
 * docs/science/star-activity.md.
 */
import { type IUniform, Vector2, Vector3, Vector4 } from 'three';
import type { StarActivity } from '../api/types/StarActivity';
import type { StarInfo } from '../api/types/StarInfo';
import {
  type ActiveRegion,
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
 * disc centre about 2 % brighter on average with a 2.5 % rms, the limb
 * about 7 %. Times `min(1, granule_contrast / 0.15)`: 0 on a radiative
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
 * Mean and rms of the raw granule brightness (cell plus bright points),
 * measured on disc patches from the centre to 0.6 radii and at several
 * phases (see `starGranules`): they bring it to zero mean and unit rms.
 */
const GRANULE_RAW_MEAN = 0.576;
const GRANULE_RAW_RMS = 0.354;

/**
 * **Calibration**: the drawn granulation's rms over `granule_contrast`,
 * on the brightness b before the soft exposure curve.
 * Prototype D, the approved look, drew 0.62 × the raw rms (about 0.236)
 * on the Sun, whose `granule_contrast` is 0.15: a gain of about 1.6.
 */
const GRANULE_GAIN = 1.6;

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
 * starFbm2(vec3 p)`, `vec3 starWave(vec3 x)`, `StarCell starCellDetail(vec3
 * p, float phase)`, `vec2 starCells(vec3 p, float phase)` and `float
 * starCellLanes(vec3 p, float phase)`); `float
 * starLimb(float mu)`, `float
 * starBrightness(vec3 n, float mu)`, `vec3 starSurfaceExposed(vec3 n,
 * float mu)` (for display, see there), `float starDither(vec2)` and
 * `vec3 starSurfaceColour(vec3 n, float mu)`, the linear colour of the photosphere at unit normal `n`
 * in the star's inertial frame (y the spin axis, longitude increasing
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
 *   ones darker in the middle; the intergranular lanes are dark, narrow
 *   along long edges and wider at junctions, and about a fifth of them
 *   carry a soft bright point. Granules are the tops of convective cells: hot
 *   upflows, bright, ringed by cooler, darker downflow lanes (Nordlund,
 *   Stein & Asplund 2009, Living Rev. Sol. Phys. 6, 2), in which
 *   magnetic bright points sit (Sánchez Almeida et al. 2004, ApJ 609,
 *   L91). The cells' centres wander by 0.3 of a cell at 2.1 to 2.4 rad
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
 *   brightest at the limb (Solanki 2003). From afar a mesogranular
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
 *   leader east and equatorward, Joy's law), each an umbra at 0.3 of the
 *   photosphere within a penumbra at 0.75, the penumbra four times the
 *   umbra's area (Solanki 2003, A&ARv 11, 153). Around them a facular
 *   plage of 3× the region's spot radius, brightened by 0.25 (1 − μ)
 *   (**calibration**, brightest at the limb, Solanki 2003), with a fill
 *   of facula_ratio / 9 (the plage covers 9 spot areas; the Sun's ratio
 *   about 10 fills it), at most 1.
 *
 * Region arrays: `uRegionA[i] = (latitude, longitude at the epoch, tilt,
 * separation)`, `uRegionB[i] = (born − epoch, growth, lifetime,
 * peakArea)`, radians, days and shares of the hemisphere. The area is
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
/** The supergranule network: cell size (star radii), contrast, phase (lifetimes, mod 289), fade. */
uniform vec4 uNetwork;
/** The star's equatorial rotation phase (rad, in [0, 2π)): the network turns with it. */
uniform float uNetworkSpin;
uniform float uRegionFade;
uniform int uRegionCount;
uniform vec4 uRegionA[STAR_REGIONS];
uniform vec4 uRegionB[STAR_REGIONS];
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
/** The warp's frequency (per cell) and amplitude (cells, **calibration**): enough to curve the cells' edges. */
const float STAR_WARP_SCALE = 0.3;
const float STAR_WARP_AMPLITUDE = 1.0;

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

/** How far from a lane's middle its bright point reaches, in cells. */
const float STAR_LANE_POINT_RADIUS = 0.10;

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
  /** Distance to where the nearest edge crosses the segment between the two centres; 1e4 away from the lanes. */
  float point;
  /** A value in [0, 1) for the pair of cells across the nearest edge. */
  float pair;
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
StarCell starCellDetail(vec3 p, float phase) {
  vec3 i = floor(p);
  vec3 f = p - i;
  vec3 axis = normalize(p);
  vec3 side = sign(f - 0.5);
  side += 1.0 - abs(side);
  vec3 turns = fract(STAR_CELL_RATES * (phase * 0.15915494));
  float d1 = 1e4;
  vec3 r1 = vec3(0.0);
  float h1 = 0.0;
  for (int k = 0; k < 20; k++) {
    vec3 g = side * STAR_CELL_STEPS[k];
    vec3 hv = starHash3(i + g);
    vec3 r = g + 0.5 + 0.3 * starWave(turns + hv) - f;
    float d = dot(r, r);
    bool nearer = d < d1;
    d1 = nearer ? d : d1;
    r1 = nearer ? r : r1;
    h1 = nearer ? hv.x : h1;
  }
  float edge = 1e4;
  float edge2 = 1e4;
  int across = 0;
  for (int k = 0; k < 20; k++) {
    vec3 g = side * STAR_CELL_STEPS[k];
    vec3 r = g + 0.5 + 0.3 * starWave(turns + starHash3(i + g)) - f;
    vec3 dr = r - r1;
    float along = dot(dr, axis);
    float apart = dot(dr, dr) - along * along;
    // The nearest centre itself (apart 0) is no edge.
    float e = apart > 1e-8 ? 0.5 * (dot(r, r) - d1) * inversesqrt(apart) : 1e4;
    bool nearer = e < edge;
    edge2 = min(edge2, max(edge, e));
    edge = nearer ? e : edge;
    across = nearer ? k : across;
  }
  StarCell cell;
  cell.f1 = sqrt(d1);
  cell.edge = edge;
  cell.edge2 = edge2;
  float depth = dot(r1, axis);
  cell.depth = abs(depth);
  cell.centre = length(r1 - depth * axis);
  cell.id = h1;
  cell.point = 1e4;
  cell.pair = 0.0;
  if (edge < STAR_LANE_POINT_RADIUS) {
    // The neighbour across the nearest edge again, rather than carry it through the loop.
    vec3 g = side * STAR_CELL_STEPS[across];
    vec3 hv = starHash3(i + g);
    vec3 dr = g + 0.5 + 0.3 * starWave(turns + hv) - f - r1;
    vec3 normal = normalize(dr - dot(dr, axis) * axis);
    vec3 t1 = r1 - depth * axis;
    cell.point = length(t1 + (edge - dot(normal, t1)) * normal);
    cell.pair = fract(h1 + hv.x);
  }
  return cell;
}

/**
 * (F1, F2) of starCellDetail, F2 = F1 + 2 · edge: F2 − F1 is 0 on a
 * cell's edge and grows into the cell, as the lane measure of a 2D cell
 * pattern.
 */
vec2 starCells(vec3 p, float phase) {
  StarCell cell = starCellDetail(p, phase);
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
 * - dark intergranular lanes with soft edges, narrow along long edges
 *   and wider and darker at junctions;
 * - a soft bright point in the middle of a fifth of the lanes.
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
  StarCell cell = starCellDetail(q, phase);
  float s = cell.centre / (cell.centre + cell.edge + 1e-4);
  float tone = 0.75 + 0.45 * fract(cell.id * 13.7);
  tone *= 1.0 - 0.35 * smoothstep(0.15, 0.5, cell.depth);
  float dome = 1.2 - 0.75 * s * s;
  // A dark middle in a few large cells.
  float large = smoothstep(0.45, 0.7, cell.centre + cell.edge) * step(0.75, fract(cell.id * 7.31));
  dome -= 0.35 * large * (1.0 - smoothstep(0.0, 0.35, s));
  // Only the width varies (edge2 may differ across a lane, where edge is 0).
  float junction = 1.0 - smoothstep(0.0, 0.45, cell.edge2);
  float lane = 1.0 - smoothstep(0.0, mix(0.045, 0.22, junction * junction), cell.edge);
  float points = step(0.8, cell.pair) * (1.0 - smoothstep(0.0, STAR_LANE_POINT_RADIUS, cell.point));
  // D's weights: the cells, then the points at 0.35 / 0.62 of them.
  float raw = tone * dome * (1.0 - lane) + 0.5645 * points;
  return ${GRANULE_GAIN.toFixed(4)} * (raw - ${GRANULE_RAW_MEAN.toFixed(4)}) / ${GRANULE_RAW_RMS.toFixed(4)};
}
#endif

/** Brightness of a spot of radius rs at distance d: umbra 0.3, penumbra 0.75 (Solanki 2003). */
float starSpot(float d, float rs, float aa) {
  float ru = rs * 0.4472136; // umbra: a fifth of the spot's area
  float penumbra = 1.0 - smoothstep(rs - max(0.15 * rs, aa), rs + max(0.15 * rs, aa), d);
  float umbra = 1.0 - smoothstep(ru - max(0.2 * ru, aa), ru + max(0.2 * ru, aa), d);
  return mix(mix(1.0, 0.75, penumbra), 0.3, umbra);
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
#ifndef STAR_NO_GRANULATION
  if (uGranule.w > 0.0) brightness += uGranule.y * uGranule.w * starGranules(n);
#endif
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
    float tone = (0.6 + 0.4 * fract(lanes.z * 7.31)) * (0.8 + 0.4 * junction);
    float mottle = 0.0;
    float mottleFade = 1.0 - uGranule.w;
    if (mottleFade > 0.0) {
      vec3 v = starValueNoise3(q * (1.0 / 0.35));
      mottle = mottleFade * 0.5 * (v.x - 0.5);
      width *= mix(1.0, 0.5 + v.y, mottleFade);
      tone *= mix(1.0, 0.3 + 1.4 * v.z, mottleFade);
    }
    float ribbon = 1.0 - smoothstep(0.0, width, lanes.x);
    float net = tone * ribbon;
    brightness += uNetwork.w * uNetwork.y * (net * (0.35 + 1.4 * (1.0 - mu)) + mottle);
  }
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
    float spot = 1.0;
    float facula = 0.0;
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
      float reach = max(plage, 0.5 * a.w + r) + aa;
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
      spot = min(spot, starSpot(length(local - axis), r * 0.7745967, aa));
      spot = min(spot, starSpot(length(local + axis), r * 0.6324555, aa));
      facula = max(facula, 1.0 - smoothstep(0.6 * plage, plage + aa, length(local)));
    }
    brightness *= mix(1.0, spot, uRegionFade);
    float fill = min(uFacula / 9.0, 1.0);
    brightness += uRegionFade * 0.25 * edge * fill * facula * smoothstep(0.75, 1.0, spot);
  }
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
 * uGranule (vec4: size, contrast, phase, fade), uNetwork (vec4: cell
 * size, contrast, phase, fade), uNetworkSpin, uRegionFade,
 * uRegionCount, uRegionA and uRegionB (vec4 each), uLocalDays, uOmegaEq,
 * uShear, uFacula.
 */
export function starSurfaceUniforms(maxRegions: number): Record<string, IUniform> {
  return {
    uStarColour: { value: new Vector3(1, 1, 1) },
    uLimb: { value: new Vector2(0.6, 0.2) },
    uGranule: { value: new Vector4(SUN_GRANULE_SIZE, 0, 0, 0) },
    uNetwork: { value: new Vector4(networkSize(SUN_GRANULE_SIZE), 0, 0, 0) },
    uNetworkSpin: { value: 0 },
    uRegionFade: { value: 0 },
    uRegionCount: { value: 0 },
    uRegionA: { value: new Float32Array(4 * maxRegions) },
    uRegionB: { value: new Float32Array(4 * maxRegions) },
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

    const contrast = networkContrast(look.granuleContrast);
    const networkFade =
      contrast > 0
        ? Math.min(detail.network, view.networkDetail ?? 1) * exposureFade(daysPerFrame, SUPERGRANULE_LIFETIME_DAYS)
        : 0;
    (u['uNetwork']!.value as Vector4).set(networkSize(look.granuleSize), contrast, networkPhase(days), networkFade);
    u['uNetworkSpin']!.value = this.activity ? rotationPhase(this.activity, days) : modulo(look.omegaEq * days, TWO_PI);

    const rotationDays = TWO_PI / look.omegaEq;
    this.exposure = exposureFade(daysPerFrame, rotationDays / 12);
    this.sizeFade = view.regionDetail ?? detail.regions;
    const regionFade = this.sizeFade * this.exposure;
    u['uRegionFade']!.value = regionFade;

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
    now.forEach(({ region, longitude }, i) => {
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
