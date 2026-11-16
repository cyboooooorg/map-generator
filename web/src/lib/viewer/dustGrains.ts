// Sparkling dust grains that make a faint dust ring and its moon readable
// at any phase: points along each dust ring a moon feeds (its radial
// profile), a cloud round the feeding moon trailing along its orbit (in
// the giant's close-up), and a halo round a moon that vents or feeds a
// ring (in its own close-up). The smooth ring sheet stays realistic (only
// bright back-lit, see dustPhase.ts); the grains follow the same forward
// scattering squeezed between a floor and a cap (see grainLight), so they
// show front- and side-lit and only brighten a few times back-lit.
import {
  AdditiveBlending,
  BufferAttribute,
  BufferGeometry,
  Color,
  Points,
  ShaderMaterial,
  SRGBColorSpace,
  Vector3,
  type IUniform,
} from 'three';
import type { Vec3 } from '../geo';
import type { FaintRing } from '../ringMoons';
import { DUST_SCALE_HEIGHT, dustGain, dustPhaseGlsl } from './dustPhase';
import { random } from './skyRandom';

/** Grains along one dust ring, at full quality (look calibration). */
export const RING_GRAINS = 2500;

/** Grains in the cloud round a ring's feeding moon, at full quality (look calibration). */
export const MOON_CLOUD_GRAINS = 1500;

/** Grains in the halo round a venting or ring-feeding moon in its close-up, at full quality (look calibration). */
export const HALO_GRAINS = 4000;

/**
 * Share of the cloud's grains packed close round the moon, the others
 * trailing ahead and behind it along its orbit (look calibration).
 */
export const MOON_CORE_SHARE = 0.4;

/** Sigma of the cloud's core round the moon, in its orbit's radii (look calibration). */
export const MOON_CORE_SIGMA = 0.012;

/**
 * Scale, radians, of the cloud's trail along the orbit, ahead and behind
 * the moon (a Laplace law: half the trail within 0.69 of it; look
 * calibration, as the E ring's fresh grains spread from Enceladus).
 */
export const MOON_TRAIL_RAD = 0.3;

/** Sigma of the trail's spread across the orbit, in its radii (look calibration). */
export const MOON_TRAIL_SIGMA = 0.025;

/** The halo's inner and outer radii, in the moon's radii (look calibration). */
export const HALO_INNER = 1.05;
export const HALO_OUTER = 3;

/**
 * Scale, in the moon's radii, of the halo's exponential fall over its
 * surface (look calibration): most grains hug the surface, so the halo
 * reads as dust round the moon rather than stars behind it.
 */
export const HALO_SCALE = 0.25;

/** Share of the halo's grains over the venting pole, when known (look calibration). */
export const HALO_POLE_SHARE = 0.55;

/** Half-angle, radians, of the cone over the venting pole the polar grains fill (look calibration). */
export const HALO_POLE_HALF_ANGLE = (35 * Math.PI) / 180;

/**
 * Floor under the grains' forward-scattering gain (look calibration): a
 * grain is never dimmer than at 90° of scattering, so the grains stay
 * seen front- and side-lit where the smooth ring vanishes.
 */
export const GRAIN_PHASE_FLOOR = 1;

/**
 * Cap on the grains' forward-scattering gain (look calibration): back-lit
 * they shine at most this many times their 90° light, not the smooth
 * sheet's tens of times (`DUST_GAIN_MAX`), so they sparkle without washing
 * the frame.
 */
export const GRAIN_GAIN_MAX = 4;

/**
 * A grain's opacity at 90° of scattering and full shine (look
 * calibration): with `GRAIN_SHINE_MIN` and the soft dot's core, a grain at
 * the floor reads at a luminance of about 120 or more on its pixel or two.
 */
export const GRAIN_ALPHA = 0.95;

/** Least shine of a grain, of 1 for the brightest (look calibration): few faint ones, never lost. */
export const GRAIN_SHINE_MIN = 0.55;

/** A grain's size, in the drawn body's radii, before the pixel floor and cap (look calibration). */
export const GRAIN_WORLD_SIZE = 0.008;

/**
 * Least and most a ring grain is drawn across, CSS pixels, before light
 * past full opacity widens it (see grainLight): fine sparkles at any
 * distance (look calibration).
 */
export const GRAIN_MIN_PX = 2;
export const GRAIN_MAX_PX = 3;

/**
 * Least and most a halo grain is drawn across, CSS pixels (look
 * calibration): slightly larger soft dots than the ring's, so the halo
 * reads as dust round the moon, not as stars.
 */
