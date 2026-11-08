// The planets of the star view: small spheres lit by their star, which
// stands at the scene's origin, with a day side and a night side, their
// relief from their normal map and their lava glowing (see lavaGlsl in
// shaders.ts, whose levels they share).
import { Color, ShaderMaterial, type Texture, Vector3 } from 'three';
import { emptyTexture } from './textures';
import { lavaGlsl } from './shaders';

const vertex = /* glsl */ `
  varying vec2 vUv;
  varying vec3 vNormalW;
  varying vec3 vEastW;
  varying vec3 vToStar;
  void main() {
    vUv = uv;
    vec3 n = normalize(position);
    // three's SphereGeometry: u grows eastward round +y, north is +y.
    vec3 east = normalize(vec3(n.z, 0.0, -n.x) + vec3(1e-6, 0.0, 0.0));
    mat3 turn = mat3(modelMatrix);
    vNormalW = normalize(turn * n);
    vEastW = normalize(turn * east);
    vec3 centre = (modelMatrix * vec4(0.0, 0.0, 0.0, 1.0)).xyz;
    vToStar = -centre;
    gl_Position = projectionMatrix * viewMatrix * modelMatrix * vec4(position, 1.0);
  }
`;

const fragment = /* glsl */ `
  uniform vec3 uColour;
  uniform float uHasMap;
  uniform sampler2D uMap;
  uniform float uHasNormal;
  uniform sampler2D uNormalMap;
  varying vec2 vUv;
  varying vec3 vNormalW;
  varying vec3 vEastW;
  varying vec3 vToStar;
  ${lavaGlsl}
  // The night side keeps a trace of light, so a small planet still shows
  // its disk against the dark.
  const float NIGHT_FLOOR = 0.06;
  void main() {
    vec3 colour = uHasMap > 0.5 ? texture2D(uMap, vUv).rgb : uColour;
    vec3 n = normalize(vNormalW);
    vec3 east = normalize(vEastW);
    vec3 north = cross(n, east);
    vec3 star = normalize(vToStar);
    float h = dot(n, star);
    float day = smoothstep(-0.08, 0.15, h) * (0.55 + 0.45 * clamp(h, 0.0, 1.0));
    if (uHasNormal > 0.5) {
      vec3 t = normalize(texture2D(uNormalMap, vUv).rgb * 2.0 - 1.0);
      vec3 relief = normalize(t.x * east + t.y * north + t.z * n);
      float ratio = clamp(max(dot(relief, star), 0.0) / max(h, 0.04), 0.12, 2.2);
      day *= mix(1.0, ratio, smoothstep(-0.02, 0.06, h));
    }
    vec3 lit = colour * (NIGHT_FLOOR + (1.0 - NIGHT_FLOOR) * day);
    if (uHasMap > 0.5) {
      float lava = lavaShare(colour);
      lit += colour * LAVA_GLOW * lava * (1.0 - smoothstep(-0.08, 0.15, h));
      lit = max(lit, colour * LAVA_DAY_GLOW * lava);
    }
    gl_FragColor = vec4(lit, 1.0);
    #include <colorspace_fragment>
  }
`;

/** The material of a planet's marker, in `colour` until it has maps. */
export function planetMarkerMaterial(colour: Color): ShaderMaterial {
  return new ShaderMaterial({
    vertexShader: vertex,
    fragmentShader: fragment,
    uniforms: {
      uColour: { value: new Vector3(colour.r, colour.g, colour.b) },
      uHasMap: { value: 0 },
      uMap: { value: emptyTexture() },
      uHasNormal: { value: 0 },
      uNormalMap: { value: emptyTexture() },
    },
  });
}

/**
 * Shows `colour` (a texture, sRGB) and `normal` (relief normals) on a
 * marker's material, or its plain `fallback` colour without a colour map;
 * no normal map leaves it flat.
 */
export function setMarkerMaps(material: ShaderMaterial, colour: Texture | null, normal: Texture | null, fallback: Color): void {
  const u = material.uniforms;
  u['uHasMap']!.value = colour ? 1 : 0;
  u['uMap']!.value = colour ?? emptyTexture();
  u['uHasNormal']!.value = colour && normal ? 1 : 0;
  u['uNormalMap']!.value = normal ?? emptyTexture();
  (u['uColour']!.value as Vector3).set(fallback.r, fallback.g, fallback.b);
}
