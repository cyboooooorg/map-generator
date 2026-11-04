// GLSL of the planet surface, the graticule and the atmosphere.
//
// Every surface vertex is placed from its uv coordinate, on the flat map
// (plane z = 0, 2π × π) and on the unit globe, then mixed by `uMorph`
// (0 = map, 1 = globe). Conventions are documented in ../geo.ts.
//
// `uData` packs the decoded layers, row 0 = north (sampled at 1 − v):
//   r = elevation, g = temperature, b = moisture, a = log10(1 + flow km²).
// `uExtra` packs the layers of raw_data format version 2 and later:
//   r = volcanic zone, g = distance to the sea (km), b = floodplain, a = highland offset.
// `uBiomes` holds the biome id of each pixel (r × 255), for highlighting one biome.
import { glslRamp } from './colormaps';
import { SUN_DEPTH_HEIGHT, SUN_DEPTH_WIDTH } from './sunDepth';

/** Layer ids shared with PlanetView (`uLayer`). */
export const LAYER_IDS = {
  biomes: 0,
  elevation: 1,
  temperature: 2,
  moisture: 3,
  flow: 4,
  volcanic: 5,
  coast: 6,
  floodplain: 7,
} as const;
export type Layer = keyof typeof LAYER_IDS;

/** Largest displacement, as a fraction of the planet radius, at full relief. */
export const RELIEF_SCALE = 0.06;

const common = /* glsl */ `
  const float PI = 3.141592653589793;
  uniform float uMorph;
  uniform float uRelief;
  uniform float uCoast;
  // 1 when the sea is liquid; 0 when it has dried up (barren worlds).
  uniform float uLiquidSea;
  uniform float uHasData;
  uniform sampler2D uData;
  // An irregular body (a small moon): its radius as a share of its mean one.
  uniform float uHasShape;
  uniform sampler2D uShape;
  uniform vec2 uShapeTexel;

  vec4 dataAt(vec2 uv) {
    return texture2D(uData, vec2(uv.x, 1.0 - uv.y));
  }

  float shapeAt(vec2 uv) {
    return uHasShape > 0.5 ? texture2D(uShape, vec2(uv.x, 1.0 - uv.y)).r : 1.0;
  }
`;

/**
 * Seconds for the fastest band of a giant to drift once round the planet
 * (its strongest wind), the others in proportion, backwards for westward
 * winds: a sight, not the true pace.
 */
export const BAND_PERIOD_S = 120;

export const surfaceVertex = /* glsl */ `
  ${common}
  uniform float uLift;
  // Flattening of a giant's globe by its spin (0 for a rocky world).
  uniform float uFlattening;
  // Part of the planet this mesh covers (u0, v0, du, dv): the whole planet
  // for the base surface, one quadtree tile for a detail patch.
  uniform vec4 uTileRect;
  varying vec2 vUv;
  varying vec2 vLocalUv;
  varying vec3 vNormal;
  varying vec3 vSphere;
  varying vec3 vEast;
  varying vec3 vNorth;
  varying vec3 vWorld;

  void main() {
    vec2 g = uTileRect.xy + uv * uTileRect.zw;
    vUv = g;
    vLocalUv = uv;
    float lon = g.x * 2.0 * PI;
    float lat = (g.y - 0.5) * PI;
    vec3 sphere = vec3(cos(lat) * sin(lon), sin(lat), cos(lat) * cos(lon));
    vec3 plane = vec3((g.x - 0.5) * 2.0 * PI, (g.y - 0.5) * PI, 0.0);
    vec3 normal = normalize(mix(vec3(0.0, 0.0, 1.0), sphere, uMorph));
    vec3 eastward = vec3(cos(lon), 0.0, -sin(lon));
    vec3 northward = vec3(-sin(lat) * sin(lon), cos(lat), -sin(lat) * cos(lon));
    // An irregular body: the sphere scaled by its shape, the normal tilted
    // by the shape's slopes (relief and lighting follow it).
    float radius = 1.0;
    if (uHasShape > 0.5) {
      radius = shapeAt(g);
      vec2 dx = vec2(uShapeTexel.x, 0.0);
      vec2 dy = vec2(0.0, uShapeTexel.y);
      float perLon = (shapeAt(g + dx) - shapeAt(g - dx)) / (4.0 * PI * uShapeTexel.x);
      float perLat = (shapeAt(g + dy) - shapeAt(g - dy)) / (2.0 * PI * uShapeTexel.y);
      vec3 tilted = normalize(
        sphere - perLon / (radius * max(cos(lat), 0.05)) * eastward - perLat / radius * northward
      );
      normal = normalize(mix(vec3(0.0, 0.0, 1.0), tilted, uMorph));
      radius = mix(1.0, radius, uMorph);
    }

    // Land rises above the coastline; liquid seas stay flat at the surface,
    // dried-up basins sink below it.
    float height = 0.0;
    if (uHasData > 0.5) {
      float above = dataAt(g).r - uCoast;
      if (uLiquidSea > 0.5) {
        above = max(above, 0.0);
      }
      height = above * uRelief * ${RELIEF_SCALE.toFixed(4)};
    }
    // A spinning giant is flattened at the poles: an ellipsoid, whose normal
    // leans towards the poles.
    if (uFlattening > 0.0) {
      float squash = 1.0 - uFlattening;
      vec3 ellipsoid = normalize(vec3(sphere.x, sphere.y / (squash * squash), sphere.z));
      normal = normalize(mix(vec3(0.0, 0.0, 1.0), ellipsoid, uMorph));
    }
    vec3 position = mix(plane, sphere * radius, uMorph) + normal * (height + uLift);
    position.y *= 1.0 - uFlattening * uMorph;

    vSphere = sphere;
    vNormal = normal;
    vEast = normalize(mix(vec3(1.0, 0.0, 0.0), eastward, uMorph));
    vNorth = normalize(mix(vec3(0.0, 1.0, 0.0), northward, uMorph));
    vec4 world = modelMatrix * vec4(position, 1.0);
    vWorld = world.xyz;
    gl_Position = projectionMatrix * viewMatrix * world;
  }
`;

