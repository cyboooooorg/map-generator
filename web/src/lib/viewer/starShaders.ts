// Shaders of the star view (StarView.ts): the star's surface and its glow.
import { STAR_SURFACE_GLSL } from './starSurface';

export const starSurfaceVertex = /* glsl */ `
  varying vec3 vObject;
  varying vec3 vViewNormal;
  void main() {
    vObject = position;
    vViewNormal = normalize(normalMatrix * normal);
    gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0);
  }
`;

// The photosphere (see starSurface.ts): limb darkening, granulation and
// the active regions of the star's activity, turned by the system clock.
// The soft exposure acts on the brightness only (starSurfaceExposed), so
// the disk keeps its colour and its limb; a dither of half an 8-bit step
// keeps the smooth gradient from banding.
export const starSurfaceFragment = /* glsl */ `
  varying vec3 vObject;
  varying vec3 vViewNormal;
  ${STAR_SURFACE_GLSL}
  void main() {
    float mu = clamp(normalize(vViewNormal).z, 0.0, 1.0);
    gl_FragColor = vec4(starSurfaceExposed(normalize(vObject), mu), 1.0);
    #include <colorspace_fragment>
    gl_FragColor.rgb += starDither(gl_FragCoord.xy) / 255.0;
  }
`;

export const glowVertex = /* glsl */ `
  varying vec2 vUv;
  void main() {
    vUv = uv;
    gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0);
  }
`;

// The corona and the light scattered around the star: a falloff outside the
// disk, on a quad facing the camera (`uSize` star radii across).
export const glowFragment = /* glsl */ `
  uniform vec3 uColor;
  uniform float uSize;
  uniform float uStrength;
  varying vec2 vUv;
  void main() {
    float r = length(vUv - 0.5) * uSize;
    if (r < 0.98) discard;
    float glow = uStrength * (0.55 * exp(-(r - 1.0) * 4.0) + 0.12 * exp(-(r - 1.0) * 1.6));
    glow *= 1.0 - smoothstep(0.42 * uSize, 0.5 * uSize, r);
    gl_FragColor = vec4(uColor * glow, 1.0);
    #include <colorspace_fragment>
  }
`;

// Prominences: arcs of plasma above the surface, glowing in the red of
// hydrogen (H-alpha). `uLift` stretches an arc outwards from the star (an
// eruption), `uOpacity` fades it.
export const prominenceVertex = /* glsl */ `
  attribute float aStrand;
  attribute vec3 aDrift;
  /** Radial stretch (1: at rest; more as an eruption rises). */
  uniform float uLift;
  /** How far the strands fray apart, per unit of height. */
  uniform float uFray;
  varying float vHeight;
  varying float vAlong;
  varying float vStrand;
  varying vec3 vViewNormal;
  varying vec3 vWorld;
  void main() {
    float r = length(position);
    float height = r - 1.0;
    vec3 lifted = position / r * (1.0 + height * uLift) + aDrift * height * uLift * uFray;
    vHeight = height;
    vAlong = uv.x;
    vStrand = aStrand;
    vViewNormal = normalize(normalMatrix * normal);
    vWorld = (modelMatrix * vec4(lifted, 1.0)).xyz;
    gl_Position = projectionMatrix * modelViewMatrix * vec4(lifted, 1.0);
  }
`;

export const prominenceFragment = /* glsl */ `
  uniform vec3 uColor;
  uniform float uOpacity;
  uniform float uTime;
  uniform float uSeed;
  varying float vHeight;
  varying float vAlong;
  varying float vStrand;
  varying vec3 vViewNormal;
  varying vec3 vWorld;
  /** Radius of the star, centred on the origin. */
  uniform float uRadius;
  void main() {
    // Seen against the disk? The ray from the eye through this point passes
    // within the star's radius of its centre (beyond the point: the parts
    // behind the star are hidden by the depth test).
    vec3 ray = normalize(vWorld - cameraPosition);
    float closest = length(cameraPosition - ray * dot(cameraPosition, ray));
    float onDisk = 1.0 - smoothstep(0.97 * uRadius, uRadius, closest);
    // A thin thread: bright along its axis, fading to its edges.
    float core = pow(abs(normalize(vViewNormal).z), 1.5);
    // Knots of brighter gas flowing down both legs to the surface, as the
    // matter of a prominence drains back along its field lines.
    float leg = vAlong < 0.5 ? vAlong : 1.0 - vAlong;
    float flow = fract(leg * (5.0 + 4.0 * vStrand) + uTime * (0.08 + 0.06 * vStrand) + vStrand * 7.0 + uSeed);
    float knots = 0.45 + 0.55 * smoothstep(0.0, 0.25, flow) * smoothstep(0.75, 0.35, flow);
    // Each strand its own brightness, flickering slowly.
    float strand = 0.45 + 0.55 * fract(vStrand * 13.7 + uSeed);
    float flicker = 0.85 + 0.15 * sin(uTime * (1.3 + vStrand) + vStrand * 40.0);
    // Faint at the feet, where the strands crowd together against the bright
    // disk, brightest in the legs and the arch.
    float density = smoothstep(0.0, 0.08, vHeight) * (1.0 - 0.4 * smoothstep(0.1, 0.5, vHeight));
    float glow = core * knots * strand * flicker * density;
    // Off the disk, the gas glows in the red of hydrogen, blended over what
    // is behind rather than added to it (added to the star's near-white
    // glow, it would wash out to white). Against the disk it is cooler than
    // the surface behind and absorbs its light: a dark filament, as H-alpha
    // images of the Sun show them.
    vec3 emission = uColor * (0.55 + 0.6 * glow);
    vec3 absorption = uColor * 0.12;
    float alpha = clamp(glow * uOpacity * mix(1.0, 1.6, onDisk), 0.0, mix(1.0, 0.8, onDisk));
    gl_FragColor = vec4(mix(emission, absorption, onDisk), alpha);
    #include <colorspace_fragment>
  }
`;
