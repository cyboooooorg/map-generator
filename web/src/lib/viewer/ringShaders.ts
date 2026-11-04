// The rings of a giant planet: a flat annulus in its equatorial plane, its
// opacity varying with the radius (ringlets, gaps), lit by the sun and
// darkened where the planet shadows it.

export const ringVertex = /* glsl */ `
  varying vec3 vLocal;
  varying vec3 vWorld;
  void main() {
    vLocal = position;
    vec4 world = modelMatrix * vec4(position, 1.0);
    vWorld = world.xyz;
    gl_Position = projectionMatrix * viewMatrix * world;
  }
`;

export const ringFragment = /* glsl */ `
  uniform vec3 uColour;        // linear colour of the ring particles
  uniform float uInner;        // edges, in planet radii
  uniform float uOuter;
  uniform float uOpacity;      // of the densest part
  uniform vec2 uGaps[4];       // centre and width, in planet radii
  uniform int uGapCount;
  uniform float uSeed;
  uniform int uLightMode;      // 0 = headlight, 1 = sun
  uniform vec3 uSunDir;        // world space
  uniform float uFade;         // the globe's morph: rings belong to the globe
  varying vec3 vLocal;
  varying vec3 vWorld;

  float hash(float x) {
    return fract(sin(x * 127.1 + uSeed * 311.7) * 43758.5453);
  }

  // Smooth noise along the radius: the ringlets.
  float ringlets(float r) {
    float x = r * 180.0;
    float i = floor(x);
    float f = fract(x);
    f = f * f * (3.0 - 2.0 * f);
    float fine = mix(hash(i), hash(i + 1.0), f);
    float y = r * 23.0;
    float j = floor(y);
    float g = fract(y);
    g = g * g * (3.0 - 2.0 * g);
    float broad = mix(hash(j + 50.0), hash(j + 51.0), g);
    return 0.35 + 0.4 * broad + 0.25 * fine;
  }

  void main() {
    float r = length(vLocal.xy);
    if (r < uInner || r > uOuter) discard;
    float across = (r - uInner) / (uOuter - uInner);
    // Thinner towards both edges, densest in the middle-outer part.
    float density = uOpacity * ringlets(r) * smoothstep(0.0, 0.08, across) * smoothstep(1.0, 0.92, across);
    for (int k = 0; k < 4; k++) {
      if (k < uGapCount) {
        density *= smoothstep(0.0, 1.0, abs(r - uGaps[k].x) / max(uGaps[k].y * 0.5, 1e-4));
      }
    }
    if (density < 0.003) discard;
    float light = 1.0;
    if (uLightMode == 1) {
      // The planet (radius 1 at the origin) between this point and the sun.
      vec3 p = vWorld;
      float b = dot(p, uSunDir);
      float c = dot(p, p) - 1.0;
      float shadow = (b < 0.0 && b * b - c > 0.0) ? 1.0 : 0.0;
      light = mix(1.0, 0.08, shadow);
    }
    gl_FragColor = vec4(uColour * light, density * uFade);
    #include <colorspace_fragment>
  }
`;