export const surfaceFragment = /* glsl */ `
  ${common}
  uniform sampler2D uMap;
  uniform int uLayer;
  uniform vec2 uTexel;
  uniform int uLightMode;      // 0 = headlight, 1 = sun
  uniform vec3 uLightDir;      // headlight, world space
  uniform vec3 uSunDir;        // sun, world space
  uniform float uHexOn;
  uniform sampler2D uHex;      // columns × rows: rgb = biome colour, a = river flag
  uniform vec4 uHexGrid;       // columns, rows, hex width (px), hex size (px)
  uniform vec2 uMapSize;       // map width and height in pixels
  uniform float uTileOn;       // 1 on a detail patch with a tile texture
  uniform sampler2D uTile;     // detail tile, row 0 = north
  uniform vec4 uTexRect;       // part of uTile this patch shows (u0, v0, du, dv), v down
  uniform float uHasExtra;
  uniform sampler2D uExtra;
  uniform float uCoastMax;     // cap of the distance to the sea, km
  uniform float uHasBiomes;
  uniform sampler2D uBiomes;
  uniform float uHighlight;    // biome id to highlight, or -1
  uniform float uVegetation[64]; // share of vegetation of each biome id
  uniform vec2 uVegShift;      // vegetation recoloured for a cool (x) or hot (y) star, 0 to 1
  uniform float uSeaContours;  // 1 draws the depth contours under water
  uniform float uLandContours; // 1 draws the altitude contours on land
  uniform float uTropics;      // 1 draws the equator, tropics and polar circles
  uniform float uTilt;         // axial tilt, degrees
  // Bodies whose shadow can fall on this one (eclipses): centre (xyz) and
  // radius (w) in radii of this body; the sun's angular radius, radians.
  uniform vec4 uCasters[4];
  uniform float uCasterCount;
  uniform float uSunRadius;
  uniform float uCloudsOn;     // cloud opacity: 0 hides them (toggle, zoom fade)
  uniform sampler2D uClouds;   // cloud cover in r, row 0 = north (sampled at 1 - v)
  uniform vec3 uCloudTint;     // linear colour of the clouds
  uniform vec3 uLightTint;     // colour of the light ("Star light"), white otherwise
  uniform float uCloudShift;   // eastward drift of the cloud layer, in turns
  // A giant: its moisture layer holds the wind (a share of the strongest),
  // its temperature range is its own, and its bands drift each at its own
  // speed (uWind: the wind every 2° from the south pole, a share of the
  // strongest, so that narrow jets turning either way keep their sign;
  // uBandTime: seconds of drift).
  uniform float uGiant;
  uniform vec2 uTempRange;     // °C at the bottom of the ramp, and its span
  uniform float uWind[91];
  uniform float uBandTime;
  uniform float uHotGlow;      // a hot giant's night-side glow, 0 to 1
  // A giant's rings, for their shadow: edges and gaps in planet radii, in
  // the equatorial plane (y = 0); no rings when uRingOuter is 0.
  uniform float uRingInner;
  uniform float uRingOuter;
  uniform float uRingOpacity;
  uniform vec2 uRingGaps[4];
  uniform float uRingGapCount;
  varying vec2 vUv;
  varying vec2 vLocalUv;
  varying vec3 vNormal;
  varying vec3 vSphere;
  varying vec3 vEast;
  varying vec3 vNorth;
  varying vec3 vWorld;

  ${glslRamp('elevation')}
  ${glslRamp('temperature')}
  ${glslRamp('moisture')}
  ${glslRamp('flow')}
  ${glslRamp('volcanic')}
  ${glslRamp('coast')}
  ${glslRamp('floodplain')}
  ${glslRamp('wind')}

  // The wind of a giant at the latitude of map row v (0 south, 1 north).
  float bandWind(float v) {
    float at = clamp(v, 0.0, 1.0) * 90.0;
    int i = int(floor(min(at, 89.999)));
    float f = at - float(i);
    float a = 0.0;
    float b = 0.0;
    for (int k = 0; k < 91; k++) {
      if (k == i) a = uWind[k];
      if (k == i + 1) b = uWind[k];
    }
    return mix(a, b, f);
  }

  // Depth contours: a pale blue mixed over the sea (linear colour, share).
  // world.png darkens its contours instead, which barely shows on the dark
  // surface palette of open water.
  const vec3 CONTOUR_COLOUR = vec3(0.45, 0.6, 0.72);
  const float CONTOUR_OPACITY = 0.45;
  // Altitude contours: topographic brown on land, every LAND_STEP above the
  // coastline (elevation units), every LAND_INDEX_EVERY-th line stronger.
  const vec3 LAND_CONTOUR_COLOUR = vec3(0.16, 0.08, 0.03);
  const float LAND_STEP = 0.05;
  const float LAND_INDEX_EVERY = 5.0;
  const float LAND_OPACITY = 0.3;
  const float LAND_INDEX_OPACITY = 0.6;

  // Coverage of a line about one screen pixel wide where elevation e
  // crosses level, e changing by width per pixel.
  float contourAt(float e, float level, float width) {
    return 1.0 - smoothstep(0.5, 1.5, abs(e - level) / width);
  }

  // Coverage of the depth contours, at mapgen::export::CONTOUR_LEVELS.
  float contours(float e) {
    float width = max(fwidth(e), 1e-6);
    float line = 0.0;
    line = max(line, contourAt(e, -0.45, width));
    line = max(line, contourAt(e, -0.15, width));
    line = max(line, contourAt(e, 0.0, width));
    line = max(line, contourAt(e, 0.15, width));
    line = max(line, contourAt(e, 0.30, width));
    line = max(line, contourAt(e, 0.45, width));
    line = max(line, contourAt(e, 0.60, width));
    line = max(line, contourAt(e, 0.75, width));
    line = max(line, contourAt(e, 0.90, width));
    return line;
  }

  // Coverage of lines every spacing of h, h changing by width per pixel.
  float periodicLines(float h, float spacing, float width) {
    float offset = abs(fract(h / spacing + 0.5) - 0.5) * spacing;
    return 1.0 - smoothstep(0.5, 1.5, offset / width);
  }

  // Reference parallels (equator, tropics, polar circles): width and dark
  // edge in screen pixels, dashes per turn, and their colours (linear, the
  // colours of world.png's lines).
  const float REFERENCE_WIDTH = 3.0;
  const float REFERENCE_EDGE = 1.0;
  const float REFERENCE_DASHES = 240.0;
  const vec3 EQUATOR_COLOUR = vec3(0.72, 0.03, 0.03);
  const vec3 TROPIC_COLOUR = vec3(0.72, 0.30, 0.0);
  const vec3 POLAR_COLOUR = vec3(0.0, 0.58, 0.87);

  // Draws a dashed reference parallel at latitude lat (degrees) over colour:
  // a coloured core REFERENCE_WIDTH pixels wide with a dark edge, so it
  // reads on light and dark ground, ice and clouds alike.
  vec3 referenceLine(vec3 colour, float lat, vec3 lineColour) {
    float v = (lat + 90.0) / 180.0;
    float pixels = abs(vUv.y - v) / max(fwidth(vUv.y), 1e-7);
    float dash = step(0.4, fract(vUv.x * REFERENCE_DASHES));
    float core = 1.0 - smoothstep(0.5 * REFERENCE_WIDTH - 0.5, 0.5 * REFERENCE_WIDTH + 0.5, pixels);
    float edge = 1.0 - smoothstep(
      0.5 * REFERENCE_WIDTH + REFERENCE_EDGE - 0.5,
      0.5 * REFERENCE_WIDTH + REFERENCE_EDGE + 0.5,
      pixels
    );
    colour = mix(colour, colour * 0.25, 0.8 * edge * dash);
    return mix(colour, lineColour, core * dash);
  }

  // Brightness of lava at night, relative to its day colour.
  const float LAVA_GLOW = 0.9;
  // Sun glint on liquid seas: exponent of the lobe (lower is broader) and
  // peak brightness added.
  const float GLINT_SHARPNESS = 18.0;
  const float GLINT_STRENGTH = 0.12;

  vec3 toLinear(vec3 srgb) {
    return pow(srgb, vec3(2.2));
  }

  // Hex containing map pixel p (x east, y south from the north pole), as in
  // mapgen::hex::HexGrid::locate: pointy-top, odd-r offset, wrapping columns.
  vec2 hexLocate(vec2 p) {
    float size = uHexGrid.w;
    float x = p.x - 0.5 * uHexGrid.z;
    float y = p.y - size;
    float q = (sqrt(3.0) / 3.0 * x - y / 3.0) / size;
    float r = (2.0 / 3.0 * y) / size;
    vec3 cube = vec3(q, -q - r, r);
    vec3 rounded = floor(cube + 0.5);
    vec3 diff = abs(rounded - cube);
    if (diff.x > diff.y && diff.x > diff.z) {
      rounded.x = -rounded.y - rounded.z;
    } else if (diff.y <= diff.z) {
      rounded.z = -rounded.x - rounded.y;
    }
    float row = clamp(rounded.z, 0.0, uHexGrid.y - 1.0);
    float col = mod(rounded.x + floor(rounded.z / 2.0), uHexGrid.x);
    return vec2(col, row);
  }

  vec3 hexColour(vec3 base) {
    vec2 p = vec2(vUv.x * uMapSize.x, (1.0 - vUv.y) * uMapSize.y);
    vec2 cell = hexLocate(p);
    vec4 hex = texture2D(uHex, (cell + 0.5) / uHexGrid.xy);
    float size = uHexGrid.w;
    vec2 centre = vec2((cell.x + 0.5 + 0.5 * mod(cell.y, 2.0)) * uHexGrid.z, size + cell.y * 1.5 * size);
    vec2 local = p - centre;
    local.x = mod(local.x + 0.5 * uMapSize.x, uMapSize.x) - 0.5 * uMapSize.x;
    vec2 a = abs(local);
    // Distance to the hex border (pointy-top), in map pixels.
    float edge = sqrt(3.0) / 2.0 * size - max(a.x, 0.5 * a.x + sqrt(3.0) / 2.0 * a.y);
    float line = fwidth(p.x) * 1.2;
    vec3 colour = toLinear(hex.rgb);
    if (hex.a > 0.5 && length(local) < 0.3 * size) {
      colour = toLinear(vec3(0.16, 0.37, 0.84));
    }
    return mix(colour * 0.55, colour, smoothstep(0.0, line, edge));
  }

  // Share of the sunlight hidden at a point of the unit sphere (see
  // lib/skyBodies.ts, eclipse).
  float eclipse(vec3 point) {
    float hidden = 0.0;
    for (int i = 0; i < 4; i++) {
      if (float(i) >= uCasterCount) break;
      vec3 c = uCasters[i].xyz - point;
      float t = dot(c, uSunDir);
      if (t <= 0.0) continue;
      float d = length(c - t * uSunDir);
      float spread = t * tan(uSunRadius);
      float radius = uCasters[i].w;
      float umbra = max(radius - spread, 0.0);
      float penumbra = radius + spread;
      float x = clamp((penumbra - d) / max(penumbra - umbra, 1e-6), 0.0, 1.0);
      float most = radius >= spread ? 1.0 : (radius / spread) * (radius / spread);
      hidden = max(hidden, x * x * (3.0 - 2.0 * x) * most);
    }
    return hidden;
  }

  // Share of the sunlight a giant's rings hide at a point of the unit
  // sphere: where the ray towards the sun crosses the ring plane, the
  // rings' density there (thinner at their edges, none in their gaps).
  float ringShadow(vec3 point) {
    if (uRingOuter <= 0.0 || abs(uSunDir.y) < 1e-4) return 0.0;
    float t = -point.y / uSunDir.y;
    if (t <= 0.0) return 0.0;
    float r = length((point + t * uSunDir).xz);
    if (r < uRingInner || r > uRingOuter) return 0.0;
    float across = (r - uRingInner) / (uRingOuter - uRingInner);
    float density = uRingOpacity * smoothstep(0.0, 0.08, across) * smoothstep(1.0, 0.92, across);
    for (int k = 0; k < 4; k++) {
      if (float(k) >= uRingGapCount) break;
      density *= smoothstep(0.0, 1.0, abs(r - uRingGaps[k].x) / max(uRingGaps[k].y * 0.5, 1e-4));
    }
    return 0.85 * density;
  }

  void main() {
    vec4 data = dataAt(vUv);
    float aboveCoast = data.r - uCoast;
    bool hasData = uHasData > 0.5;
    bool water = hasData && uLiquidSea > 0.5 && aboveCoast < 0.0;

    // The surface colours (detail tile or base map), sampled once: they are
    // the biome layer, and tell lava and magma apart on every layer (below).
    // Detail tiles are drawn on the biome layer only.
    vec3 surface = uTileOn > 0.5
      ? texture2D(uTile, uTexRect.xy + vec2(vLocalUv.x, 1.0 - vLocalUv.y) * uTexRect.zw).rgb
      : texture2D(uMap, uGiant > 0.5 ? vec2(fract(vUv.x - uBandTime * bandWind(vUv.y) / ${BAND_PERIOD_S.toFixed(1)}), vUv.y) : vUv).rgb;
    vec3 colour;
    if (uTileOn > 0.5 || uLayer == 0 || !hasData) {
      colour = surface;
    } else if (uLayer == 1) {
      colour = toLinear(ramp_elevation((aboveCoast + 1.0) * 0.5));
    } else if (uLayer == 2) {
      colour = toLinear(ramp_temperature((data.g - uTempRange.x) / uTempRange.y));
    } else if (uLayer == 3) {
      colour = uGiant > 0.5
        ? toLinear(ramp_wind((data.b + 1.0) * 0.5))
        : toLinear(ramp_moisture((data.b + 1.0) * 0.5));
    } else if (uLayer == 4) {
      colour = toLinear(ramp_flow(data.a / 7.0));
    } else if (uHasExtra < 0.5) {
      colour = surface;
    } else {
      vec4 extra = texture2D(uExtra, vec2(vUv.x, 1.0 - vUv.y));
      if (uLayer == 5) {
        colour = toLinear(ramp_volcanic(extra.r));
      } else if (uLayer == 6) {
        colour = toLinear(ramp_coast(sqrt(clamp(extra.g / uCoastMax, 0.0, 1.0))));
      } else {
        colour = toLinear(ramp_floodplain(extra.b));
      }
    }
    // Vegetation under another star ("Star light"): photosynthesis adapts
    // to the light, so leaves turn dark, near black, under red dwarfs and
    // yellow under F stars (Kiang et al. 2007). The biome map and the
    // detail tiles only, by the share of vegetation of each biome.
    if ((uVegShift.x > 0.0 || uVegShift.y > 0.0) && uHasBiomes > 0.5 && (uTileOn > 0.5 || uLayer == 0)) {
      float id = floor(texture2D(uBiomes, vec2(vUv.x, 1.0 - vUv.y)).r * 255.0 + 0.5);
      float plants = uVegetation[int(min(id, 63.0))];
      if (plants > 0.0) {
        float lum = dot(colour, vec3(0.2126, 0.7152, 0.0722));
        vec3 dark = vec3(0.55, 0.28, 0.42) * lum;
        vec3 gold = vec3(1.35, 1.05, 0.3) * lum;
        vec3 adapted = mix(mix(colour, dark, uVegShift.x), gold, uVegShift.y);
        colour = mix(colour, adapted, plants);
      }
    }
    if (uHexOn > 0.5) {
      colour = hexColour(colour);
    }
    // One biome highlighted (legend hover): the others fade to dim grey.
    if (uHighlight >= 0.0 && uHasBiomes > 0.5) {
      float id = floor(texture2D(uBiomes, vec2(vUv.x, 1.0 - vUv.y)).r * 255.0 + 0.5);
      if (abs(id - uHighlight) > 0.5) {
        float grey = dot(colour, vec3(0.2126, 0.7152, 0.0722));
        colour = mix(vec3(grey), colour, 0.15) * 0.3;
      }
    }

    // Depth contours under water, at the levels of world.png: a line about
    // one screen pixel wide wherever the elevation crosses a level, so it
    // stays sharp at every zoom (the elevation is the base layer,
    // interpolated).
    // fwidth needs every pixel of a quad: it is taken in branches on
    // uniforms only (the same for every pixel), never on the pixel's own
    // depth or height, and skipped altogether when the lines are off.
    if (uSeaContours > 0.5 && uHasData > 0.5) {
      float contour = contours(data.r);
      if (aboveCoast < 0.0) {
        colour = mix(colour, CONTOUR_COLOUR, CONTOUR_OPACITY * contour);
      }
    }
    // Altitude contours on land, from the coastline up: regular lines and,
    // every LAND_INDEX_EVERY of them, stronger index lines.
    if (uLandContours > 0.5 && uHasData > 0.5) {
      float landWidth = max(fwidth(aboveCoast), 1e-6);
      float landLine = periodicLines(aboveCoast, LAND_STEP, landWidth);
      float indexLine = periodicLines(aboveCoast, LAND_STEP * LAND_INDEX_EVERY, landWidth);
      if (aboveCoast > 0.5 * LAND_STEP) {
        float share = max(LAND_OPACITY * landLine, LAND_INDEX_OPACITY * indexLine);
        colour = mix(colour, LAND_CONTOUR_COLOUR, share);
      }
    }

    // Lava and magma glow on the night side (sun mode, biome map and detail
    // tiles). They are told by their colour, red with almost no green, so
    // detail tiles glow at their own resolution; no other surface colour
    // comes close (deserts and rust have green above half their red).
    // Read from the surface colours whatever the layer shown, so the glint
    // stays off magma on every layer too.
    float lava = (1.0 - smoothstep(0.12, 0.35, surface.g / max(surface.r, 1e-4))) * smoothstep(0.02, 0.1, surface.r);
    float glow = 0.0;
    if (uLightMode == 1 && uHexOn < 0.5 && (uTileOn > 0.5 || uLayer == 0)) {
      float night = 1.0 - smoothstep(-0.08, 0.15, dot(normalize(vSphere), uSunDir));
      glow = LAVA_GLOW * lava * night;
    }
    vec3 emitted = colour * glow;
    // A hot giant's clouds glow deep red on its night side, as hot coals.
    if (uHotGlow > 0.0 && uLightMode == 1 && uHexOn < 0.5) {
      float dark = 1.0 - smoothstep(-0.08, 0.15, dot(normalize(vSphere), uSunDir));
      emitted += vec3(0.9, 0.2, 0.05) * uHotGlow * dark;
    }

    // Relief shading: tilt the normal by the elevation slope (land only).
    // The biome map and the detail tiles are hillshaded already: shading
    // them again would double every slope.
    vec3 normal = normalize(vNormal);
    bool shaded = uTileOn > 0.5 || uLayer == 0;
    if (hasData && uRelief > 0.0 && !water && !shaded) {
      float east = dataAt(vUv + vec2(uTexel.x, 0.0)).r - dataAt(vUv - vec2(uTexel.x, 0.0)).r;
      float north = dataAt(vUv + vec2(0.0, uTexel.y)).r - dataAt(vUv - vec2(0.0, uTexel.y)).r;
      normal = normalize(normal - uRelief * 40.0 * (east * vEast + north * vNorth));
    }

    // Relief is always shaded by the headlight, relative to flat ground, so
    // slopes read the same in every view; the sun only sets day and night.
    float lambert = max(dot(normal, uLightDir), 0.0);
    float flatLambert = max(dot(normalize(vNormal), uLightDir), 0.2);
    float relief = min(lambert / flatLambert, 1.5);
    float shade;
    if (uLightMode == 1) {
      // Day and night come from the sphere normal, on the globe and the map alike.
      float sunHeight = dot(normalize(vSphere), uSunDir);
      float day = smoothstep(-0.08, 0.15, sunHeight);
      // Low sun is dimmer than high sun.
      float strength = 0.55 + 0.45 * clamp(sunHeight, 0.0, 1.0);
      // A moon's shadow on its planet, the planet's on its moon.
      float sunlit = uCasterCount > 0.0 ? 1.0 - eclipse(normalize(vSphere)) : 1.0;
      // A giant's rings shadow a band of it.
      sunlit *= 1.0 - ringShadow(normalize(vSphere));
      shade = (0.07 + 0.93 * day * strength * sunlit) * relief;
    } else {
      float globe = 0.35 + 0.75 * lambert;
      shade = mix(relief, globe, uMorph);
    }
    // Clouds: their shadow on the ground, offset away from the light (on
    // the globe), then the cloud layer itself, lit like flat ground.
    vec3 light = uLightMode == 1 ? uSunDir : uLightDir;
    float cover = 0.0;
    if (uCloudsOn > 0.0) {
      // Opacity of the cover: thin clouds are translucent veils, only
      // thick ones hide the ground.
      vec2 cloudUv = vec2(vUv.x - uCloudShift, 1.0 - vUv.y);
      cover = smoothstep(0.3, 1.0, texture2D(uClouds, cloudUv).r) * 0.85 * uCloudsOn;
      float lat = (vUv.y - 0.5) * PI;
      vec2 towardsLight = vec2(dot(light, vEast) / max(cos(lat), 0.1) / (2.0 * PI), dot(light, vNorth) / PI);
      float shadow = smoothstep(0.3, 1.0, texture2D(uClouds, cloudUv + vec2(1.0, -1.0) * towardsLight * 0.004 * uMorph).r) * uCloudsOn;
      colour *= 1.0 - 0.45 * shadow;
    }
    colour *= shade * uLightTint;
    colour += emitted;

    // Sun glint on the sea, on the globe only, dimmed under clouds. Waves
    // spread it into a broad, faint sheen (a tight bright spot made the
    // globe look like a polished ball); viscous magma, crusted over, barely
    // reflects at all.
    if (water) {
      vec3 view = normalize(cameraPosition - vWorld);
      float glint = pow(max(dot(reflect(-light, normal), view), 0.0), GLINT_SHARPNESS);
      float strength = GLINT_STRENGTH * (1.0 - 0.9 * lava);
      colour += uLightTint * strength * glint * uMorph * (1.0 - cover);
    }

    if (cover > 0.0) {
      float cloudShade;
      if (uLightMode == 1) {
        float sunHeight = dot(normalize(vSphere), uSunDir);
        cloudShade = 0.04 + 0.96 * smoothstep(-0.08, 0.15, sunHeight) * (0.6 + 0.4 * clamp(sunHeight, 0.0, 1.0));
      } else {
        cloudShade = mix(1.0, 0.35 + 0.75 * max(dot(normalize(vNormal), uLightDir), 0.0), uMorph);
      }
      colour = mix(colour, uCloudTint * cloudShade * uLightTint, cover);
    }

    // Reference parallels last, over the clouds, unshaded: they are map
    // marks, not part of the ground.
    if (uTropics > 0.5) {
      colour = referenceLine(colour, 0.0, EQUATOR_COLOUR);
      colour = referenceLine(colour, uTilt, TROPIC_COLOUR);
      colour = referenceLine(colour, -uTilt, TROPIC_COLOUR);
      colour = referenceLine(colour, 90.0 - uTilt, POLAR_COLOUR);
      colour = referenceLine(colour, uTilt - 90.0, POLAR_COLOUR);
    }

    gl_FragColor = vec4(colour, 1.0);
    #include <colorspace_fragment>
  }
`;

