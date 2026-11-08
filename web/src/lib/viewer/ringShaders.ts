// The rings of a giant planet: a flat annulus in its equatorial plane, its
// opacity varying with the radius (ringlets, gaps), lit by the sun and
// darkened where the planet shadows it.

// The radial structure shared by a giant's rings in its own view
// (ringFragment) and in the sky of its moons (skyRingFragment): ringlets,
// two scales of smooth noise along the radius, seeded by the giant
// (`ringSeed` in lib/skyBodies.ts).
export const ringletsGlsl = /* glsl */ `
  uniform float uRingSeed;

  float ringHash(float x) {
    return fract(sin(x * 127.1 + uRingSeed * 311.7) * 43758.5453);
  }

  // How far a scale of noise fades to its mean when a pixel spans x of its
  // cells: from nothing at two pixels a cell (Nyquist) to most of it at
  // under one. A third of it is kept: sub-pixel ringlets still read as
  // grain, as in the giant's own view, rather than as a plain sheet.
  float ringletFade(float x) {
    return 0.65 * smoothstep(0.5, 1.5, x);
  }

  // The ringlets' scaling of the opacity at radius r (the giant's radii),
  // for a pixel spanning w of them (0: exact). Each scale of the noise
  // fades towards its mean as its cells shrink under two pixels (see
  // ringletFade), so that rings seen from afar do not alias into a shimmer.
  float ringlets(float r, float w) {
    float x = r * 180.0;
    float i = floor(x);
    float f = fract(x);
    f = f * f * (3.0 - 2.0 * f);
    float fine = mix(ringHash(i), ringHash(i + 1.0), f);
    fine = mix(fine, 0.5, ringletFade(w * 180.0));
    float y = r * 23.0;
    float j = floor(y);
    float g = fract(y);
    g = g * g * (3.0 - 2.0 * g);
    float broad = mix(ringHash(j + 50.0), ringHash(j + 51.0), g);
    broad = mix(broad, 0.5, ringletFade(w * 23.0));
    return 0.35 + 0.4 * broad + 0.25 * fine;
  }
`;

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
  // Half-width a gap keeps at least on screen, in drawn pixels (see main):
  // the half-size of a small moon's marker (see drawSkyBodies).
  uniform float uGapPixels;
  uniform int uLightMode;      // 0 = headlight, 1 = sun
  uniform vec3 uSunDir;        // world space
  uniform float uFade;         // the globe's morph: rings belong to the globe
  varying vec3 vLocal;
  varying vec3 vWorld;

  ${ringletsGlsl}

  void main() {
    float r = length(vLocal.xy);
    // A moonlet's own gap is a few tens of km wide, under a pixel from
    // afar, while the moonlet is drawn a few pixels across: the gap is
    // drawn at least as wide as it, so that it shows round it. (Taken
    // before the discard: fwidth needs every pixel of a quad.)
    float footprint = fwidth(r);
    float least = uGapPixels * footprint;
    if (r < uInner || r > uOuter) discard;
    float across = (r - uInner) / (uOuter - uInner);
    // Thinner towards both edges, densest in the middle-outer part.
    float density = uOpacity * ringlets(r, footprint) * smoothstep(0.0, 0.08, across) * smoothstep(1.0, 0.92, across);
    for (int k = 0; k < 4; k++) {
      if (k < uGapCount) {
        density *= smoothstep(0.0, 1.0, abs(r - uGaps[k].x) / max(max(uGaps[k].y * 0.5, least), 1e-4));
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

// A dust ring reaches far beyond the camera's far plane, which only bounds
// the globe's depth: its depth is held at that plane instead of being cut
// there (the globe, nearer, still hides it).
export const dustRingVertex = /* glsl */ `
  varying vec3 vLocal;
  varying vec3 vWorld;
  void main() {
    vLocal = position;
    vec4 world = modelMatrix * vec4(position, 1.0);
    vWorld = world.xyz;
    gl_Position = projectionMatrix * viewMatrix * world;
    gl_Position.z = min(gl_Position.z, gl_Position.w * 0.99999);
  }
`;

// A faint dust ring (crates/mapgen/src/rings.rs, `DustRing`): smooth, with
// no ringlets, densest along its source moon's orbit and fading to its
// edges; lit by the sun and shadowed by the planet as the main rings are.
export const dustRingFragment = /* glsl */ `
  uniform vec3 uColour;
  uniform float uInner;
  uniform float uOuter;
  uniform float uPeak;         // the source moon's orbit, planet radii
  uniform float uOpacity;      // where densest
  uniform int uLightMode;
  uniform vec3 uSunDir;
  uniform float uFade;
  varying vec3 vLocal;
  varying vec3 vWorld;

  void main() {
    float r = length(vLocal.xy);
    if (r < uInner || r > uOuter) discard;
    // Falling off from the peak towards each edge.
    float side = r < uPeak ? (uPeak - r) / max(uPeak - uInner, 1e-4) : (r - uPeak) / max(uOuter - uPeak, 1e-4);
    float density = uOpacity * (1.0 - smoothstep(0.0, 1.0, side));
    if (density < 0.001) discard;
    float light = 1.0;
    if (uLightMode == 1) {
      vec3 p = vWorld;
      float b = dot(p, uSunDir);
      float c = dot(p, p) - 1.0;
      light = (b < 0.0 && b * b - c > 0.0) ? 0.08 : 1.0;
    }
    gl_FragColor = vec4(uColour * light, density * uFade);
    #include <colorspace_fragment>
  }
`;

// The opacity of a giant's rings at `r` (its radii), seen straight down:
// thinner towards both edges, none in their gaps. For the rings of a body
// of the sky, and the shadow they cast on its disk.
export const skyRingDensityGlsl = /* glsl */ `
  uniform float uRingInner;    // edges, in the giant's radii (0: no rings)
  uniform float uRingOuter;
  uniform float uRingOpacity;  // of the densest part
  uniform vec2 uRingGaps[4];   // centre and width, in the giant's radii
  uniform float uRingGapCount;
  // Half-width a gap keeps at least, in the giant's radii (0: its own): a
  // few pixels at the giant's distance, as in ringFragment, so that a
  // moonlet's gap shows round its marker. Set from the CPU: this runs in
  // branches, where fwidth is undefined.
  uniform float uRingGapLeast;
  float ringDensity(float r) {
    if (uRingOuter <= 0.0 || r < uRingInner || r > uRingOuter) return 0.0;
    float across = (r - uRingInner) / (uRingOuter - uRingInner);
    float density = uRingOpacity * smoothstep(0.0, 0.08, across) * smoothstep(1.0, 0.92, across);
    for (int k = 0; k < 4; k++) {
      if (float(k) >= uRingGapCount) break;
      density *= smoothstep(0.0, 1.0, abs(r - uRingGaps[k].x) / max(max(uRingGaps[k].y * 0.5, uRingGapLeast), 1e-4));
    }
    return density;
  }
`;

// The rings of a body of the sky (the giant seen from its moon), drawn
// over the whole screen: each pixel's ray meets the giant's equatorial
// plane where it meets it, and the pixel takes that point's depth, so that
// the rings round a ring moon pass in front of it and behind it (beyond
// the far plane, held on it: behind the globe). Positions are in the
// scene's units (radii of the body shown), from the camera.
export const skyRingVertex = /* glsl */ `
  uniform mat4 uProjectionInverse;
  varying vec3 vRay;
  void main() {
    // A triangle covering the screen, at the far plane.
    vec2 ndc = position.xy;
    vec4 view = uProjectionInverse * vec4(ndc, 1.0, 1.0);
    // Back from the view's frame to the scene's (the view matrix's
    // rotation is orthonormal: its inverse is its transpose).
    vRay = transpose(mat3(viewMatrix)) * (view.xyz / view.w);
    gl_Position = vec4(ndc, 1.0, 1.0);
  }
`;

// Lit by the sun on the face it turns to it (Lommel-Seeliger, as dusty
// ice reflects), faintly through itself on the other face, darkened where
// the giant shadows it, hidden behind the giant. Seen edge on, as from a
// regular moon or a ring moon in their plane, a ring a few km thick would
// vanish: it is kept a line about two pixels wide, as dense as it is seen
// along its length, at the horizon where its nearest part lies.
export const skyRingFragment = /* glsl */ `
  uniform vec3 uCentre;        // the giant's centre, from the camera
  uniform float uRadius;       // the giant's radius
  uniform vec3 uNormal;        // its spin axis: the rings' plane's normal
  uniform vec3 uColour;        // linear colour of the ring particles
  uniform vec3 uSunDir;
  uniform float uFade;
  // The camera's projection (three.js gives fragment shaders none).
  uniform mat4 uProjection;
  // The angle one drawn pixel spans, radians: the footprint of a pixel on
  // the rings is worked out from it, as fwidth is undefined in branches.
  uniform float uPixelAngle;
  ${skyRingDensityGlsl}
  ${ringletsGlsl}
  varying vec3 vRay;
  // Most a sheet of them seen nearly edge on hides (see main).
  const float GRAZING_OPACITY = 0.55;

  // The depth of the point at offset from the camera (scene units),
  // clamped to the far plane.
  float depthOf(vec3 offset) {
    vec4 clip = uProjection * vec4(mat3(viewMatrix) * offset, 1.0);
    return clamp(0.5 * clip.z / clip.w + 0.5, 0.0, 1.0);
  }

  // Light off the rings: the face towards the sun lit, the other faintly
  // through them (the less, the denser), the giant's shadow on them.
  vec3 ringLight(vec3 p, float mu, float density) {
    float sunMu = dot(uSunDir, uNormal);
    float lambert = min(1.0, 2.0 * abs(sunMu) / (abs(sunMu) + mu + 1e-4));
    float facing = sign(sunMu) * sign(dot(-uCentre, uNormal));
    float light = facing > 0.0 ? lambert : 0.3 * lambert * (1.0 - 0.7 * density);
    float b = dot(p, uSunDir);
    float off = length(p - b * uSunDir) / uRadius;
    light *= b < 0.0 ? mix(0.03, 1.0, smoothstep(0.98, 1.02, off)) : 1.0;
    return uColour * (0.01 + 0.99 * light);
  }

  void main() {
    if (uRingOuter <= 0.0) discard;
    vec3 ray = normalize(vRay);
    // Where the ray meets the giant, if it does (hiding what lies behind).
    float b = dot(ray, uCentre);
    float disc = b * b - dot(uCentre, uCentre) + uRadius * uRadius;
    float hit = disc > 0.0 && b > 0.0 ? b - sqrt(disc) : 1e30;
    vec3 camera = -uCentre;
    float height = dot(camera, uNormal);
    float g = dot(ray, uNormal);
    vec4 colour = vec4(0.0);
    float depth = 1.0;
    // The rings seen open: where the ray crosses their plane.
    if (abs(g) > 1e-7) {
      float t = -height / g;
      if (t > 0.0 && t < hit) {
        vec3 p = camera + t * ray;
        float rp = length(p) / uRadius;
        // The span of a pixel along the radius, from the radius met by the
        // rays a pixel to the right and a pixel up (long across a ring seen
        // slant, short along it); never beyond the footprint of a ray
        // meeting the plane slant, which the pixel's neighbours may miss.
        float widest = t * uPixelAngle / max(abs(g), 0.02) / uRadius;
        vec3 right = vec3(viewMatrix[0][0], viewMatrix[1][0], viewMatrix[2][0]);
        vec3 up = vec3(viewMatrix[0][1], viewMatrix[1][1], viewMatrix[2][1]);
        float footprint = widest;
        float gx = dot(ray + right * uPixelAngle, uNormal);
        float gy = dot(ray + up * uPixelAngle, uNormal);
        if (abs(gx) > 1e-7 && abs(gy) > 1e-7) {
          float tx = -height / gx;
          float ty = -height / gy;
          if (tx > 0.0 && ty > 0.0) {
            float rx = length(camera + tx * (ray + right * uPixelAngle)) / uRadius;
            float ry = length(camera + ty * (ray + up * uPixelAngle)) / uRadius;
            footprint = min(widest, max(abs(rx - rp), abs(ry - rp)));
          }
        }
        float d = ringDensity(rp) * ringlets(rp, footprint);
        // Seen nearly edge on, as from a ring moon a little off their
        // plane, through more of them (their opacity to the power 1/mu),
        // held at most half opaque (a choice, not physics: dense rings
        // would hide the giant behind them, a uniform veil over half the
        // sky). Opened up, from about 7 degrees over their plane, their
        // opacity as the giant's own view draws it, scaled to that cap,
        // not raised to 1/mu: that law drives every ringlet but the
        // thinnest to full opacity from 20 degrees down, flattening them
        // to a plain sheet where the giant's own view still shows them.
        float mu = abs(g);
        float cap = mix(GRAZING_OPACITY, 1.0, smoothstep(0.15, 0.45, mu));
        float slant = min(1.0 - pow(max(1.0 - d, 0.0), 1.0 / max(mu, 1e-3)), cap);
        float opened = d * min(1.0, cap / max(uRingOpacity, 1e-3));
        // Where the ringlets are resolved (a pixel spans well under one
        // of the broad ones), as along the rings round a ring moon down to
        // the horizon, they show whatever the elevation.
        float resolved = 1.0 - smoothstep(0.25, 0.8, footprint * 23.0);
        float seen = mix(slant, opened, max(smoothstep(0.01, 0.12, mu), resolved));
        if (seen > 0.002) {
          colour = vec4(ringLight(p, mu, d), seen);
          depth = depthOf(t * ray);
        }
      }
    }
    // Nearly edge on: the ray's path along their plane, from where the
    // camera stands over it, enters them at s; the line where they lie in
    // the view, about a pixel wide.
    vec3 along = ray - g * uNormal;
    float alongLength = length(along);
    if (alongLength > 1e-6) {
      along /= alongLength;
      vec3 foot = camera - height * uNormal;
      float outer = uRingOuter * uRadius;
      float inner = uRingInner * uRadius;
      float fb = dot(foot, along);
      float fc = dot(foot, foot);
      float s = -1.0;
      bool inside = false;
      if (fc <= outer * outer && fc >= inner * inner) {
        // Standing over them (a ring moon): the first of them along the
        // way, past the gap the camera may stand in, in growing steps,
        // then pinned down between the last two by halving: the steps
        // alone moved the line by jumps from one column of pixels to the
        // next, cutting it into dashes.
        inside = true;
        float step = 1e-4 * uRadius;
        float before = 0.0;
        float dist = 0.0;
        for (int k = 0; k < 48; k++) {
          if (ringDensity(length(foot + dist * along) / uRadius) > 0.02) {
            s = dist;
            break;
          }
          before = dist;
          dist += step;
          step *= 1.3;
        }
        if (s > 0.0) {
          float lo = before;
          float hi = s;
          for (int k = 0; k < 14; k++) {
            float mid = 0.5 * (lo + hi);
            if (ringDensity(length(foot + mid * along) / uRadius) > 0.02) hi = mid;
            else lo = mid;
          }
          s = hi;
        }
      } else if (fc > outer * outer) {
        float dOut = fb * fb - fc + outer * outer;
        if (dOut > 0.0) s = -fb - sqrt(dOut);
      } else {
        // Inside the inner edge: out through it.
        s = -fb + sqrt(max(fb * fb - fc + inner * inner, 0.0));
      }
      if (s >= 0.0 && s < hit) {
        // The line's direction height: below the camera by its height at s.
        float atS = -height / sqrt(s * s + height * height);
        float pixel = max(fwidth(g), 1e-7);
        float cover = 1.0 - smoothstep(0.8, 1.6, abs(g - atS) / pixel);
        // Only where the open rings are thinner than a few pixels: seen
        // from afar, their ellipse's minor axis; from over them, the sheet
        // they make from the horizon down to their nearest part.
        float span = inside ? abs(height) / max(s, 1e-6) : outer * abs(height) / max(dot(camera, camera), outer * outer);
        cover *= 1.0 - smoothstep(1.5, 4.0, span / pixel);
        // Its brightness follows the rings' structure where the line of
        // sight skims them: its ray's closest approach to the giant's axis,
        // where it spends the longest among rings of the same radius (a gap
        // there shows as a dip along the line), or, from over them, the
        // radius at which it enters them.
        vec3 p = foot + max(s, 0.0) * along;
        float rLine = fc > outer * outer ? sqrt(max(fc - fb * fb, 0.0)) / uRadius : length(p) / uRadius;
        float wLine = length(camera) * uPixelAngle / uRadius;
        float structure = clamp(ringDensity(rLine) * ringlets(rLine, wLine) / max(uRingOpacity, 1e-3), 0.0, 1.0);
        if (rLine < uRingInner) structure = 1.0;
        float a = cover * min(1.0, 1.5 * uRingOpacity) * mix(1.0, structure, 0.8);
        if (a > colour.a) {
          colour = vec4(ringLight(p, abs(g), uRingOpacity), a);
          depth = depthOf(p - camera);
        }
      }
    }
    if (colour.a <= 0.002) discard;
    gl_FragDepth = depth;
    gl_FragColor = vec4(colour.rgb, colour.a * uFade);
    #include <colorspace_fragment>
  }
`;
