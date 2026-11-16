// Jets rising from a moon's geyser fractures in its close-up (the tiger
// stripes of Enceladus): one camera-facing cone per source, its apex on
// the surface along each fracture, widening with height. Seen down its
// axis, a cone turns into a round glow brightest at the vent (see
// jetVertex). The plume in a moon's sky is plume.ts.
import {
  AdditiveBlending,
  Float32BufferAttribute,
  InstancedBufferAttribute,
  InstancedBufferGeometry,
  Mesh,
  ShaderMaterial,
  type IUniform,
  type Vector3,
} from 'three';
import type { Fracture } from '../api/types/Fracture';
import type { Geysers } from '../api/types/Geysers';
import type { LonLat } from '../geo';
import { dustPhaseGlsl } from './dustPhase';

/** Jets along each fracture: about a hundred over Enceladus's four stripes (Porco et al. 2014, doi:10.1088/0004-6256/148/3/45). */
export const JETS_PER_FRACTURE = 24;

/** Height of a jet in the body's radii (look constant). */
export const JET_HEIGHT_RADII = 0.3;

/**
 * Opening half-angle of a jet, degrees: each jet is a cone with its apex
 * on the vent, opening by roughly 10 to 20° (Porco et al. 2014,
 * doi:10.1088/0004-6256/148/3/45).
 */
export const JET_HALF_ANGLE_DEG = 15;

/** Half-width of a jet at its top, in the body's radii. */
export const JET_TOP_HALF_WIDTH = JET_HEIGHT_RADII * Math.tan((JET_HALF_ANGLE_DEG * Math.PI) / 180);

/** Width of a jet at its foot, as a share of its top width: a point-like vent (look constant). */
export const JET_APEX_SHARE = 0.05;

/**
 * Share of the top width under which a jet's brightness per unit area
 * stops rising towards the vent (look constant): the gas thins as the
 * cone widens, but a vent brighter than this would read as a hot spot.
 */
export const JET_CORE_SHARE = 0.25;

/**
 * Brightness per unit area of a jet at its full width, seen at 90° of
 * scattering, before the phase gain and its fade with height (look
 * constant): `JET_OPACITY / JET_CORE_SHARE`, about 0.16, at the vent.
 */
export const JET_OPACITY = 0.04;

/**
 * How end-on a jet is seen, |cos| of the angle between its axis and the
 * view, over which it turns from the side-on trapezoid into the end-on
 * glow (look constant): side-on below the first, a glow past the second.
 */
export const JET_END_ON_FROM = 0.75;
export const JET_END_ON_TO = 0.97;

/**
 * Least sine of the angle between a jet's axis and the view by which its
 * side-on trapezoid is brightened (look constant): the gas is thin, so a
 * jet seen at a slant packs the same light into a shorter shape; this
 * keeps that boost finite as the trapezoid fades out.
 */
export const JET_MIN_SLANT = 0.25;

/**
 * The light of one jet's side-on trapezoid at full brightness per unit
 * area, in its top half-widths times its heights: the integral of the
 * fragment's side and height profiles (with `JET_APEX_SHARE` and
 * `JET_CORE_SHARE`) over the trapezoid.
 */
export function jetSideLight(steps = 2000): number {
  let sum = 0;
  for (let i = 0; i < steps; i += 1) {
    const y = (i + 0.5) / steps;
    const share = JET_APEX_SHARE + (1 - JET_APEX_SHARE) * y;
    // Across: the side profile 1 − smoothstep integrates to `share` (of the top half-width, on both sides).
    sum += (share * Math.pow(1 - y, 1.5)) / Math.max(share, JET_CORE_SHARE);
  }
  return sum / steps;
}

/**
 * Brightness per unit area at the centre of a jet's end-on glow, a disc
 * of the top half-width falling as (1 − ρ)² (π/6 of its radius squared in
 * all), so that it holds the same light as the side-on trapezoid.
 */
export const JET_GLOW_OPACITY =
  (JET_OPACITY * JET_HEIGHT_RADII * JET_TOP_HALF_WIDTH * jetSideLight()) / ((Math.PI / 6) * JET_TOP_HALF_WIDTH ** 2);