export const lineFragment = /* glsl */ `
  uniform vec3 uColor;
  uniform float uOpacity;
  void main() {
    gl_FragColor = vec4(uColor, uOpacity);
  }
`;

// Stars: points at infinity. Only the camera's rotation moves them, so
// they stay put while the camera orbits the planet; drawn at the far plane
// (z = w), behind everything, without writing depth.
export const starsVertex = /* glsl */ `
  attribute vec3 colour;
  attribute float size;
  uniform float uPixelRatio;
  uniform float uOpacity;
  varying vec3 vColour;
  void main() {
    vec4 clip = projectionMatrix * vec4(mat3(viewMatrix) * position * 10.0, 1.0);
    gl_Position = clip.xyww;
    gl_PointSize = size * uPixelRatio;
    vColour = colour * uOpacity;
  }
`;

// A soft round point, added to what is behind it.
export const starsFragment = /* glsl */ `
  varying vec3 vColour;
  void main() {
    float r = length(gl_PointCoord - 0.5) * 2.0;
    float a = 1.0 - smoothstep(0.35, 1.0, r);
    if (a <= 0.0) discard;
    gl_FragColor = vec4(vColour * a, 1.0);
    #include <colorspace_fragment>
  }
`;

// The host star: one point at infinity in the sun's direction, like the
// star field, sized in drawing-buffer pixels. A disk of the star's apparent
// diameter, slightly darker at its limb, inside a glow.
export const hostStarVertex = /* glsl */ `
  uniform float uSize;
  void main() {
    vec4 clip = projectionMatrix * vec4(mat3(viewMatrix) * position * 10.0, 1.0);
    gl_Position = clip.xyww;
    gl_PointSize = uSize;
  }
`;

