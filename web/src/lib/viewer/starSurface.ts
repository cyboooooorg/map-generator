/**
 * The living surface of a star, shared by every view that draws one: the
 * system view's star (StarView), the star view and a planet's sky disc.
 *
 * - `starLook` turns a star and its `StarActivity` into what the shader
 *   needs (limb darkening, granulation, rotation).
 * - `STAR_SURFACE_GLSL` is a GLSL chunk with its uniforms and
 *   `vec3 starSurfaceColour(vec3 n, float mu)`; a material includes it,
 *   defines `STAR_REGIONS` (the length of the region arrays) and, for a
 *   disc too small to show granules, `STAR_NO_GRANULATION`.
 * - `StarSurfaceState` keeps the uniforms in step with the clock: it asks
 *   the region timeline (lib/starActivity.ts) for the regions alive at a
 *   day and packs them into two small uniform arrays, again only when the
 *   set changes. The shader rebuilds each region's area and longitude
 *   from its packed birth, so between repacks only `uLocalDays` moves.
 *
 * Time: the shader sees only `uLocalDays`, days since the last repack
 * (under 21), and the granule phase reduced in JS: never an absolute day.
 * The surface at a day depends only on the activity (which carries the
 * seed), the day, and the nominal speed of the clock (the fades), never
 * on frame history or wall time.
 *
 * Cost: detail falls with the star's size on screen (`surfaceDetail`) and
 * with the clock's speed (`exposureFade`); the shader skips the region
 * loop and the granulation behind uniform branches when they are faded
 * out, so a small or fast-turning star costs only its limb darkening.
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
 * How much surface detail a star `radiusPx` pixels in radius shows:
 * granulation fades in from 1.5 to 4 pixels per granule (`granuleSize`
 * in star radii) and is off below; the active regions fade in from 6 to
 * 12 pixels of star radius and are off below.
 */
export function surfaceDetail(radiusPx: number, granuleSize: number): { granulation: number; regions: number } {
  return {
    granulation: ramp(radiusPx * granuleSize, 1.5, 4),
    regions: ramp(radiusPx, 6, 12),
  };
}

/**
 * Period of the granule pattern's phase: the shader offsets field k by
 * `mod(k · (17, 31, 7), 289)`, so the fields for k and k + 289 are the
 * same, and the phase can be reduced modulo 289 in JS without a jump
 * (the cross-fade from field 288 goes on to field 289, which is field 0).
 */
const GRANULE_PHASE_PERIOD = 289;
/** Days at which the granule clock wraps before its phase is computed (keeps it exact in f64). */
const GRANULE_DAYS_WRAP = 1e4;
/** Days from the epoch after which the regions are packed again, to keep `uLocalDays` small. */
const EPOCH_SPAN_DAYS = 20;
/**
 * Half-width of the window of days the cached candidate regions cover:
 * every region alive at some day within it. The timeline is drawn again
 * only when the clock leaves the window.
 */
const WINDOW_DAYS = 21;

/**
 * The surface's GLSL: its uniforms, `float starLimb(float mu)`, `float
 * starBrightness(vec3 n, float mu)`, `vec3 starSurfaceExposed(vec3 n,
 * float mu)` (for display, see there), `float starDither(vec2)` and
 * `vec3 starSurfaceColour(vec3 n, float mu)`, the linear colour of the photosphere at unit normal `n`
 * in the star's inertial frame (y the spin axis, longitude increasing
 * anticlockwise seen from +y, as the orbits go) and `mu` the cosine of
 * the angle to the line of sight.
 *
 * The photosphere's brightness is the quadratic limb law times the
 * granulation, the spots and the faculae, times `uStarColour`:
 *
 * - Granulation: simplex noise at the granule size, cross-faded between
 *   two fields over each granule lifetime (it boils rather than slides).
 *   Compiled out under `STAR_NO_GRANULATION` (noise and all).
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
uniform float uRegionFade;
uniform int uRegionCount;
uniform vec4 uRegionA[STAR_REGIONS];
uniform vec4 uRegionB[STAR_REGIONS];
/** Days since the regions were packed. */
uniform float uLocalDays;
uniform float uOmegaEq;
uniform float uShear;
uniform float uFacula;