export const HALO_MIN_PX = 2.5;
export const HALO_MAX_PX = 4;

/**
 * A halo grain's opacity at 90° of scattering and full shine (look
 * calibration): under the ring grains', so the denser halo stays a haze of
 * dots and the jets show through it.
 */
export const HALO_ALPHA = 0.6;

/**
 * Inner edge of a grain's soft dot, as a share of its size (look
 * calibration): a flat core out to it, then a smooth fall to the rim. The
 * halo's softer dots start falling sooner.
 */
export const GRAIN_CORE = 0.3;
export const HALO_CORE = 0.1;

/**
 * The light of a grain at `cosScatter` (1 back-lit), with its `shine` and
 * `alpha` at 90° (`GRAIN_ALPHA`, `HALO_ALPHA`): its opacity, and how much
 * its size grows. The phase gain (dustGain) is held between
 * `GRAIN_PHASE_FLOOR` and `GRAIN_GAIN_MAX`; light past full opacity widens
 * the dot instead (area grows with it), so a back-lit grain is up to
 * `GRAIN_GAIN_MAX` times brighter in all without saturating. Twin of the
 * grain shader's code.
 */
export function grainLight(
  cosScatter: number,
  shine: number,
  alpha = GRAIN_ALPHA,
): { opacity: number; sizeScale: number } {
  const gain = Math.min(Math.max(dustGain(cosScatter), GRAIN_PHASE_FLOOR), GRAIN_GAIN_MAX);
  const light = alpha * shine * gain;
  return { opacity: Math.min(light, 1), sizeScale: Math.sqrt(Math.max(light, 1)) };
}

/** A standard normal deviate (Box-Muller) from `rand`. */
function gauss(rand: () => number): number {
  const u = Math.max(rand(), 1e-12);
  return Math.sqrt(-2 * Math.log(u)) * Math.cos(2 * Math.PI * rand());
}

/** The dust ring shader's radial profile at `r`: 1 at the peak, 0 at either edge (see dustRingFragment). */
export function ringProfile(ring: Pick<FaintRing, 'inner' | 'outer' | 'peak'>, r: number): number {
  if (r < ring.inner || r > ring.outer) return 0;
  const side =
    r < ring.peak
      ? (ring.peak - r) / Math.max(ring.peak - ring.inner, 1e-4)
      : (r - ring.peak) / Math.max(ring.outer - ring.peak, 1e-4);
  const t = Math.min(Math.max(side, 0), 1);
  return 1 - t * t * (3 - 2 * t);
}

/**
 * `count` grains along `ring` (giant radii, y its axis, the ring in the
 * xz plane), xyz each: as many per unit area as the ring's profile
 * (`ringProfile`, so the radius is drawn by profile x r), at any angle,
 * and spread over the plane as a Gaussian of sigma `DUST_SCALE_HEIGHT` x r.
 */
export function ringGrains(
  ring: Pick<FaintRing, 'inner' | 'outer' | 'peak'>,
  count: number,
  seed: number,
): Float32Array {
  const rand = random(seed);
  const out = new Float32Array(3 * count);
  const width = ring.outer - ring.inner;
  for (let i = 0; i < count; i += 1) {
    let r = ring.peak;
    // Rejection: at most a few dozen draws, else the peak.
    for (let tries = 0; tries < 64; tries += 1) {
      const candidate = ring.inner + rand() * width;
      if (rand() * ring.outer <= ringProfile(ring, candidate) * candidate) {
        r = candidate;
        break;
      }
    }
    const angle = 2 * Math.PI * rand();
    out[3 * i] = r * Math.cos(angle);
    out[3 * i + 1] = DUST_SCALE_HEIGHT * r * gauss(rand);
    out[3 * i + 2] = r * Math.sin(angle);
  }
  return out;
}

/**
 * `count` grains round a moon on its orbit, as offsets from it in the
 * orbit's own terms, three each: the angle along the orbit (radians),
 * then the radial and vertical offsets in the orbit's radii. A core of
 * `MOON_CORE_SHARE` of them close round the moon, the others trailing
 * ahead and behind along the orbit, ever fewer away from it.
 */