export const hostStarFragment = /* glsl */ `
  uniform vec3 uColor;
  uniform float uSize;     // point size, px
  uniform float uDisk;     // disk diameter, px
  uniform float uGlow;     // glow strength
  uniform float uOpacity;
  void main() {
    // Distance from the centre in px.
    float r = length(gl_PointCoord - 0.5) * uSize;
    float radius = 0.5 * uDisk;
    float inside = 1.0 - smoothstep(radius - 0.75, radius + 0.75, r);
    float limb = 1.0 - 0.35 * pow(clamp(r / max(radius, 1e-3), 0.0, 1.0), 2.0);
    // Fades to nothing before the edge of the square point.
    float fade = 1.0 - smoothstep(0.25 * uSize, 0.5 * uSize, r);
    float glow = uGlow * exp(-max(r - radius, 0.0) / max(0.08 * uSize, 1.0)) * fade;
    vec3 colour = uColor * (inside * limb * 1.4 + glow * (1.0 - inside));
    if (max(colour.r, max(colour.g, colour.b)) < 0.002) discard;
    gl_FragColor = vec4(colour * uOpacity, 1.0);
    #include <colorspace_fragment>
  }
`;

// A body in the sky (the planet seen from its moon, a moon from its
// planet): a disk facing the view at infinity, `uHalf` (the tangent of its
// angular radius) across, drawn behind everything like the host star.
export const skyBodyVertex = /* glsl */ `
  uniform vec3 uDir;
  uniform vec3 uRight;
  uniform vec3 uUp;
  uniform float uHalf;
  varying vec2 vP;
  void main() {
    vP = position.xy * 2.0;
    vec3 d = uDir + (vP.x * uRight + vP.y * uUp) * uHalf;
    vec4 clip = projectionMatrix * vec4(mat3(viewMatrix) * d * 10.0, 1.0);
    gl_Position = clip.xyww;
  }
`;