#ifndef STAR_NO_GRANULATION
// 3D simplex noise (Ashima Arts / Stefan Gustavson, MIT), in [-1, 1];
// renamed so that it never clashes with a host shader's own noise.
vec3 starMod289(vec3 x) { return x - floor(x * (1.0 / 289.0)) * 289.0; }
vec4 starMod289(vec4 x) { return x - floor(x * (1.0 / 289.0)) * 289.0; }
vec4 starPermute(vec4 x) { return starMod289(((x * 34.0) + 10.0) * x); }
vec4 starTaylorInvSqrt(vec4 r) { return 1.79284291400159 - 0.85373472095314 * r; }
float starNoise(vec3 v) {
  const vec2 C = vec2(1.0 / 6.0, 1.0 / 3.0);
  const vec4 D = vec4(0.0, 0.5, 1.0, 2.0);
  vec3 i = floor(v + dot(v, C.yyy));
  vec3 x0 = v - i + dot(i, C.xxx);
  vec3 g = step(x0.yzx, x0.xyz);
  vec3 l = 1.0 - g;
  vec3 i1 = min(g.xyz, l.zxy);
  vec3 i2 = max(g.xyz, l.zxy);
  vec3 x1 = x0 - i1 + C.xxx;
  vec3 x2 = x0 - i2 + C.yyy;
  vec3 x3 = x0 - D.yyy;
  i = starMod289(i);
  vec4 p = starPermute(starPermute(starPermute(i.z + vec4(0.0, i1.z, i2.z, 1.0)) + i.y + vec4(0.0, i1.y, i2.y, 1.0)) + i.x + vec4(0.0, i1.x, i2.x, 1.0));
  float n_ = 0.142857142857;
  vec3 ns = n_ * D.wyz - D.xzx;
  vec4 j = p - 49.0 * floor(p * ns.z * ns.z);
  vec4 x_ = floor(j * ns.z);
  vec4 y_ = floor(j - 7.0 * x_);
  vec4 x = x_ * ns.x + ns.yyyy;
  vec4 y = y_ * ns.x + ns.yyyy;
  vec4 h = 1.0 - abs(x) - abs(y);
  vec4 b0 = vec4(x.xy, y.xy);
  vec4 b1 = vec4(x.zw, y.zw);
  vec4 s0 = floor(b0) * 2.0 + 1.0;
  vec4 s1 = floor(b1) * 2.0 + 1.0;
  vec4 sh = -step(h, vec4(0.0));
  vec4 a0 = b0.xzyw + s0.xzyw * sh.xxyy;
  vec4 a1 = b1.xzyw + s1.xzyw * sh.zzww;
  vec3 p0 = vec3(a0.xy, h.x);
  vec3 p1 = vec3(a0.zw, h.y);
  vec3 p2 = vec3(a1.xy, h.z);
  vec3 p3 = vec3(a1.zw, h.w);
  vec4 norm = starTaylorInvSqrt(vec4(dot(p0, p0), dot(p1, p1), dot(p2, p2), dot(p3, p3)));
  p0 *= norm.x; p1 *= norm.y; p2 *= norm.z; p3 *= norm.w;
  vec4 m = max(0.5 - vec4(dot(x0, x0), dot(x1, x1), dot(x2, x2), dot(x3, x3)), 0.0);
  m = m * m;
  return 105.0 * dot(m * m, vec4(dot(p0, x0), dot(p1, x1), dot(p2, x2), dot(p3, x3)));
}

/** Granulation at n, about zero-mean with unit contrast: two noise fields cross-faded over a lifetime. */
float starGranules(vec3 n) {
  float k = floor(uGranule.z);
  float w = smoothstep(0.0, 1.0, uGranule.z - k);
  vec3 p = n / uGranule.x;
  vec3 step3 = vec3(17.0, 31.0, 7.0);
  float g = mix(starNoise(p + mod(step3 * k, 289.0)), starNoise(p + mod(step3 * (k + 1.0), 289.0)), w);
  // Keep the contrast through the cross-fade (two independent fields).
  g /= sqrt((1.0 - w) * (1.0 - w) + w * w);
  return clamp(3.0 * g, -2.0, 2.0);
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
  if (uRegionFade > 0.0 && uRegionCount > 0) {
    float latitude = asin(clamp(n.y, -1.0, 1.0));
    // About a pixel on the sphere, to keep small spots from aliasing.
    float aa = length(fwidth(n));
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
 * uGranule (vec4: size, contrast, phase, fade), uRegionFade,
 * uRegionCount, uRegionA and uRegionB (vec4 each), uLocalDays, uOmegaEq,
 * uShear, uFacula.
 */
export function starSurfaceUniforms(maxRegions: number): Record<string, IUniform> {
  return {
    uStarColour: { value: new Vector3(1, 1, 1) },
    uLimb: { value: new Vector2(0.6, 0.2) },
    uGranule: { value: new Vector4(SUN_GRANULE_SIZE, 0, 0, 0) },
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
   * |days − epoch| > 20. Returns whether it repacked.
   */
  update(days: number, view: { radiusPx: number; daysPerSecond: number }): boolean {
    const look = this.look;
    if (!look) return false;
    const u = this.uniforms;
    const daysPerFrame = Math.abs(view.daysPerSecond) / 30;
    const detail = surfaceDetail(view.radiusPx, look.granuleSize);

    const granuleFade =
      look.granuleContrast > 0 ? detail.granulation * exposureFade(daysPerFrame, look.granuleLifetimeDays) : 0;
    const phase =
      look.granuleLifetimeDays > 0
        ? modulo(modulo(days, GRANULE_DAYS_WRAP) / look.granuleLifetimeDays, GRANULE_PHASE_PERIOD)
        : 0;
    (u['uGranule']!.value as Vector4).set(look.granuleSize, look.granuleContrast, phase, granuleFade);

    const rotationDays = TWO_PI / look.omegaEq;
    this.exposure = exposureFade(daysPerFrame, rotationDays / 12);
    this.sizeFade = detail.regions;
    const regionFade = detail.regions * this.exposure;
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
