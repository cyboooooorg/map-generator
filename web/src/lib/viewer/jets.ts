// Jets rising from a moon's geyser fractures in its close-up (the tiger
// stripes of Enceladus): one camera-facing cone per source, its apex on
// the surface along each fracture, widening with height. The sources are
// scattered irregularly, in groups and gaps, each with its own height,
// brightness and tilt, with a few faint broad curtains between them (see
// jetLayout). Seen down its axis, a cone turns into a round glow
// brightest at the vent (see jetVertex). The plume in a moon's sky is
// plume.ts.
import {
  AdditiveBlending,
  Float32BufferAttribute,
  InstancedBufferAttribute,
  InstancedBufferGeometry,
  Mesh,
  ShaderMaterial,
  type IUniform,
  Vector3,
} from 'three';
import type { Fracture } from '../api/types/Fracture';
import type { Geysers } from '../api/types/Geysers';
import type { LonLat } from '../geo';
import { dustPhaseGlsl } from './dustPhase';
import { random, subSeed } from './skyRandom';

/**
 * Least and most jets along one fracture: about a hundred over
 * Enceladus's four stripes, in irregular groups (Porco et al. 2014,
 * doi:10.1088/0004-6256/148/3/45).
 */
export const JETS_MIN = 12;
export const JETS_MAX = 30;

/**
 * The light of the jets of one fracture, in jets of full height, width
 * and brightness (look constant): what the 24 identical jets drawn before
 * held, shared out among the irregular ones (see jetLayout).
 */
export const JET_LIGHT_PER_FRACTURE = 24;

/** Least and most groups the jets of a fracture gather in (look constant). */
export const JET_GROUPS_MIN = 2;
export const JET_GROUPS_MAX = 5;

/** Least and most spread of a group along its fracture, sigma in its lengths (look constant). */
export const JET_GROUP_SIGMA_MIN = 0.015;
export const JET_GROUP_SIGMA_MAX = 0.05;

/** Share of the jets standing alone, anywhere along the fracture, not in a group (look constant). */
export const JET_LONE_SHARE = 0.25;

/** Least and most height of a jet, in `JET_HEIGHT_RADII` (look constant). */
export const JET_HEIGHT_MIN = 0.4;
export const JET_HEIGHT_MAX = 1.3;

/** Least brightness of a jet, of 1 for the brightest (look constant). */
export const JET_BRIGHTNESS_MIN = 0.3;

/** Most tilt of a jet off the vertical, degrees, in any direction (look constant). */
export const JET_TILT_MAX_DEG = 15;

/**
 * Faint broad curtains of the fracture's diffuse emission between the
 * jets (Spitale et al. 2015, Nature 521, "Curtain eruptions from
 * Enceladus' south-polar terrain"): least and most per fracture, and their
 * width, height and brightness in a jet's (look constants).
 */
export const CURTAINS_MIN = 1;
export const CURTAINS_MAX = 3;
export const CURTAIN_WIDTH = 3;
export const CURTAIN_HEIGHT = 0.5;
export const CURTAIN_BRIGHTNESS = 0.2;

/** Height of a jet of full height in the body's radii (look constant). */
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
 * ends excluded (see fracturePointsAt).
 */
export function fracturePoints(f: Fracture, index: number, radiusKm: number, n: number): LonLat[] {
  return fracturePointsAt(
    f,
    index,
    radiusKm,
    Array.from({ length: n }, (_, i) => -0.5 + (i + 0.5) / n),
  );
}

/**
 * The points at `alongs` (−0.5 and 0.5 at its ends) on the fracture
 * number `index` of its body. Same great circle as `Fracture::ends` in
 * crates/mapgen/src/planet/geysers.rs, each point moved sideways by
 * `meanderKm` as the stripes are, computed in that frame (z the pole,
 * longitude 0 on x) and returned as latitude and longitude in radians,
 * which the viewer uses with its own axes.
 */
