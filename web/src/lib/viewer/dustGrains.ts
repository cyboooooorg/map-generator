// The halo of dust grains round a moon that vents or feeds a dust ring, in
// its own close-up: fine soft dots hugging the surface, denser over the
// venting pole, with the dust rings' tamed forward scattering (see
// dustRingGain), so they show front- and side-lit and only brighten a
// little back-lit. (In the giant's close-up the dust rings carry the main
// rings' grain instead, see dustRingFragment.)
import {
  AdditiveBlending,
  BufferAttribute,
  BufferGeometry,
  Color,
  Points,
  ShaderMaterial,
  SRGBColorSpace,
  type IUniform,
} from 'three';
import type { Vec3 } from '../geo';
import { dustPhaseGlsl, dustRingGain } from './dustPhase';
import { random } from './skyRandom';

/** Grains in the halo round a venting or ring-feeding moon in its close-up, at full quality (look calibration). */
export const HALO_GRAINS = 9000;

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

// The grains, each at its place. Depth held at the far plane like the dust
// rings', so the globe still hides them.
const grainVertex = /* glsl */ `
  attribute float aShine;
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
    float gain = 1.0;
    if (uLightMode > 0.5) {
      // cosScatter is 1 back-lit; the gain held between floor and cap (grainLight).
      gain = dustRingGain(-dot(uSunDir, normalize(cameraPosition - p)));
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

/**
 * Points for the grains at `positions` (xyz), with their `shine`, in the
 * sRGB `colour`, sharing the view's sun uniforms. Additive, behind the
 * body drawn (depth tested, not written).
 */
export function createGrains(
  positions: Float32Array,
  shine: Float32Array,
  colour: readonly [number, number, number],
  uniforms: { uSunDir: IUniform; uLightMode: IUniform },
): GrainPoints {
  const geometry = new BufferGeometry();
  geometry.setAttribute('position', new BufferAttribute(positions, 3));
  geometry.setAttribute('aShine', new BufferAttribute(shine, 1));
  const material = new ShaderMaterial({
    vertexShader: grainVertex,
    fragmentShader: grainFragment,
    uniforms: {
      uSunDir: uniforms.uSunDir,
      uLightMode: uniforms.uLightMode,
      uAlpha: { value: HALO_ALPHA },
      uFade: { value: 1 },
      uWorldSize: { value: GRAIN_WORLD_SIZE },
      uPxScale: { value: 500 },
      uPixelRatio: { value: 1 },
      uMinPx: { value: HALO_MIN_PX },
      uMaxPx: { value: HALO_MAX_PX },
      uCore: { value: HALO_CORE },
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
