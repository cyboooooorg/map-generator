// Dust grains as fine soft dots, with the dust rings' tamed forward
// scattering (see dustRingGain), so they show front- and side-lit and only
// brighten a little back-lit: the halo round a moon that vents or feeds a
// dust ring, in its own close-up, hugging the surface and denser over the
// venting pole; and a sparse cloud round the moon feeding a dust ring,
// thinning along its orbit ahead and behind it, the same in the giant's
// close-up and in its moons' sky. (The dust ring itself carries the main
// rings' grain, see dustRingFragment.)
import {
  AdditiveBlending,
  BufferAttribute,
  BufferGeometry,
  Color,
  Matrix3,
  NormalBlending,
  Points,
  ShaderMaterial,
  SRGBColorSpace,
  Vector3,
  type IUniform,
} from 'three';
import type { Vec3 } from '../geo';
import { DUST_SCALE_HEIGHT, dustPhaseGlsl, dustRingGain } from './dustPhase';
import { random } from './skyRandom';

/** Grains in the halo round a venting or ring-feeding moon in its close-up, at full quality (look calibration). */
export const HALO_GRAINS = 9000;

/**
 * Grains in the cloud round the moon feeding a dust ring, in the giant's
 * close-up and its moons' sky, at full quality (look calibration): a few hundred, a soft
 * concentration round the moon, not a field of dots over the ring.
 */
export const MOON_CLOUD_GRAINS = 320;

/** Share of the cloud's grains packed close round the moon, the others along its orbit (look calibration). */
export const MOON_CORE_SHARE = 0.4;

/** Sigma of the cloud's core round the moon, in its orbit's radii (look calibration). */
export const MOON_CORE_SIGMA = 0.012;

/**
 * Scale, radians, of the cloud's exponential fall along the orbit ahead
 * and behind the moon (look calibration, as the E ring's fresh grains
 * spread from Enceladus): its density falls e times every 25°.
 */
export const MOON_TRAIL_RAD = (25 * Math.PI) / 180;

/**
 * Sigma of the cloud's spread across the orbit by the moon, in its radii,
 * widening with the angle from it (look calibration): the grains spread
 * as they drift away.
 */
export const MOON_TRAIL_SIGMA = 0.012;

/**
 * A cloud grain's opacity at 90° of scattering and full shine (look
 * calibration): low, so that the cloud reads as a soft concentration.
 */
export const CLOUD_ALPHA = 0.65;

/**
 * The brightest a pixel of the cloud gets, as a share of the ring's colour
 * (look calibration): its grains blend over the scene (not additively),
 * so however many overlap, the pixel tends to this and never saturates.
 */
export const CLOUD_LEVEL = 0.75;

/**
 * Radius, in the drawn body's radii, within which the moon cloud is not
 * drawn (look calibration): in the feeding moon's own sky the cloud
 * surrounds it, and its halo (`HALO_INNER` out) stands in for the grains
 * at its surface.
 */
export const CLOUD_SKIP = 1.1;

/** Least and most a cloud grain is drawn across, CSS pixels (look calibration). */
export const CLOUD_MIN_PX = 1;
export const CLOUD_MAX_PX = 1.5;

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

/** Least shine of a grain, of 1 for the brightest (look calibration): few faint ones, never lost. */
export const GRAIN_SHINE_MIN = 0.55;

/** A grain's size, in the drawn body's radii, before the pixel floor and cap (look calibration). */
export const GRAIN_WORLD_SIZE = 0.008;

/**
 * Least and most a halo grain is drawn across, CSS pixels, before light
 * past full opacity widens it (look calibration): fine dots, many of them
 * (`HALO_GRAINS`), so the halo reads as dust round the moon, not as stars
 * or blobs.
 */
export const HALO_MIN_PX = 1;
export const HALO_MAX_PX = 2;

/**
 * A halo grain's opacity at 90° of scattering and full shine (look
 * calibration): the dense halo stays a haze of dots the jets show through.
 */
export const HALO_ALPHA = 0.6;

/**
 * Inner edge of a grain's soft dot, as a share of its size (look
 * calibration): a flat core out to it, then a smooth fall to the rim.
 */
export const HALO_CORE = 0.2;

/**
 * The light of a grain at `cosScatter` (1 back-lit), with its `shine` and
 * `alpha` at 90° (`HALO_ALPHA`): its opacity, and how much its size grows.
 * The phase gain is the dust rings' (`dustRingGain`, held between a floor
 * and `DUST_RING_GAIN_MAX`); light past full opacity widens the dot instead
 * (area grows with it), so a back-lit grain is up to `DUST_RING_GAIN_MAX`
 * times brighter in all without saturating. Twin of the grain shader's code.
 */