export function fracturePointsAt(f: Fracture, index: number, radiusKm: number, alongs: readonly number[]): LonLat[] {
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
  for (const along of alongs) {
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

/** One jet, or curtain, along a fracture (see jetLayout). */
export interface JetShape {
  /** Where along its fracture, −0.5 and 0.5 at its ends. */
  along: number;
  /** Height and top width, in a full jet's (`JET_HEIGHT_RADII`, `JET_TOP_HALF_WIDTH`). */
  height: number;
  width: number;
  /** Brightness, of 1 for the brightest jet of its fracture. */
  brightness: number;
  /** Brightness per unit area drawn, in `JET_OPACITY`: the brightness, scaled to the fracture's light. */
  light: number;
  /** Tilt off the vertical, radians, and its direction, radians from east towards north. */
  tilt: number;
  tiltAzimuth: number;
  /** A faint broad curtain rather than a jet. */
  curtain: boolean;
}

/** A jet where it stands. */
export interface JetSource extends JetShape {
  site: LonLat;
}

/** A standard normal deviate (Box-Muller) from `rand`. */
function gauss(rand: () => number): number {
  const u = Math.max(rand(), 1e-12);
  return Math.sqrt(-2 * Math.log(u)) * Math.cos(2 * Math.PI * rand());
}

/**
 * The jets of the fracture number `index`, seeded by it and its place
 * (the same fracture always gets the same jets): `JETS_MIN` to
 * `JETS_MAX` of them, most in `JET_GROUPS_MIN` to `JET_GROUPS_MAX` groups
 * along it, some alone (`JET_LONE_SHARE`), each with its own height,
 * brightness and tilt; and a few curtains. Sorted along the fracture.
 * Their light (brightness x height x width) adds up to
 * `JET_LIGHT_PER_FRACTURE` full jets.
 */
export function jetLayout(f: Fracture, index: number): JetShape[] {
  const rand = random(subSeed(index, `${f.lat_deg},${f.lon_deg},${f.azimuth_deg},${f.length_km}`));
  const between = (lo: number, hi: number) => lo + (hi - lo) * rand();
  const count = JETS_MIN + Math.floor(rand() * (JETS_MAX - JETS_MIN + 1));
  const groups = Array.from(
    { length: JET_GROUPS_MIN + Math.floor(rand() * (JET_GROUPS_MAX - JET_GROUPS_MIN + 1)) },
    () => ({
      centre: between(-0.42, 0.42),
      sigma: between(JET_GROUP_SIGMA_MIN, JET_GROUP_SIGMA_MAX),
    }),
  );
  const jets: JetShape[] = [];
  for (let i = 0; i < count; i += 1) {
    let along: number;
    if (rand() < JET_LONE_SHARE) along = between(-0.48, 0.48);
    else {
      const g = groups[Math.floor(rand() * groups.length)]!;
      along = g.centre + g.sigma * gauss(rand);
    }
    jets.push({
      along: Math.min(Math.max(along, -0.48), 0.48),
      height: between(JET_HEIGHT_MIN, JET_HEIGHT_MAX),
      width: 1,
      // Most of them faint, a few bright.
      brightness: JET_BRIGHTNESS_MIN + (1 - JET_BRIGHTNESS_MIN) * rand() ** 1.5,
      light: 0,
      // Uniform over the cap of the most tilt.
      tilt: Math.acos(1 - rand() * (1 - Math.cos((JET_TILT_MAX_DEG * Math.PI) / 180))),
      tiltAzimuth: between(0, 2 * Math.PI),
      curtain: false,
    });
  }
  const curtains = CURTAINS_MIN + Math.floor(rand() * (CURTAINS_MAX - CURTAINS_MIN + 1));
  for (let i = 0; i < curtains; i += 1) {
    jets.push({
      along: between(-0.4, 0.4),
      height: CURTAIN_HEIGHT,
      width: CURTAIN_WIDTH,
      brightness: CURTAIN_BRIGHTNESS,
      light: 0,
      tilt: 0,
      tiltAzimuth: 0,
      curtain: true,
    });
  }
  const total = jets.reduce((sum, j) => sum + j.brightness * j.height * j.width, 0);
  for (const j of jets) j.light = (j.brightness * JET_LIGHT_PER_FRACTURE) / total;
  return jets.sort((a, b) => a.along - b.along);
}

/** Where the jets of a body stand, and their shapes (see jetLayout). */
export function jetSources(g: Geysers, radiusKm: number): JetSource[] {
  // A zero-length fracture draws no stripe, so it has no jets either; the
  // others keep their index in the list for their meander and their seed.
  return g.fractures.flatMap((f, index) => {
    if (f.length_km <= 0) return [];
    const shapes = jetLayout(f, index);
    const sites = fracturePointsAt(
      f,
      index,
      radiusKm,
      shapes.map((j) => j.along),
    );
    return shapes.map((j, i) => ({ ...j, site: sites[i]! }));
  });
}

// Two quads per jet, position.z telling them apart: the side-on
// trapezoid (0) and the end-on glow (1), weighted by how end-on the jet is
// seen so that its light stays about the same at any angle.
export const jetVertex = /* glsl */ `
  attribute vec3 aBase;
  attribute vec3 aAxis;        // unit, the jet's axis (tilted off the vertical)
  attribute vec3 aLook;        // height and width in a full jet's, light in uOpacity
  varying vec2 vUv;
  varying vec3 vBase;
  varying vec3 vWorld;
  varying float vGlow;
  varying float vWeight;
  varying float vLight;
  uniform float uHeight;
  uniform float uWidth;
  uniform float uApex;
  uniform float uEndOnFrom;
  uniform float uEndOnTo;
  uniform float uMinSlant;
  void main() {
    vec3 up = aAxis;
    float height = uHeight * aLook.x;
    float width = uWidth * aLook.y;
    vLight = aLook.z;
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
      world = aBase + right * (position.x * share * width) + up * (position.y * height);
      vUv = vec2(position.x * share, position.y);
      vWeight = (1.0 - glow) / max(sqrt(1.0 - endOn * endOn), uMinSlant);
      vGlow = 0.0;
    } else {
      // A disc of the top half-width facing the camera, centred half-way
      // up the jet (over the vent when seen end-on, clear of the ground),
      // holding the trapezoid's light (height x width) on a disc of
      // width squared.
      vUv = vec2(position.x, 2.0 * position.y - 1.0);
      world = aBase + up * (0.5 * height) + (camRight * vUv.x + camUp * vUv.y) * width;
      vWeight = glow * aLook.x / aLook.y;
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
  varying float vLight;
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
    float a = clamp(body * vLight * vWeight * gain, 0.0, 1.0);
    gl_FragColor = vec4(vec3(0.92, 0.96, 1.0), a);
    #include <colorspace_fragment>
  }
`;

/** The mesh of jets: instanced cones, with their end-on glows, at the `aBase` positions with their `aAxis` and `aLook` (see `placeJets`). */
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

/**
 * The axis of a jet standing at `base` (the body at the origin, y its
 * pole): the vertical tilted by `tilt` towards `tiltAzimuth` (from east
 * towards north).
 */
export function jetAxis(base: Vector3, tilt: number, tiltAzimuth: number): Vector3 {
  const up = base.clone().normalize();
  const east = new Vector3(0, 1, 0).cross(up);
  if (east.lengthSq() < 1e-12) east.set(1, 0, 0);
  east.normalize();
  const north = up.clone().cross(east);
  const lean = east.multiplyScalar(Math.cos(tiltAzimuth)).add(north.multiplyScalar(Math.sin(tiltAzimuth)));
  return up
    .multiplyScalar(Math.cos(tilt))
    .add(lean.multiplyScalar(Math.sin(tilt)))
    .normalize();
}

/**
 * Stand the jets `shapes` on `bases`, their feet in world positions (the
 * body at the origin), one each.
 */
export function placeJets(mesh: JetMesh, bases: readonly Vector3[], shapes: readonly JetShape[]): void {
  const base = new Float32Array(bases.length * 3);
  const axis = new Float32Array(bases.length * 3);
  const look = new Float32Array(bases.length * 3);
  bases.forEach((b, i) => {
    const s = shapes[i]!;
    b.toArray(base, i * 3);
    jetAxis(b, s.tilt, s.tiltAzimuth).toArray(axis, i * 3);
    look.set([s.height, s.width, s.light], i * 3);
  });
  // Free the GL buffers: replacing the attributes alone leaves the old ones
  // allocated (three re-uploads what the next frame needs).
  mesh.geometry.dispose();
  mesh.geometry.setAttribute('aBase', new InstancedBufferAttribute(base, 3));
  mesh.geometry.setAttribute('aAxis', new InstancedBufferAttribute(axis, 3));
  mesh.geometry.setAttribute('aLook', new InstancedBufferAttribute(look, 3));
  mesh.geometry.instanceCount = bases.length;
}