export function moonCloudGrains(count: number, seed: number): Float32Array {
  const rand = random(seed);
  const out = new Float32Array(3 * count);
  for (let i = 0; i < count; i += 1) {
    if (rand() < MOON_CORE_SHARE) {
      out[3 * i] = MOON_CORE_SIGMA * gauss(rand);
      out[3 * i + 1] = MOON_CORE_SIGMA * gauss(rand);
      out[3 * i + 2] = MOON_CORE_SIGMA * gauss(rand);
    } else {
      const sign = rand() < 0.5 ? -1 : 1;
      out[3 * i] = sign * -MOON_TRAIL_RAD * Math.log(Math.max(rand(), 1e-12));
      out[3 * i + 1] = MOON_TRAIL_SIGMA * gauss(rand);
      out[3 * i + 2] = DUST_SCALE_HEIGHT * gauss(rand);
    }
  }
  return out;
}

/**
 * `count` grains round a moon in its close-up (its radii, at its centre),
 * xyz each, between `HALO_INNER` and `HALO_OUTER`: denser near the
 * surface (an exponential fall of scale `HALO_SCALE`), and with
 * `HALO_POLE_SHARE` of them in a cone over `pole` (unit, the venting
 * pole) when given, the others in every direction.
 */
export function haloGrains(count: number, seed: number, pole: Vec3 | null): Float32Array {
  const rand = random(seed);
  const out = new Float32Array(3 * count);
  // A frame round the pole for the polar cone.
  const p = pole ?? ([0, 1, 0] as const);
  const helper: Vec3 = Math.abs(p[1]) < 0.9 ? [0, 1, 0] : [1, 0, 0];
  const e1 = normalise(cross(p, helper));
  const e2 = cross(p, e1);
  const span = HALO_OUTER - HALO_INNER;
  const fall = 1 - Math.exp(-span / HALO_SCALE);
  for (let i = 0; i < count; i += 1) {
    let dir: Vec3;
    if (pole && rand() < HALO_POLE_SHARE) {
      // Uniform over the cap of half-angle HALO_POLE_HALF_ANGLE.
      const cosT = 1 - rand() * (1 - Math.cos(HALO_POLE_HALF_ANGLE));
      const sinT = Math.sqrt(Math.max(0, 1 - cosT * cosT));
      const phi = 2 * Math.PI * rand();
      const [a, b] = [sinT * Math.cos(phi), sinT * Math.sin(phi)];
      dir = [0, 1, 2].map((k) => p[k]! * cosT + e1[k]! * a + e2[k]! * b) as unknown as Vec3;
    } else {
      const z = 2 * rand() - 1;
      const s = Math.sqrt(1 - z * z);
      const phi = 2 * Math.PI * rand();
      dir = [s * Math.cos(phi), z, s * Math.sin(phi)];
    }
    // Exponential in height, truncated at the outer radius.
    const r = HALO_INNER - HALO_SCALE * Math.log(1 - rand() * fall);
    out.set([dir[0] * r, dir[1] * r, dir[2] * r], 3 * i);
  }
  return out;
}

/** Each grain's own shine, from `GRAIN_SHINE_MIN` to 1 (most of them fainter, a few bright): sparkle, not a haze. */
export function grainShine(count: number, seed: number): Float32Array {
  const rand = random(seed);
  const out = new Float32Array(count);
  for (let i = 0; i < count; i += 1) out[i] = GRAIN_SHINE_MIN + (1 - GRAIN_SHINE_MIN) * rand() ** 2;
  return out;
}

/** The number of grains drawn for `count` at `share` of full quality (at least one). */
export function grainCount(count: number, share: number): number {
  return Math.max(1, Math.round(count * share));
}

