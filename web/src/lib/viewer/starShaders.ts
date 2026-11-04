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
// in belts on either side of the equator with bright faculae around them,
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
    // Spots, mostly within 40° of the equator.
    float belt = 1.0 - smoothstep(0.45, 0.75, abs(p.y));
    float field = fbm(p * 3.2 + uSeed) * belt;
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