/**
 * The weights of a jet's side-on trapezoid and end-on glow when seen
 * `endOn` (|cos| of the angle between its axis and the view): the glow
 * fades in from `JET_END_ON_FROM` to `JET_END_ON_TO`, the trapezoid out,
 * brightened by 1 / sin of that angle (at least `JET_MIN_SLANT`) as it
 * shortens. Twin of jetVertex.
 */
export function jetWeights(endOn: number): { side: number; glow: number } {
  const t = Math.min(Math.max((endOn - JET_END_ON_FROM) / (JET_END_ON_TO - JET_END_ON_FROM), 0), 1);
  const glow = t * t * (3 - 2 * t);
  return { side: (1 - glow) / Math.max(Math.sqrt(1 - endOn * endOn), JET_MIN_SLANT), glow };
}

/** Golden angle, radians: steps the phases of `meanderKm` from one fracture to the next. */
const GOLDEN_ANGLE = 2.399963229728653;

/**
 * Signed sideways offset, km, of the drawn centre line of the fracture
 * number `index` (its position in `Geysers.fractures`, zero-length ones
 * included) from its great circle, at `along` its length from its centre
 * (−0.5 and 0.5 at its ends), positive on the side of centre × strike.
 * Twin of `meander_km` in crates/mapgen/src/planet/geysers.rs, which
 * explains the look calibration (a wave, a bow, hooked ends); keep both
 * in step (the same values are pinned in both tests).
 */
export function meanderKm(index: number, along: number, lengthKm: number): number {
  const k = index;
  const amp = Math.min(5, 0.06 * lengthKm);
  const wave =
    amp *
    (0.65 * Math.sin(2 * Math.PI * 0.9 * along + k * GOLDEN_ANGLE) +
      0.35 * Math.sin(2 * Math.PI * 2.1 * along + 1.7 + k * GOLDEN_ANGLE * 1.3));
  const bow = Math.min(4, 0.05 * lengthKm) * Math.cos(k * GOLDEN_ANGLE * 0.7) * (1 - 4 * along * along);
  const u = Math.max(0, Math.abs(along) - 0.38) / 0.12;
  const hook = Math.min(8, 0.08 * lengthKm) * u * u * (along > 0 ? 1 : -1) * (index % 2 === 0 ? 1 : -1);
  return wave + bow + hook;
}

/**
 * `n` points evenly along the fracture number `index` of its body, its
 * ends excluded. Same great circle as `Fracture::ends` in
 * crates/mapgen/src/planet/geysers.rs, each point moved sideways by
 * `meanderKm` as the stripes are, computed in that frame (z the pole,
 * longitude 0 on x) and returned as latitude and longitude in radians,
 * which the viewer uses with its own axes.
 */
export function fracturePoints(f: Fracture, index: number, radiusKm: number, n: number): LonLat[] {
  const lat = (f.lat_deg * Math.PI) / 180;
  const lon = (f.lon_deg * Math.PI) / 180;
  const centre = [Math.cos(lat) * Math.cos(lon), Math.cos(lat) * Math.sin(lon), Math.sin(lat)] as const;
  // North, tangent at the centre; at a pole, along the meridian `lon`.
  const north =
    Math.abs(f.lat_deg) >= 90
      ? ([Math.cos(lon), Math.sin(lon), 0] as const)
      : ([-Math.sin(lat) * Math.cos(lon), -Math.sin(lat) * Math.sin(lon), Math.cos(lat)] as const);
  const east = [
    north[1] * centre[2] - north[2] * centre[1],
    north[2] * centre[0] - north[0] * centre[2],
    north[0] * centre[1] - north[1] * centre[0],
  ] as const;
  const az = (f.azimuth_deg * Math.PI) / 180;
  const raw = [0, 1, 2].map((i) => north[i]! * Math.cos(az) + east[i]! * Math.sin(az));
  const norm = Math.hypot(raw[0]!, raw[1]!, raw[2]!);
  const strike = raw.map((v) => v / norm);
  // The normal centre × strike, which `meanderKm` is positive towards.
  const normal = [
    centre[1] * strike[2]! - centre[2] * strike[1]!,
    centre[2] * strike[0]! - centre[0] * strike[2]!,
    centre[0] * strike[1]! - centre[1] * strike[0]!,
  ];
  const span = f.length_km / radiusKm;
  const points: LonLat[] = [];
  for (let i = 0; i < n; i += 1) {
    const along = -0.5 + (i + 0.5) / n;
    const t = along * span;
    const [c, s] = [Math.cos(t), Math.sin(t)];
    // Rotate the point on the great circle towards the normal (which
    // stays perpendicular to it) by the meander's angle.
    const m = meanderKm(index, along, f.length_km) / radiusKm;
    const [cm, sm] = [Math.cos(m), Math.sin(m)];
    const p = [0, 1, 2].map((j) => (centre[j]! * c + strike[j]! * s) * cm + normal[j]! * sm);
    const r = Math.hypot(p[0]!, p[1]!, p[2]!);
    const [x, y, z] = [p[0]! / r, p[1]! / r, p[2]! / r];
    points.push({
      lat: Math.asin(Math.max(-1, Math.min(1, z))),
      lon: Math.atan2(y, x),
    });
  }
  return points;
}