export function grainLight(
  cosScatter: number,
  shine: number,
  alpha = HALO_ALPHA,
): { opacity: number; sizeScale: number } {
  const light = alpha * shine * dustRingGain(cosScatter);
  return { opacity: Math.min(light, 1), sizeScale: Math.sqrt(Math.max(light, 1)) };
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

/** A standard normal deviate (Box-Muller) from `rand`. */
function gauss(rand: () => number): number {
  const u = Math.max(rand(), 1e-12);
  return Math.sqrt(-2 * Math.log(u)) * Math.cos(2 * Math.PI * rand());
}

/**
 * `count` grains of the cloud round a dust ring's feeding moon, as offsets
 * from it, xyz each: the angle along its orbit (radians, ahead positive),
 * the radial offset and the height over the orbit's plane (both in the
 * orbit's radii); the grain shader places them round the moon each frame.
 * `MOON_CORE_SHARE` of them close round the moon (Gaussian of sigma
 * `MOON_CORE_SIGMA`), the others ahead or behind it along the orbit, as
 * many each way, falling exponentially with the angle (scale
 * `MOON_TRAIL_RAD`) and as a Gaussian off the orbit (sigma
 * `MOON_TRAIL_SIGMA` widening with the angle, and `DUST_SCALE_HEIGHT` in
 * height).
 */
export function moonCloudGrains(count: number, seed: number): Float32Array {
  const rand = random(seed);
  const out = new Float32Array(3 * count);
  for (let i = 0; i < count; i += 1) {
    if (rand() < MOON_CORE_SHARE) {
      out.set([MOON_CORE_SIGMA * gauss(rand), MOON_CORE_SIGMA * gauss(rand), MOON_CORE_SIGMA * gauss(rand)], 3 * i);
    } else {
      const sign = rand() < 0.5 ? -1 : 1;
      const along = -MOON_TRAIL_RAD * Math.log(Math.max(rand(), 1e-12));
      const widen = 1 + along / MOON_TRAIL_RAD;
      out.set([sign * along, MOON_TRAIL_SIGMA * widen * gauss(rand), DUST_SCALE_HEIGHT * widen * gauss(rand)], 3 * i);
    }
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

/** A giant's axes (its x, spin axis, z, in the scene's frame), as a `Matrix3.set` row-major list taking its frame to the scene's. */
export function hostAxesMatrix(
  axes: readonly [Vec3, Vec3, Vec3],
): [number, number, number, number, number, number, number, number, number] {
  const [x, y, z] = axes;
  return [x[0], y[0], z[0], x[1], y[1], z[1], x[2], y[2], z[2]];
}

/**
 * `point` (scene units) in the frame of a giant at `centre` of radius
 * `scale` (scene units) with `axes`: its radii, its spin axis y. Where the
 * grain shader takes the feeding moon (`uMoon`); undone there by
 * `uHostCentre + uHostScale * uHostAxes * p`.
 */
export function toHostFrame(
  point: readonly [number, number, number],
  centre: readonly [number, number, number],
  scale: number,
  axes: readonly [Vec3, Vec3, Vec3],
): Vec3 {
  const d: Vec3 = [(point[0] - centre[0]) / scale, (point[1] - centre[1]) / scale, (point[2] - centre[2]) / scale];
  return axes.map((a) => a[0] * d[0] + a[1] * d[1] + a[2] * d[2]) as unknown as Vec3;
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
// round uMoon in the giant's frame (its radii, its spin axis y, the orbit
// taken as a circle in its equatorial plane, xz), then into the scene by
// the giant's centre, radius and axes there (uHost*): the same cloud in
// the giant's close-up (identity) and in its moons' sky. There, none
// within uSkip of the body shown (radius 1 at the origin): round the
// feeding moon in its own sky, its halo stands in for them. Depth held at
// the far plane like the dust rings', so the bodies still hide them.
const grainVertex = /* glsl */ `
  attribute float aShine;
  uniform float uOrbit;
  uniform vec3 uMoon;
  uniform vec3 uHostCentre;
  uniform float uHostScale;
  uniform mat3 uHostAxes;
  uniform float uSkip;
  uniform vec3 uSunDir;
  uniform float uLightMode;
  uniform float uAlpha;
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
    // Its shadow: the giant's round a cloud, the moon's round its halo.
    vec3 caster = vec3(0.0);
    float casterRadius = 1.0;
    float size = uWorldSize;
    if (uOrbit > 0.5) {
      float a = length(uMoon.xz);
      float angle = atan(uMoon.z, uMoon.x) + position.x;
      float r = a * (1.0 + position.y);
      p = uHostCentre + uHostScale * (uHostAxes * vec3(r * cos(angle), uMoon.y + a * position.z, r * sin(angle)));
      caster = uHostCentre;
      casterRadius = uHostScale;
      size *= uHostScale;
      if (length(p) < uSkip) {
        gl_PointSize = 0.0;
        gl_Position = vec4(2.0, 2.0, 2.0, 1.0);
        vAlpha = 0.0;
        return;
      }
    }
    float gain = 1.0;
    if (uLightMode > 0.5) {
      // cosScatter is 1 back-lit; the gain held between floor and cap (grainLight).
      gain = dustRingGain(-dot(uSunDir, normalize(cameraPosition - p)));
      // Nothing to scatter in the shadow.
      vec3 c = p - caster;
      float b = dot(c, uSunDir);
      if (b < 0.0 && dot(c, c) - b * b < casterRadius * casterRadius) gain = 0.0;
    }
    // Light past full opacity widens the dot instead (its area grows with it).
    float light = uAlpha * aShine * gain;
    vAlpha = clamp(light, 0.0, 1.0) * uFade;
    vec4 view = viewMatrix * vec4(p, 1.0);
    float px = size * uPxScale / max(-view.z, 1e-6);
    gl_PointSize = clamp(px, uMinPx, uMaxPx) * sqrt(max(light, 1.0)) * uPixelRatio;
    gl_Position = projectionMatrix * view;
    gl_Position.z = min(gl_Position.z, gl_Position.w * 0.99999);
  }
`;

export const grainFragment = /* glsl */ `
  uniform vec3 uColour;
  uniform float uLevel;
  uniform float uCore;
  varying float vAlpha;
  void main() {
    // A soft round dot: flat out to uCore of its size, then falling to the rim.
    vec2 d = gl_PointCoord - 0.5;
    float fall = 1.0 - smoothstep(uCore, 0.5, length(d));
    if (fall <= 0.0 || vAlpha <= 0.0) discard;
    gl_FragColor = vec4(uColour * uLevel, vAlpha * fall);
    #include <colorspace_fragment>
  }
`;

/** Points of dust grains (see createGrains). */
export type GrainPoints = Points<BufferGeometry, ShaderMaterial>;

/**
 * How a set of grains is drawn: opacity at 90° (`alpha`), size range, soft
 * core, colour `level`, and whether they add up (the halo) or blend over
 * the scene, never past `level` of their colour (the moon cloud).
 */
export interface GrainLook {
  alpha: number;
  minPx: number;
  maxPx: number;
  core: number;
  level: number;
  additive: boolean;
}

/** The halo's look, round a moon in its close-up. */
export const HALO_LOOK: GrainLook = {
  alpha: HALO_ALPHA,
  minPx: HALO_MIN_PX,
  maxPx: HALO_MAX_PX,
  core: HALO_CORE,
  level: 1,
  additive: true,
};

/** The moon cloud's look, in the giant's close-up and its moons' sky. */
export const CLOUD_LOOK: GrainLook = {
  alpha: CLOUD_ALPHA,
  minPx: CLOUD_MIN_PX,
  maxPx: CLOUD_MAX_PX,
  core: HALO_CORE,
  level: CLOUD_LEVEL,
  additive: false,
};

/**
 * Points for the grains at `positions` (xyz, or offsets along the orbit
 * of the moon at uniform `uMoon` when `orbit`, see moonCloudGrains, in the
 * frame of the giant at uniforms `uHostCentre`, `uHostScale`, `uHostAxes`), with
 * their `shine`, in the sRGB `colour`, sharing the view's sun uniforms,
 * drawn with `look`. Behind the body drawn (depth tested, not written).
 */
export function createGrains(
  positions: Float32Array,
  shine: Float32Array,
  colour: readonly [number, number, number],
  uniforms: { uSunDir: IUniform; uLightMode: IUniform },
  look: GrainLook = HALO_LOOK,
  orbit = false,
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
      uHostCentre: { value: new Vector3() },
      uHostScale: { value: 1 },
      uHostAxes: { value: new Matrix3() },
      uSkip: { value: CLOUD_SKIP },
      uSunDir: uniforms.uSunDir,
      uLightMode: uniforms.uLightMode,
      uAlpha: { value: look.alpha },
      uFade: { value: 1 },
      uWorldSize: { value: GRAIN_WORLD_SIZE },
      uPxScale: { value: 500 },
      uPixelRatio: { value: 1 },
      uMinPx: { value: look.minPx },
      uMaxPx: { value: look.maxPx },
      uCore: { value: look.core },
      uLevel: { value: look.level },
      uColour: { value: new Color().setRGB(...colour, SRGBColorSpace) },
    },
    transparent: true,
    depthWrite: false,
    blending: look.additive ? AdditiveBlending : NormalBlending,
  });
  const points = new Points(geometry, material);
  points.frustumCulled = false;
  points.visible = false;
  points.renderOrder = 2;
  return points;
}