// The body as a lit sphere: its visible hemisphere faces the viewer, its
// day side the sun (phases), mapped with its surface when it has one.
export const skyBodyFragment = /* glsl */ `
  const float PI = 3.141592653589793;
  uniform vec3 uDir;
  uniform vec3 uRight;
  uniform vec3 uUp;
  uniform vec3 uSunDir;
  uniform vec3 uColor;
  uniform float uHasMap;
  uniform sampler2D uMap;
  uniform float uOpacity;
  varying vec2 vP;
  void main() {
    float r2 = dot(vP, vP);
    if (r2 > 1.0) discard;
    vec3 n = normalize(vP.x * uRight + vP.y * uUp - sqrt(1.0 - r2) * uDir);
    vec3 colour = uColor;
    if (uHasMap > 0.5) {
      float lon = atan(n.x, n.z);
      float lat = asin(clamp(n.y, -1.0, 1.0));
      colour = texture2D(uMap, vec2(fract(lon / (2.0 * PI)), lat / PI + 0.5)).rgb;
    }
    float lit = max(dot(n, uSunDir), 0.0);
    // A soft edge, and a faint night side.
    float edge = 1.0 - smoothstep(0.96, 1.0, r2);
    gl_FragColor = vec4(colour * (0.015 + 0.985 * lit) * uOpacity * edge, 1.0);
    #include <colorspace_fragment>
  }
`;

