// GLSL of the planet surface, the graticule and the atmosphere.
//
// Every surface vertex is placed from its uv coordinate, on the flat map
// (plane z = 0, 2π × π) and on the unit globe, then mixed by `uMorph`
// (0 = map, 1 = globe). Conventions are documented in ../geo.ts.
//
// `uData` packs the decoded layers, row 0 = north (sampled at 1 − v):
//   r = elevation, g = temperature, b = moisture, a = log10(1 + flow km²).
import { glslRamp } from './colormaps';

/** Layer ids shared with PlanetView (`uLayer`). */
export const LAYER_IDS = { biomes: 0, elevation: 1, temperature: 2, moisture: 3, flow: 4 } as const;
export type Layer = keyof typeof LAYER_IDS;

/** Largest displacement, as a fraction of the planet radius, at full relief. */
export const RELIEF_SCALE = 0.06;

const common = /* glsl */ `
  const float PI = 3.141592653589793;
  uniform float uMorph;
  uniform float uRelief;
  uniform float uCoast;
  uniform float uHasData;
  uniform sampler2D uData;

  vec4 dataAt(vec2 uv) {
    return texture2D(uData, vec2(uv.x, 1.0 - uv.y));
  }
`;

export const surfaceVertex = /* glsl */ `
  ${common}
  uniform float uLift;
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

    // Land rises above the coastline; seas stay flat at the surface.
    float height = 0.0;
    if (uHasData > 0.5) {
      height = max(dataAt(g).r - uCoast, 0.0) * uRelief * ${RELIEF_SCALE.toFixed(4)};
    }
    vec3 position = mix(plane, sphere, uMorph) + normal * (height + uLift);

    vSphere = sphere;
    vNormal = normal;
    vEast = normalize(mix(vec3(1.0, 0.0, 0.0), vec3(cos(lon), 0.0, -sin(lon)), uMorph));
    vNorth = normalize(mix(vec3(0.0, 1.0, 0.0), vec3(-sin(lat) * sin(lon), cos(lat), -sin(lat) * cos(lon)), uMorph));
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

  void main() {
    vec4 data = dataAt(vUv);
    float aboveCoast = data.r - uCoast;
    bool hasData = uHasData > 0.5;
    bool water = hasData && aboveCoast < 0.0;

    vec3 colour;
    if (uTileOn > 0.5) {
      // Detail tiles are drawn on the biome layer only.
      colour = texture2D(uTile, uTexRect.xy + vec2(vLocalUv.x, 1.0 - vLocalUv.y) * uTexRect.zw).rgb;
    } else if (uLayer == 0 || !hasData) {
      colour = texture2D(uMap, vUv).rgb;
    } else if (uLayer == 1) {
      colour = toLinear(ramp_elevation((aboveCoast + 1.0) * 0.5));
    } else if (uLayer == 2) {
      colour = toLinear(ramp_temperature(data.g));
    } else if (uLayer == 3) {
      colour = toLinear(ramp_moisture((data.b + 1.0) * 0.5));
    } else {
      colour = toLinear(ramp_flow(data.a / 7.0));
    }
    if (uHexOn > 0.5) {
      colour = hexColour(colour);
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
      shade = (0.07 + 0.93 * day * strength) * relief;
    } else {
      float globe = 0.35 + 0.75 * lambert;
      shade = mix(relief, globe, uMorph);
    }
    colour *= shade;

    // Sun glint on the sea, on the globe only.
    if (water) {
      vec3 light = uLightMode == 1 ? uSunDir : uLightDir;
      vec3 view = normalize(cameraPosition - vWorld);
      float glint = pow(max(dot(reflect(-light, normal), view), 0.0), 60.0);
      colour += vec3(0.5) * glint * uMorph;
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

export const atmosphereVertex = /* glsl */ `
  varying vec3 vNormal;
  varying vec3 vWorldNormal;
  void main() {
    vNormal = normalize(normalMatrix * normal);
    vWorldNormal = normalize(normal);
    gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0);
  }
`;

// Rim glow of a slightly larger sphere, seen from inside (back faces).
export const atmosphereFragment = /* glsl */ `
  uniform float uOpacity;
  uniform vec3 uColor;
  uniform int uLightMode;
  uniform vec3 uSunDir;
  varying vec3 vNormal;
  varying vec3 vWorldNormal;
  void main() {
    float rim = pow(max(0.0, 0.72 + dot(vNormal, vec3(0.0, 0.0, 1.0))), 4.0);
    float lit = uLightMode == 1 ? smoothstep(-0.3, 0.3, dot(vWorldNormal, uSunDir)) : 1.0;
    gl_FragColor = vec4(uColor, 1.0) * rim * uOpacity * (0.15 + 0.85 * lit);
  }
`;