function cross(a: Vec3, b: Vec3): Vec3 {
  return [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
}

function normalise(v: Vec3): Vec3 {
  const n = Math.hypot(...v) || 1;
  return [v[0] / n, v[1] / n, v[2] / n];
}

// The grains: `position` holds each one's place (uOrbit 0), or its offset
// from the moon along its orbit (uOrbit 1, see moonCloudGrains), placed
// round uMoon. Depth held at the far plane like the dust rings', so the
// globe still hides them.
const grainVertex = /* glsl */ `
  attribute float aShine;
  uniform float uOrbit;
  uniform vec3 uMoon;
  uniform vec3 uSunDir;
  uniform float uLightMode;
  uniform float uAlpha;
  uniform float uFloor;
  uniform float uGainMax;
  uniform float uFade;
  uniform float uWorldSize;
  uniform float uPxScale;
  uniform float uPixelRatio;
  uniform float uMinPx;
  uniform float uMaxPx;
  varying float vAlpha;
  ${dustPhaseGlsl}
  void main() {
    vec3 p = position;
    if (uOrbit > 0.5) {
      float a = length(uMoon.xz);
      float angle = atan(uMoon.z, uMoon.x) + position.x;
      float r = a * (1.0 + position.y);
      p = vec3(r * cos(angle), uMoon.y + a * position.z, r * sin(angle));
    }
    float gain = 1.0;
    if (uLightMode > 0.5) {
      // cosScatter is 1 back-lit; the gain held between floor and cap (grainLight).
      gain = clamp(dustGain(-dot(uSunDir, normalize(cameraPosition - p))), uFloor, uGainMax);
      // Nothing to scatter in the shadow of the body drawn (radius 1 at the origin).
      float b = dot(p, uSunDir);
      if (b < 0.0 && dot(p, p) - b * b < 1.0) gain = 0.0;
    }
    // Light past full opacity widens the dot instead (its area grows with it).
    float light = uAlpha * aShine * gain;
    vAlpha = clamp(light, 0.0, 1.0) * uFade;
    vec4 view = viewMatrix * vec4(p, 1.0);
    float px = uWorldSize * uPxScale / max(-view.z, 1e-6);
    gl_PointSize = clamp(px, uMinPx, uMaxPx) * sqrt(max(light, 1.0)) * uPixelRatio;
    gl_Position = projectionMatrix * view;
    gl_Position.z = min(gl_Position.z, gl_Position.w * 0.99999);
  }
`;

export const grainFragment = /* glsl */ `
  uniform vec3 uColour;
  uniform float uCore;
  varying float vAlpha;
  void main() {
    // A soft round dot: flat out to uCore of its size, then falling to the rim.
    vec2 d = gl_PointCoord - 0.5;
    float fall = 1.0 - smoothstep(uCore, 0.5, length(d));
    if (fall <= 0.0 || vAlpha <= 0.0) discard;
    gl_FragColor = vec4(uColour, vAlpha * fall);
    #include <colorspace_fragment>
  }
`;

/** Points of dust grains (see createGrains). */
export type GrainPoints = Points<BufferGeometry, ShaderMaterial>;

/** How a set of grains is drawn: the ring's fine sparkles, or the halo's softer, larger dots. */
export interface GrainLook {
  alpha: number;
  minPx: number;
  maxPx: number;
  core: number;
}

/** The ring and moon cloud grains' look. */
export const RING_LOOK: GrainLook = { alpha: GRAIN_ALPHA, minPx: GRAIN_MIN_PX, maxPx: GRAIN_MAX_PX, core: GRAIN_CORE };

/** The halo grains' look, round a moon in its close-up. */
export const HALO_LOOK: GrainLook = { alpha: HALO_ALPHA, minPx: HALO_MIN_PX, maxPx: HALO_MAX_PX, core: HALO_CORE };

/**
 * Points for the grains at `positions` (xyz, or orbit offsets when
 * `orbit`), with their `shine`, in the sRGB `colour`, sharing the view's
 * sun uniforms, drawn with `look`. Additive, behind the body drawn (depth
 * tested, not written).
 */
export function createGrains(
  positions: Float32Array,
  shine: Float32Array,
  colour: readonly [number, number, number],
  orbit: boolean,
  uniforms: { uSunDir: IUniform; uLightMode: IUniform },
  look: GrainLook = RING_LOOK,
): GrainPoints {
  const geometry = new BufferGeometry();
  geometry.setAttribute('position', new BufferAttribute(positions, 3));
  geometry.setAttribute('aShine', new BufferAttribute(shine, 1));
  const material = new ShaderMaterial({
    vertexShader: grainVertex,
    fragmentShader: grainFragment,
    uniforms: {
      uOrbit: { value: orbit ? 1 : 0 },
      uMoon: { value: new Vector3() },
      uSunDir: uniforms.uSunDir,
      uLightMode: uniforms.uLightMode,
      uAlpha: { value: look.alpha },
      uFloor: { value: GRAIN_PHASE_FLOOR },
      uGainMax: { value: GRAIN_GAIN_MAX },
      uFade: { value: 1 },
      uWorldSize: { value: GRAIN_WORLD_SIZE },
      uPxScale: { value: 500 },
      uPixelRatio: { value: 1 },
      uMinPx: { value: look.minPx },
      uMaxPx: { value: look.maxPx },
      uCore: { value: look.core },
      uColour: { value: new Color().setRGB(...colour, SRGBColorSpace) },
    },
    transparent: true,
    depthWrite: false,
    blending: AdditiveBlending,
  });
  const points = new Points(geometry, material);
  points.frustumCulled = false;
  points.visible = false;
  points.renderOrder = 2;
  return points;
}
