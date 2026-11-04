// Shaders of the star view (StarView.ts): the star's surface and its glow.

/** 3D simplex noise (Ashima Arts / Stefan Gustavson, MIT), in [-1, 1]. */
const SIMPLEX = /* glsl */ `
  vec3 mod289(vec3 x) { return x - floor(x * (1.0 / 289.0)) * 289.0; }
  vec4 mod289(vec4 x) { return x - floor(x * (1.0 / 289.0)) * 289.0; }
  vec4 permute(vec4 x) { return mod289(((x * 34.0) + 10.0) * x); }
  vec4 taylorInvSqrt(vec4 r) { return 1.79284291400159 - 0.85373472095314 * r; }
  float snoise(vec3 v) {
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
    i = mod289(i);
    vec4 p = permute(permute(permute(i.z + vec4(0.0, i1.z, i2.z, 1.0)) + i.y + vec4(0.0, i1.y, i2.y, 1.0)) + i.x + vec4(0.0, i1.x, i2.x, 1.0));
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
    vec4 norm = taylorInvSqrt(vec4(dot(p0, p0), dot(p1, p1), dot(p2, p2), dot(p3, p3)));
    p0 *= norm.x; p1 *= norm.y; p2 *= norm.z; p3 *= norm.w;
    vec4 m = max(0.5 - vec4(dot(x0, x0), dot(x1, x1), dot(x2, x2), dot(x3, x3)), 0.0);
    m = m * m;
    return 105.0 * dot(m * m, vec4(dot(p0, x0), dot(p1, x1), dot(p2, x2), dot(p3, x3)));
  }
`;

export const starSurfaceVertex = /* glsl */ `
  varying vec3 vObject;
  varying vec3 vViewNormal;
  void main() {
    vObject = position;
    vViewNormal = normalize(normalMatrix * normal);
    gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0);
  }
`;

// The photosphere: limb darkening (I = 1 − a(1 − μ) − b(1 − μ)²), convective
// granulation (none on hot stars, whose outer layers do not convect), spots
// anywhere on the surface with bright faculae around them,
// and on red dwarfs an occasional flare. `uTime` turns and boils it.
export const starSurfaceFragment = /* glsl */ `
  uniform vec3 uColor;
  uniform float uTime;
  uniform float uLimb;         // linear limb darkening coefficient a
  uniform float uGranulation;  // contrast of the granules
  uniform float uSpots;        // share of the surface under spots, about
  uniform float uFlares;       // 1 on flaring red dwarfs
  uniform float uSeed;
  varying vec3 vObject;
  varying vec3 vViewNormal;
  ${SIMPLEX}
  float fbm(vec3 p) {
    return 0.5 * snoise(p) + 0.25 * snoise(p * 2.03) + 0.125 * snoise(p * 4.01);
  }
  void main() {
    float mu = clamp(normalize(vViewNormal).z, 0.0, 1.0);
    float edge = 1.0 - mu;
    float limb = 1.0 - uLimb * edge - 0.2 * edge * edge;
    // Rotation about the vertical axis.
    float a = uTime * 0.02;
    vec3 p = vec3(cos(a) * vObject.x + sin(a) * vObject.z, vObject.y, -sin(a) * vObject.x + cos(a) * vObject.z);
    float granules = snoise(p * 28.0 + vec3(0.0, 0.0, uTime * 0.25)) * 0.6 + snoise(p * 61.0 - vec3(uTime * 0.4)) * 0.4;
    float brightness = 1.0 + uGranulation * granules;
    // Spots, scattered over the whole surface.
    float field = fbm(p * 3.2 + uSeed);
    float threshold = 0.42 - 0.9 * uSpots;
    float penumbra = smoothstep(threshold, threshold + 0.06, field);
    float umbra = smoothstep(threshold + 0.07, threshold + 0.12, field);
    brightness *= 1.0 - 0.45 * penumbra - 0.4 * umbra;
    // Faculae: bright patches around the spots, seen near the limb.
    float faculae = smoothstep(threshold - 0.12, threshold, field) * (1.0 - penumbra);
    brightness += 0.25 * faculae * edge * step(0.001, uSpots);
    // A flare now and then on red dwarfs.
    float flare = uFlares * pow(max(sin(uTime * 0.35 + uSeed), 0.0), 60.0);
    float spot = smoothstep(0.25, 0.0, distance(p, normalize(vec3(0.5, 0.3, 0.8))));
    vec3 colour = uColor * brightness * limb + vec3(1.2, 1.1, 1.0) * flare * spot;
    // Soft exposure, so the bright disk keeps its colour.
    gl_FragColor = vec4(1.0 - exp(-1.6 * colour), 1.0);
    #include <colorspace_fragment>
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
