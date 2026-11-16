// Jets rising from a moon's geyser fractures in its close-up (the tiger
// stripes of Enceladus): one camera-facing column per source, standing on
// the surface along each fracture. The plume in a moon's sky is plume.ts.
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

/** Width of a jet at its foot, as a share of its height (look constant). */
export const JET_WIDTH_SHARE = 0.06;

/** Brightness of a jet seen at 90° of scattering, before the phase gain (look constant). */
export const JET_OPACITY = 0.18;

/**
 * `n` points evenly along a fracture, its ends excluded. Same great circle
 * as `Fracture::ends` in crates/mapgen/src/planet/geysers.rs, computed in
 * that frame (z the pole, longitude 0 on x) and returned as latitude and
 * longitude in radians, which the viewer uses with its own axes.
 */
export function fracturePoints(f: Fracture, radiusKm: number, n: number): LonLat[] {
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
  const span = f.length_km / radiusKm;
  const points: LonLat[] = [];
  for (let i = 0; i < n; i += 1) {
    const t = (-0.5 + (i + 0.5) / n) * span;
    const [c, s] = [Math.cos(t), Math.sin(t)];
    const x = centre[0] * c + strike[0]! * s;
    const y = centre[1] * c + strike[1]! * s;
    const z = centre[2] * c + strike[2]! * s;
    points.push({ lat: Math.asin(Math.max(-1, Math.min(1, z))), lon: Math.atan2(y, x) });
  }
  return points;
}

/** Where the jets of a body stand: `JETS_PER_FRACTURE` along each fracture. */
export function jetSources(g: Geysers, radiusKm: number): LonLat[] {
  return g.fractures.flatMap((f) => fracturePoints(f, radiusKm, JETS_PER_FRACTURE));
}

const jetVertex = /* glsl */ `
  attribute vec3 aBase;
  varying vec2 vUv;
  varying vec3 vBase;
  uniform float uHeight;
  uniform float uWidth;
  void main() {
    vec3 up = normalize(aBase);
    vec3 toEye = cameraPosition - aBase;
    vec3 right = normalize(cross(up, toEye));
    // The quad runs from the foot (y 0) to the top (y 1), x from -1 to 1.
    vec3 world = aBase + right * (position.x * uWidth) + up * (position.y * uHeight);
    vUv = vec2(position.x, position.y);
    vBase = aBase;
    gl_Position = projectionMatrix * viewMatrix * vec4(world, 1.0);
  }
`;

const jetFragment = /* glsl */ `
  uniform vec3 uSunDir;
  uniform float uLightMode;
  uniform float uOpacity;
  varying vec2 vUv;
  varying vec3 vBase;
  ${dustPhaseGlsl}
  void main() {
    // Narrower and fainter with height, soft at the sides and the top.
    float width = mix(1.0, 0.35, vUv.y);
    float side = 1.0 - smoothstep(0.0, 1.0, abs(vUv.x) / width);
    float top = pow(1.0 - vUv.y, 1.5);
    float gain = 1.0;
    if (uLightMode > 0.5) {
      vec3 toEye = normalize(cameraPosition - vBase);
      gain = dustGain(-dot(uSunDir, toEye));
    }
    float a = clamp(uOpacity * gain * side * top, 0.0, 1.0);
    gl_FragColor = vec4(vec3(0.92, 0.96, 1.0) * a, a);
  }
`;

/** The mesh of jets: instanced columns at the `aBase` positions (see `placeJets`). */
export type JetMesh = Mesh<InstancedBufferGeometry, ShaderMaterial>;

/** A jets mesh sharing the view's sun uniforms; empty until `placeJets`. */
export function createJets(uniforms: { uSunDir: IUniform; uLightMode: IUniform }): JetMesh {
  const geometry = new InstancedBufferGeometry();
  geometry.setAttribute('position', new Float32BufferAttribute([-1, 0, 0, 1, 0, 0, 1, 1, 0, -1, 1, 0], 3));
  geometry.setIndex([0, 1, 2, 0, 2, 3]);
  geometry.instanceCount = 0;
  const material = new ShaderMaterial({
    vertexShader: jetVertex,
    fragmentShader: jetFragment,
    uniforms: {
      uHeight: { value: JET_HEIGHT_RADII },
      uWidth: { value: JET_HEIGHT_RADII * JET_WIDTH_SHARE },
      uOpacity: { value: JET_OPACITY },
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
  mesh.geometry.setAttribute('aBase', new InstancedBufferAttribute(data, 3));
  mesh.geometry.instanceCount = bases.length;
}