/** Where the jets of a body stand: `JETS_PER_FRACTURE` along each fracture. */
export function jetSources(g: Geysers, radiusKm: number): LonLat[] {
  // A zero-length fracture draws no stripe, so it has no jets either; the
  // others keep their index in the list for their meander.
  return g.fractures.flatMap((f, index) =>
    f.length_km > 0 ? fracturePoints(f, index, radiusKm, JETS_PER_FRACTURE) : [],
  );
}

// Two quads per jet, position.z telling them apart: the side-on
// trapezoid (0) and the end-on glow (1), weighted by how end-on the jet is
// seen so that its light stays about the same at any angle.
export const jetVertex = /* glsl */ `
  attribute vec3 aBase;
  varying vec2 vUv;
  varying vec3 vBase;
  varying vec3 vWorld;
  varying float vGlow;
  varying float vWeight;
  uniform float uHeight;
  uniform float uWidth;
  uniform float uApex;
  uniform float uEndOnFrom;
  uniform float uEndOnTo;
  uniform float uMinSlant;
  void main() {
    vec3 up = normalize(aBase);
    vec3 toEye = normalize(cameraPosition - aBase);
    float endOn = abs(dot(up, toEye));
    float glow = smoothstep(uEndOnFrom, uEndOnTo, endOn);
    vec3 camRight = vec3(viewMatrix[0][0], viewMatrix[1][0], viewMatrix[2][0]);
    vec3 camUp = vec3(viewMatrix[0][1], viewMatrix[1][1], viewMatrix[2][1]);
    vec3 world;
    if (position.z < 0.5) {
      // A trapezoid from the foot (y 0) to the top (y 1), x from -1 to 1
      // of its width there: uApex of the top width at the foot, all of it
      // at the top. vUv.x is the offset in top widths, linear across the
      // trapezoid; the fragment divides it by the local width. Brighter
      // as it shortens at a slant (thin gas), fading out end-on.
      vec3 c = cross(up, toEye);
      vec3 right = length(c) > 1e-4 ? normalize(c) : camRight;
      float share = mix(uApex, 1.0, position.y);
      world = aBase + right * (position.x * share * uWidth) + up * (position.y * uHeight);
      vUv = vec2(position.x * share, position.y);
      vWeight = (1.0 - glow) / max(sqrt(1.0 - endOn * endOn), uMinSlant);
      vGlow = 0.0;
    } else {
      // A disc of the top half-width facing the camera, centred half-way
      // up the jet (over the vent when seen end-on, clear of the ground).
      vUv = vec2(position.x, 2.0 * position.y - 1.0);
      world = aBase + up * (0.5 * uHeight) + (camRight * vUv.x + camUp * vUv.y) * uWidth;
      vWeight = glow;
      vGlow = 1.0;
    }
    vBase = aBase;
    vWorld = world;
    gl_Position = projectionMatrix * viewMatrix * vec4(world, 1.0);
  }
`;