export const atmosphereVertex = /* glsl */ `
  varying vec3 vWorld;
  void main() {
    vec4 world = modelMatrix * vec4(position, 1.0);
    vWorld = world.xyz;
    gl_Position = projectionMatrix * viewMatrix * world;
  }
`;

// Single scattering through the atmosphere shell around the globe (radius
// 1, centred on the origin), drawn after the surface: for each pixel the
// view ray is marched through the shell (and stopped at the ground), each
// sample lit by sunlight attenuated along its own ray to the sun. Rayleigh
// (gas, per channel) and Mie (aerosols, one extinction tinted by their
// albedo) scattering. The output is premultiplied in-scattered light plus,
// in alpha, the transmittance of the ray: blended as dst × alpha + src, it
// veils the ground (an opaque Venus hides its surface) and draws the sky.
//
// The optical depth towards the sun is not marched but looked up in a
// small table (uSunDepth, see sunDepth.ts) of the Chapman function of each
// species, an exponential atmosphere over a sphere: one texture fetch per
// sample instead of eight steps (sixteen exp) towards the sun.
//
// The shell is drawn twice, from its far side, split by the depth test
// (uGround): where the surface is in front of the far side, the ray stops
// at the ground and veils it; elsewhere it crosses the whole shell, the sky.
// The ground of the shader is the unit sphere, but displaced relief rises
// above it at the limb: there the ray stops at its closest point to the
// planet, where such a peak stands, instead of drawing the sky over it.
export const atmosphereFragment = /* glsl */ `
  uniform float uOpacity;
  uniform int uLightMode;
  uniform vec3 uSunDir;
  uniform vec3 uLightDir;
  uniform float uTop;
  uniform vec3 uBetaR;
  uniform float uBetaM;
  uniform vec3 uMieAlbedo;
  uniform float uHeightR;
  uniform float uHeightM;
  uniform float uG;
  uniform vec3 uLightTint;     // colour of the sunlight entering the shell
  uniform float uGround;       // 1: the surface pass, 0: the sky pass
  uniform sampler2D uSunDepth; // optical depth towards the sun over density (sunDepth.ts)
  varying vec3 vWorld;

  const int VIEW_STEPS = 16;
  // From [0, 1] to the centres of the first and last texels.
  const vec2 SUN_DEPTH_SCALE = vec2(${(SUN_DEPTH_WIDTH - 1) / SUN_DEPTH_WIDTH}, ${(SUN_DEPTH_HEIGHT - 1) / SUN_DEPTH_HEIGHT});
  const vec2 SUN_DEPTH_OFFSET = vec2(${0.5 / SUN_DEPTH_WIDTH}, ${0.5 / SUN_DEPTH_HEIGHT});
  const float PI = 3.14159265;
  const float EXPOSURE = 1.6;

  // Distances to the two intersections of a ray with a sphere at the origin
  // (far > near; no hit gives near > far).
  vec2 sphere(vec3 origin, vec3 dir, float radius) {
    float b = dot(origin, dir);
    float c = dot(origin, origin) - radius * radius;
    float h = b * b - c;
    if (h < 0.0) return vec2(1e9, -1e9);
    h = sqrt(h);
    return vec2(-b - h, -b + h);
  }

  void main() {
    vec3 origin = cameraPosition;
    vec3 dir = normalize(vWorld - cameraPosition);
    vec3 sun = uLightMode == 1 ? uSunDir : uLightDir;
    vec2 shell = sphere(origin, dir, uTop);
    if (shell.y <= 0.0) discard;
    vec2 ground = sphere(origin, dir, 1.0);
    bool hitsGround = ground.x > 0.0 && ground.x < ground.y;
    float near = max(shell.x, 0.0);
    bool surface = uGround > 0.5;
    // Relief above the unit sphere: the peak stands near the limb point.
    float limb = max(-dot(origin, dir), near);
    float far = hitsGround ? min(shell.y, ground.x) : surface ? min(shell.y, limb) : shell.y;
    float ds = (far - near) / float(VIEW_STEPS);

    vec3 sumR = vec3(0.0);
    vec3 sumM = vec3(0.0);
    float depthR = 0.0;
    float depthM = 0.0;
    for (int i = 0; i < VIEW_STEPS; i++) {
      vec3 p = origin + dir * (near + (float(i) + 0.5) * ds);
      float r = max(length(p), 1.0);
      float h = r - 1.0;
      float densityR = exp(-h / uHeightR);
      float densityM = exp(-h / uHeightM);
      float dR = densityR * ds;
      float dM = densityM * ds;
      depthR += dR;
      depthM += dM;
      // In the planet's shadow (below the ray grazing the ground), no
      // sunlight reaches this sample.
      float mu = dot(p, sun) / r;
      float grazing = -sqrt(max(1.0 - 1.0 / (r * r), 0.0));
      if (mu < grazing) continue;
      vec2 at = vec2(sqrt((mu - grazing) / (1.0 - grazing)), sqrt(h / (uTop - 1.0)));
      vec2 factor = texture2D(uSunDepth, at * SUN_DEPTH_SCALE + SUN_DEPTH_OFFSET).rg;
      float sunR = densityR * factor.r;
      float sunM = densityM * factor.g;
      vec3 attenuation = exp(-(uBetaR * (depthR + sunR) + uBetaM * (depthM + sunM)));
      sumR += dR * attenuation;
      sumM += dM * attenuation;
    }
    float mu = dot(dir, sun);
    float phaseR = 3.0 / (16.0 * PI) * (1.0 + mu * mu);
    float g2 = uG * uG;
    float phaseM = 3.0 / (8.0 * PI) * ((1.0 - g2) * (1.0 + mu * mu)) /
      ((2.0 + g2) * pow(1.0 + g2 - 2.0 * uG * mu, 1.5));
    vec3 light = (sumR * uBetaR * phaseR + sumM * uBetaM * uMieAlbedo * phaseM) * uLightTint;
    // Soft exposure: bright skies saturate instead of clipping.
    vec3 colour = 1.0 - exp(-EXPOSURE * 4.0 * PI * light);
    vec3 transmittance = exp(-(uBetaR * depthR + uBetaM * depthM));
    float behind = surface ? dot(transmittance, vec3(1.0 / 3.0)) : 1.0;
    gl_FragColor = vec4(colour * uOpacity, mix(1.0, behind, uOpacity));
  }
`;