export const jetFragment = /* glsl */ `
  uniform vec3 uSunDir;
  uniform float uLightMode;
  uniform float uOpacity;
  uniform float uApex;
  uniform float uCore;
  uniform float uGlowOpacity;
  varying vec2 vUv;
  varying vec3 vBase;
  varying vec3 vWorld;
  varying float vGlow;
  varying float vWeight;
  ${dustPhaseGlsl}
  void main() {
    float body;
    if (vGlow > 0.5) {
      // End-on: a round glow brightest at the vent, falling to the rim.
      float rho = length(vUv);
      if (rho >= 1.0) discard;
      body = uGlowOpacity * (1.0 - rho) * (1.0 - rho);
    } else {
      // A cone: soft at its sides, its gas spread over its width (denser
      // towards the vent, down to the core share) and fading with height.
      float share = mix(uApex, 1.0, vUv.y);
      float side = 1.0 - smoothstep(0.0, 1.0, abs(vUv.x) / share);
      float top = pow(1.0 - vUv.y, 1.5) / max(share, uCore);
      body = uOpacity * side * top;
    }
    float gain = 1.0;
    if (uLightMode > 0.5) {
      vec3 toEye = normalize(cameraPosition - vBase);
      gain = dustGain(-dot(uSunDir, toEye));
      // Nothing to scatter in the planet's shadow (same test as the dust rings).
      float b = dot(vWorld, uSunDir);
      if (b < 0.0 && dot(vWorld, vWorld) - b * b < 1.0) gain = 0.0;
    }
    float a = clamp(body * vWeight * gain, 0.0, 1.0);
    gl_FragColor = vec4(vec3(0.92, 0.96, 1.0), a);
    #include <colorspace_fragment>
  }
`;

/** The mesh of jets: instanced cones, with their end-on glows, at the `aBase` positions (see `placeJets`). */
export type JetMesh = Mesh<InstancedBufferGeometry, ShaderMaterial>;

/** A jets mesh sharing the view's sun uniforms; empty until `placeJets`. */
export function createJets(uniforms: { uSunDir: IUniform; uLightMode: IUniform }): JetMesh {
  const geometry = new InstancedBufferGeometry();
  // The trapezoid (z 0) and the glow (z 1), each a quad of x −1 to 1, y 0 to 1.
  geometry.setAttribute(
    'position',
    new Float32BufferAttribute([-1, 0, 0, 1, 0, 0, 1, 1, 0, -1, 1, 0, -1, 0, 1, 1, 0, 1, 1, 1, 1, -1, 1, 1], 3),
  );
  geometry.setIndex([0, 1, 2, 0, 2, 3, 4, 5, 6, 4, 6, 7]);
  geometry.instanceCount = 0;
  const material = new ShaderMaterial({
    vertexShader: jetVertex,
    fragmentShader: jetFragment,
    uniforms: {
      uHeight: { value: JET_HEIGHT_RADII },
      uWidth: { value: JET_TOP_HALF_WIDTH },
      uApex: { value: JET_APEX_SHARE },
      uCore: { value: JET_CORE_SHARE },
      uOpacity: { value: JET_OPACITY },
      uGlowOpacity: { value: JET_GLOW_OPACITY },
      uEndOnFrom: { value: JET_END_ON_FROM },
      uEndOnTo: { value: JET_END_ON_TO },
      uMinSlant: { value: JET_MIN_SLANT },
      uSunDir: uniforms.uSunDir,
      uLightMode: uniforms.uLightMode,
    },
    transparent: true,
    depthWrite: false,
    blending: AdditiveBlending,
  });
  const mesh = new Mesh(geometry, material);
  mesh.frustumCulled = false;
  mesh.visible = false;
  mesh.renderOrder = 2;
  return mesh;
}

/** Stand the jets on `bases`, their feet in world positions (the body at the origin). */
export function placeJets(mesh: JetMesh, bases: readonly Vector3[]): void {
  const data = new Float32Array(bases.length * 3);
  bases.forEach((b, i) => b.toArray(data, i * 3));
  // Free the GL buffers: replacing the attribute alone leaves the old one
  // allocated (three re-uploads what the next frame needs).
  mesh.geometry.dispose();
  mesh.geometry.setAttribute('aBase', new InstancedBufferAttribute(data, 3));
  mesh.geometry.instanceCount = bases.length;
}
