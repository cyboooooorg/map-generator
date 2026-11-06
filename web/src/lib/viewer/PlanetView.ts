// three.js view of a world: one mesh morphing between the flat map and the
// globe in its vertex shader, data layers coloured on the GPU, relief, light
// (headlight or sun), atmosphere, graticule, hex overlay, biome highlight,
// measured paths, picking and hovering, screenshots and camera sharing.
// Coordinate conventions are documented in ../geo.ts, shaders in ./shaders.ts.
import { axesOnScreen, type ScreenAxis } from './axes';
import {
  AddEquation,
  AdditiveBlending,
  BackSide,
  BufferAttribute,
  BufferGeometry,
  ClampToEdgeWrapping,
  Color,
  CustomBlending,
  DataTexture,
  DataUtils,
  DoubleSide,
  FrontSide,
  GreaterDepth,
  Float32BufferAttribute,
  HalfFloatType,
  IcosahedronGeometry,
  InstancedBufferAttribute,
  InstancedBufferGeometry,
  LinearFilter,
  LinearMipmapLinearFilter,
  LessEqualDepth,
  LineBasicMaterial,
  LineSegments,
  Matrix4,
  Mesh,
  MeshBasicMaterial,
  NoColorSpace,
  MOUSE,
  NotEqualStencilFunc,
  NearestFilter,
  OneFactor,
  PerspectiveCamera,
  PlaneGeometry,
  Points,
  RingGeometry,
  Quaternion,
  Raycaster,
  RedFormat,
  RepeatWrapping,
  RGFormat,
  RGBAFormat,
  Scene,
  ShaderMaterial,
  SphereGeometry,
  SrcAlphaFactor,
  SRGBColorSpace,
  Texture,
  UnsignedByteType,
  Vector2,
  Vector3,
  Vector4,
  WebGLRenderer,
  type IUniform,
} from 'three';
import { OrbitControls } from 'three/examples/jsm/controls/OrbitControls.js';
import { BodyGuides, type GuidePoints, labelIndex, type HostGuide } from './bodyGuides';
import type { HexMap } from '../api/types/HexMap';
import {
  greatCircleArc,
  type LonLat,
  lonLatToUv,
  lonLatToSphere,
  planeToLonLat,
  rayPlaneZ0,
  raySphere,
  sphereToLonLat,
  type Vec3,
} from '../geo';
import { type Grey8, loadGrey8, type WorldLayers } from '../layers';
import { decodePng } from '../layers/png';
import { IDENTITY_AXES, inFrontOfGlobe, MAX_CASTERS, type ShadowCaster, type SkyBody } from '../skyBodies';
import type { Preview } from '../api/types/Preview';
import { type LoadProgress, readWithProgress } from '../progress';
import { liftSky, type Sky } from '../sky';
import { fullPixelRatio, pixelRatio } from './pixelRatio';
import {
  altitudeForMapHeight,
  clampMapCentre,
  GLOBE_DISTANCE,
  lerpLonLat,
  MAP_DISTANCE,
  mapHeightForAltitude,
  mapPoint,
  morphCamera,
} from './viewTransfer';
import { SUN_DEPTH_HEIGHT, SUN_DEPTH_WIDTH, sunDepthTable } from './sunDepth';
import { downsample, fitFactor, releaseOnUpload } from './textures';
import { DETAIL_STENCIL, MAX_TEXTURES, TileLayer } from './TileLayer';
import {
  atmosphereFragment,
  atmosphereHaloFragment,
  atmosphereVertex,
  LAYER_IDS,
  type Layer,
  lineFragment,
  hostStarFragment,
  skyBodyFragment,
  skyBodyVertex,
  previewGlobeFragment,
  previewGlobeVertex,
  SKY_ORBIT_DASH,
  skyOrbitDash,
  skyOrbitFragment,
  skyOrbitVertex,
  hostStarVertex,
  starsFragment,
  starsVertex,
  surfaceFragment,
  surfaceVertex,
} from './shaders';
import { PROFILES, type QualityProfile } from './quality';
import {
  BOULDER_M,
  CELL_KM,
  GRAIN_CELL_KM,
  GRAIN_ROWS,
  GRAIN_WINDOW,
  GRAINS_PER_CELL,
  type LatticeWindow,
  ROCKS_PER_CELL,
  ROWS,
  type RingRocks,
  WINDOW,
} from '../ringRocks';
import {
  RING_GRAIN_SLOTS,
  RING_ROCK_SLOTS,
  ringGrainFragment,
  ringGrainVertex,
  ringRockFragment,
  ringRockVertex,
} from './ringRockShaders';
import { dustRingFragment, dustRingVertex, ringFragment, ringVertex, skyRingFragment, skyRingVertex } from './ringShaders';
import type { DustRing } from '../api/types/DustRing';
import type { RingSystem } from '../api/types/RingSystem';
import { type GiantStorm, MAX_STORMS } from '../giantStorms';
import type { GiantInfo } from '../api/types/GiantInfo';
import { starField } from './stars';

export type ViewMode = 'globe' | 'map';

/** Where the camera is, to show another view from the same place. */
export interface CameraState {
  mode: ViewMode;
  position: [number, number, number];
  target: [number, number, number];
  up: [number, number, number];
}

/** Where the light comes from. */
export type Lighting =
  | { kind: 'headlight' }
  | {
      kind: 'sun';
      /** Fraction of the year: 0 = northern spring equinox, 0.25 = northern summer solstice. */
      season: number;
      /** Longitude under the sun, in degrees. */
      longitudeDeg: number;
      axialTiltDeg: number;
      /** Latitude under the sun, degrees, when given (on a moon); else from the season and the tilt. */
      latitudeDeg?: number;
    };

/** Layers drawn from uExtra, uploaded the first time one of them is shown. */
const EXTRA_LAYERS: ReadonlySet<Layer> = new Set(['volcanic', 'coast', 'floodplain']);

const MORPH_MS = 1200;
/** How far the spin axes reach out of a body, in its radii (see setSpinAxes). */
const SPIN_AXIS_LENGTH = 1.5;
/**
 * Draw order of a sky body in front of the body shown: after the globe,
 * its atmosphere, rings, grid and axes, before the near orbit lines (10).
 */
const SKY_FRONT_ORDER = 5;
/**
 * Half-size, in CSS pixels, below which a body of the sky is drawn no
 * smaller (with a faint ring), and to which a gap in a giant's rings is
 * widened on screen, so that a moonlet shows inside its gap.
 */
const SKY_BODY_MIN_PX = 2.5;
/** Vertices across a detail patch (see TileLayer); the base surface's are the profile's (surfaceSegments). */
const PATCH_SEGMENTS = 16;
/** The arrow of the sense of rotation round it: its height, radius and arc (radians). */
/** Colours of their north and south ends, linear. */
const SPIN_AXIS_NORTH: [number, number, number] = [0.9, 0.9, 0.85];
const SPIN_AXIS_SOUTH: [number, number, number] = [0.3, 0.45, 0.85];
/** Pointer moves larger than this (px) are drags, not clicks. */
const CLICK_TOLERANCE = 5;
/** Altitudes (planet radii, or map units) over which clouds fade in when zooming out. */
const CLOUD_FADE: [number, number] = [0.15, 0.5];
/** Seconds for the drifting cloud layer to go once around the planet. */
const CLOUD_DRIFT_PERIOD_S = 300;
/**
 * Redraw interval while only the clouds drift, ms. The drift is slow (a few
 * pixels a second at the equator), so ten frames a second look smooth and
 * cost a tenth of a continuous loop.
 */
const CLOUD_DRIFT_FRAME_MS = 100;

function smoothstepJs(edge0: number, edge1: number, x: number): number {
  const t = Math.min(Math.max((x - edge0) / (edge1 - edge0), 0), 1);
  return t * t * (3 - 2 * t);
}

/** Headlight direction in view space: above and to the left of the camera. */
const HEADLIGHT = new Vector3(-0.4, 0.5, 1).normalize();
/** Lowest camera altitude over the globe (radius 1): about 2 km on an Earth-sized planet. */
const MIN_ALTITUDE = 0.0003;
/** Farthest camera distance from the globe's centre, and the map camera's height range. */
const GLOBE_MAX_DISTANCE = 8;
const MAP_MIN_DISTANCE = 0.002;
const MAP_MAX_DISTANCE = 12;
/** Latitude (radians) the camera stops short of the poles at when switching views. */
const POLE_LIMIT = (89.5 * Math.PI) / 180;

/** Graticule every `step` degrees, as line segments in uv space. */
function graticule(step: number): BufferGeometry {
  const uvs: number[] = [];
  const samples = 180;
  for (let lon = 0; lon < 360; lon += step) {
    for (let i = 0; i < samples; i += 1) {
      uvs.push(lon / 360, i / samples, lon / 360, (i + 1) / samples);
    }
  }
  for (let lat = -90 + step; lat < 90; lat += step) {
    const v = (lat + 90) / 180;
    for (let i = 0; i < samples * 2; i += 1) {
      uvs.push(i / (samples * 2), v, (i + 1) / (samples * 2), v);
    }
  }
  const geometry = new BufferGeometry();
  // Positions are computed in the shader; three.js still needs the attribute.
  geometry.setAttribute('position', new Float32BufferAttribute(new Array((uvs.length / 2) * 3).fill(0), 3));
  geometry.setAttribute('uv', new Float32BufferAttribute(uvs, 2));
  return geometry;
}

/**
 * A path of great-circle legs as line segments in uv space, with a small
 * circle around each point. Segments crossing the date line are cut in two
 * at the edge of the map, so the flat map shows no line across it.
 */
function pathGeometry(points: readonly LonLat[]): BufferGeometry {
  const uvs: number[] = [];
  const push = (a: LonLat, b: LonLat) => {
    const [u0, v0] = lonLatToUv(a);
    const [u1, v1] = lonLatToUv(b);
    if (Math.abs(u1 - u0) < 0.5) {
      uvs.push(u0, v0, u1, v1);
      return;
    }
    // Crossing eastwards (u wraps from 1 to 0) or westwards.
    const east = u1 < u0;
    const edge = east ? 1 : 0;
    const unwrapped = east ? u1 + 1 : u1 - 1;
    const t = (edge - u0) / (unwrapped - u0);
    const v = v0 + t * (v1 - v0);
    uvs.push(u0, v0, edge, v, 1 - edge, v, u1, v1);
  };
  for (let i = 1; i < points.length; i += 1) {
    const a = points[i - 1]!;
    const b = points[i]!;
    const arc = greatCircleArc(a, b, 64);
    for (let k = 1; k < arc.length; k += 1) {
      push(arc[k - 1]!, arc[k]!);
    }
  }
  const radius = (0.6 * Math.PI) / 180;
  for (const p of points) {
    const stretch = 1 / Math.max(0.1, Math.cos(p.lat));
    let previous: LonLat | null = null;
    for (let k = 0; k <= 24; k += 1) {
      const angle = (k / 24) * 2 * Math.PI;
      const q = { lon: p.lon + Math.cos(angle) * radius * stretch, lat: p.lat + Math.sin(angle) * radius };
      if (previous) {
        push(previous, q);
      }
      previous = q;
    }
  }
  const geometry = new BufferGeometry();
  geometry.setAttribute('position', new Float32BufferAttribute(new Array((uvs.length / 2) * 3).fill(0), 3));
  geometry.setAttribute('uv', new Float32BufferAttribute(uvs, 2));
  return geometry;
}

/** The optical depth table of the sky shader, for a shell and its scale heights (see sunDepth.ts). */
function sunDepthTexture(top: number, heightR: number, heightM: number): DataTexture {
  const texture = new DataTexture(
    sunDepthTable(top, heightR, heightM),
    SUN_DEPTH_WIDTH,
    SUN_DEPTH_HEIGHT,
    RGFormat,
    HalfFloatType,
  );
  texture.magFilter = LinearFilter;
  texture.minFilter = LinearFilter;
  texture.wrapS = ClampToEdgeWrapping;
  texture.wrapT = ClampToEdgeWrapping;
  texture.generateMipmaps = false;
  texture.needsUpdate = true;
  return texture;
}

/** The last part of a URL's path ("world.png"), for progress labels. */
function fileName(url: string): string {
  const path = url.split(/[?#]/)[0] ?? url;
  return path.slice(path.lastIndexOf('/') + 1) || path;
}

function emptyTexture(): DataTexture {
  const texture = new DataTexture(new Uint8Array(4), 1, 1, RGBAFormat, UnsignedByteType);
  texture.needsUpdate = true;
  return texture;
}

/**
 * An RGBA half-float data layer, filtered, wrapping east-west. `owned`: the
 * array is a copy made for the GPU, dropped once uploaded. `mipmaps`: with
 * a mipmap chain (the GPU must render to half floats to build it), so that
 * a large map seen whole is averaged instead of sampled every few texels,
 * which made its contours and colours shimmer and stair-step.
 */
function halfFloatTexture(data: Uint16Array, width: number, height: number, owned: boolean, mipmaps = false): DataTexture {
  const texture = new DataTexture(data, width, height, RGBAFormat, HalfFloatType);
  texture.magFilter = LinearFilter;
  texture.minFilter = mipmaps ? LinearMipmapLinearFilter : LinearFilter;
  texture.wrapS = RepeatWrapping;
  texture.wrapT = ClampToEdgeWrapping;
  texture.generateMipmaps = mipmaps;
  texture.needsUpdate = true;
  releaseOnUpload(texture, owned);
  return texture;
}

/**
 * The cloud cover as a single-channel texture (one byte per pixel, row 0 =
 * north), filtered with mipmaps, wrapping east-west.
 */
function cloudTexture(image: Grey8): DataTexture {
  const texture = new DataTexture(image.data, image.width, image.height, RedFormat, UnsignedByteType);
  // Rows of an odd width are not 4-byte aligned.
  texture.unpackAlignment = 1;
  texture.wrapS = RepeatWrapping;
  texture.wrapT = ClampToEdgeWrapping;
  texture.magFilter = LinearFilter;
  texture.minFilter = LinearMipmapLinearFilter;
  texture.generateMipmaps = true;
  texture.needsUpdate = true;
  releaseOnUpload(texture);
  return texture;
}

/** Hex biome colours (rgb) and river flags (a) as a columns × rows texture. */
function hexTexture(map: HexMap): DataTexture {
  const colours = map.biomes.map((b) => {
    const c = Number.parseInt(b.color.slice(1), 16);
    return [(c >> 16) & 255, (c >> 8) & 255, c & 255] as const;
  });
  const data = new Uint8Array(map.columns * map.rows * 4);
  map.cells.biome.forEach((id, i) => {
    const [r, g, b] = colours[id] ?? [0, 0, 0];
    data.set([r, g, b, map.cells.river[i] ? 255 : 0], i * 4);
  });
  const texture = new DataTexture(data, map.columns, map.rows, RGBAFormat, UnsignedByteType);
  texture.magFilter = NearestFilter;
  texture.minFilter = NearestFilter;
  texture.needsUpdate = true;
  return texture;
}

/**
 * Detail tile textures each view keeps while several views are open (two
 * compared worlds): half the budget of a single view, so comparing does
 * not double the GPU memory of the tiles.
 */
const COMPARE_TILE_BUDGET = MAX_TEXTURES / 2;

/** Views alive, which share the tile budget. */
const liveViews = new Set<PlanetView>();

function balanceTileBudgets(): void {
  const budget = liveViews.size > 1 ? COMPARE_TILE_BUDGET : MAX_TEXTURES;
  for (const view of liveViews) {
    // Its profile's share: without a GPU the textures live in the
    // machine's memory, and each one is mipmapped on the CPU.
    view.setTileBudget(budget * view.quality.tileBudget);
  }
}

/** The host star as the view draws it (see `setHostStar`). */
export interface HostStar {
  /** Linear RGB of its light, the Sun white. */
  colour: [number, number, number];
  /** Apparent diameter from the planet, degrees. */
  angularDiameterDeg: number;
  /** Light the planet gets, relative to Earth's. */
  insolation: number;
  /** Effective temperature, K. */
  teffK: number;
}

/**
 * How far vegetation is recoloured under a star (see the surface shader):
 * towards dark purple-black below the Sun's temperature (fully at 3 300 K),
 * towards gold above it (fully at 7 300 K); none for a Sun-like star.
 */
export function vegetationShift(teffK: number): [number, number] {
  const clamp = (x: number) => Math.min(1, Math.max(0, x));
  return [clamp((5772 - teffK) / 2500), clamp((teffK - 5772) / 1500)];
}

/** The Sun from Earth: worlds made before stars. */
export const SUN_STAR: HostStar = { colour: [1, 1, 1], angularDiameterDeg: 0.533, insolation: 1, teffK: 5772 };

/**
 * Temperatures of a giant's cloud tops on the temperature layer, °C: a
 * cold giant's within 25 K of its effective temperature, a hot one's from
 * its night to its day side.
 */
export function giantTemperatureRange(info: Pick<GiantInfo, 'effective_k' | 'hot'>): [number, number] {
  const t = info.effective_k - 273.15;
  return info.hot ? [info.effective_k * 0.65 - 273.15, info.effective_k * 1.35 - 273.15] : [t - 25, t + 25];
}

export class PlanetView {
  private readonly renderer: WebGLRenderer;
  private readonly scene = new Scene();
  private readonly camera = new PerspectiveCamera(40, 1, 0.01, 100);
  private readonly controls: OrbitControls;
  private readonly surface: Mesh<PlaneGeometry, ShaderMaterial>;
  private readonly grid: LineSegments<BufferGeometry, ShaderMaterial>;
  /** The star field (see stars.ts). */
  private readonly stars: Points<BufferGeometry, ShaderMaterial>;
  /** The host star, drawn with Sunlight at the sun's direction. */
  private readonly hostStar: Points<BufferGeometry, ShaderMaterial>;
  /** The atmosphere's ground (veil) and sky passes. */
  private readonly atmosphere: Mesh<SphereGeometry, ShaderMaterial>[];
  private readonly resizeObserver: ResizeObserver;
  private readonly tiles: TileLayer;
  private layer: Layer = 'biomes';
  private hexOn = false;
  /** Uniforms shared by the surface and the graticule (the same objects). */
  private readonly shared = {
    uMorph: { value: 1 } as IUniform<number>,
    uRelief: { value: 0 } as IUniform<number>,
    uReliefScale: { value: 0 } as IUniform<number>,
    uCoast: { value: 0 } as IUniform<number>,
    uLiquidSea: { value: 1 } as IUniform<number>,
    uHasData: { value: 0 } as IUniform<number>,
    uData: { value: emptyTexture() } as IUniform<Texture>,
    uHasShape: { value: 0 } as IUniform<number>,
    uShape: { value: emptyTexture() } as IUniform<Texture>,
    uShapeTexel: { value: new Vector2(1 / 256, 1 / 128) } as IUniform<Vector2>,
    uFlattening: { value: 0 } as IUniform<number>,
  };
  /** Counts the shape requests: a load answering after a newer one is dropped. */
  private shapeRequest = 0;
  /** Surface-only uniforms. */
  private readonly u = {
    uLift: { value: 0 } as IUniform<number>,
    uMap: { value: emptyTexture() } as IUniform<Texture>,
    uCloudsOn: { value: 0 } as IUniform<number>,
    // 1 draws the clouds' shadows on the ground (the profile's cloudShadows).
    uCloudShadows: { value: 1 } as IUniform<number>,
    uClouds: { value: emptyTexture() } as IUniform<Texture>,
    uCloudTint: { value: new Vector3(1, 1, 1) } as IUniform<Vector3>,
    uLightTint: { value: new Vector3(1, 1, 1) } as IUniform<Vector3>,
    uCloudShift: { value: 0 } as IUniform<number>,
    uGiant: { value: 0 } as IUniform<number>,
    uTempRange: { value: new Vector2(-30, 70) } as IUniform<Vector2>,
    uWind: { value: new Array<number>(91).fill(0) } as IUniform<number[]>,
    uBandTime: { value: 0 } as IUniform<number>,
    // A giant's storms, each drifting whole (see setGiantStorms).
    uStorms: { value: Array.from({ length: MAX_STORMS }, () => new Vector4()) } as IUniform<Vector4[]>,
    uStormCount: { value: 0 } as IUniform<number>,
    uHotGlow: { value: 0 } as IUniform<number>,
    uRingInner: { value: 0 } as IUniform<number>,
    uRingOuter: { value: 0 } as IUniform<number>,
    uRingOpacity: { value: 0 } as IUniform<number>,
    uRingGaps: { value: Array.from({ length: 4 }, () => new Vector2()) } as IUniform<Vector2[]>,
    uRingGapCount: { value: 0 } as IUniform<number>,
    uLayer: { value: LAYER_IDS.biomes as number } as IUniform<number>,
    uTexel: { value: new Vector2(1 / 1024, 1 / 512) } as IUniform<Vector2>,
    uLightMode: { value: 0 } as IUniform<number>,
    uLightDir: { value: new Vector3(0, 0, 1) } as IUniform<Vector3>,
    uSunDir: { value: new Vector3(0, 0, 1) } as IUniform<Vector3>,
    uHexOn: { value: 0 } as IUniform<number>,
    uHex: { value: emptyTexture() } as IUniform<Texture>,
    uHexGrid: { value: [1, 1, 1, 1] } as IUniform<number[]>,
    uMapSize: { value: new Vector2(1, 1) } as IUniform<Vector2>,
    // The base surface covers the whole planet and draws no detail tile.
    uTileRect: { value: new Vector4(0, 0, 1, 1) } as IUniform<Vector4>,
    // Vertices across the mesh drawn: the base surface's (see surfaceVertex,
    // the relief's level of detail), set from the profile when the view is made.
    uMeshSegments: { value: 720 } as IUniform<number>,
    uTileOn: { value: 0 } as IUniform<number>,
    uTile: { value: emptyTexture() } as IUniform<Texture>,
    uTexRect: { value: new Vector4(0, 0, 1, 1) } as IUniform<Vector4>,
    uHasExtra: { value: 0 } as IUniform<number>,
    uExtra: { value: emptyTexture() } as IUniform<Texture>,
    uCoastMax: { value: 2000 } as IUniform<number>,
    uHasBiomes: { value: 0 } as IUniform<number>,
    uVegetation: { value: new Array<number>(64).fill(0) } as IUniform<number[]>,
    uVegShift: { value: new Vector2(0, 0) } as IUniform<Vector2>,
    uBiomes: { value: emptyTexture() } as IUniform<Texture>,
    uHighlight: { value: -1 } as IUniform<number>,
    uSeaContours: { value: 0 } as IUniform<number>,
    uLandContours: { value: 0 } as IUniform<number>,
    uTropics: { value: 0 } as IUniform<number>,
    uTilt: { value: 23.5 } as IUniform<number>,
    uCasters: { value: Array.from({ length: MAX_CASTERS }, () => new Vector4()) } as IUniform<Vector4[]>,
    uCasterCount: { value: 0 } as IUniform<number>,
    uSunRadius: { value: (0.533 / 2) * (Math.PI / 180) } as IUniform<number>,
  };
  private readonly path: LineSegments<BufferGeometry, ShaderMaterial>;
  /** Axial tilt the reference lines are drawn for, or null for none. */
  private referenceTilt: number | null = null;
  private referencesOn = false;
  private atmosphereOpacity = { value: 1 } as IUniform<number>;
  private texture: Texture | null = null;
  /**
   * The world whose layers of format version 2 are not uploaded yet (with
   * the factor that fits them to the GPU), until a layer needs them.
   */
  private pendingExtra: { packed: Uint16Array; width: number; height: number; factor: number } | null = null;
  /**
   * Rendering on demand: a frame is requested when something changed (a
   * setter, the camera, a loaded tile, a resize), and frames keep coming
   * only while the camera's damping or the morph animation runs.
   */
  private frame: number | null = null;
  private dirty = true;
  private readonly lastPosition = new Vector3();
  private readonly lastQuaternion = new Quaternion();
  /** When the camera or the morph last moved (see pixelRatio). */
  private lastMotion = -Infinity;
  /** Counts setTexture calls: only the latest one may set the map. */
  private textureRequest = 0;
  private disposed = false;
  /** When the last frame was drawn (requestAnimationFrame's clock), for the frame cap. */
  private lastDrawn = -Infinity;
  private atmosphereOn = true;
  /** Whether the star field is shown. */
  private starsOn = true;
  /** The other bodies drawn in the sky (see setSkyBodies), with their maps. */
  private skyBodies: {
    mesh: Mesh<PlaneGeometry, ShaderMaterial>;
    body: SkyBody;
    maps: Texture[];
    /** Its rings, drawn over the sky (see skyRingFragment), once it has some. */
    rings: Mesh<BufferGeometry, ShaderMaterial> | null;
  }[] = [];
  /** The orbits of the bodies in the sky (see setSkyOrbits), null until some are given. */
  private skyOrbits: LineSegments<BufferGeometry, ShaderMaterial> | null = null;
  /**
   * The globe's spin axis (see setSpinAxes): out of its poles, the north
   * end light, the south end blue.
   */
  private spinAxis: LineSegments<BufferGeometry, LineBasicMaterial> | null = null;
  /** The spin arrow (or a locked body's ring) and the host planet's line and arrows (see lib/viewer/bodyGuides). */
  private readonly guides = new BodyGuides(new Color(...SPIN_AXIS_NORTH));
  /**
   * The host planet's spin, in a moon's sky (see setHostSpin): its arrow
   * or locked ring in 3D as the body shown's, round the planet far away,
   * its depth held at the far plane and hidden where the planet stands in
   * front of it.
   */
  private readonly hostSpinUniforms = { uHostCentre: { value: new Vector3() }, uHostRadius: { value: 1 } };
  private readonly hostSpin = new BodyGuides(new Color(...SPIN_AXIS_NORTH), (material) => farGuide(material, this.hostSpinUniforms));
  /** Which body of the sky is the host planet, and whether it is locked to its star; null for none. */
  private hostSpinOf: { index: number; locked: boolean } | null = null;
  private hostGuideOn = false;
  private spinAxesOn = false;
  /** Their near parts, drawn after the globe (see skyOrbitFragment). */
  private skyOrbitsNear: LineSegments<BufferGeometry, ShaderMaterial> | null = null;
  private skyRequest = 0;
  /** Half-size of a small sky body's marker this frame, radians (see drawSkyBodies). */
  private skySmallest = 0;
  /** The host star (see setHostStar); the Sun until a world sets it. */
  private host: HostStar = SUN_STAR;
  /**
   * The host star's centre and radius in radii of the body shown, in its
   * frame (see setStarPlace); null to draw it at infinity, at the sun's
   * direction.
   */
  private starPlace: { position: Vector3; radius: number } | null = null;
  /** How far out the camera may go on the globe, radii from the centre (see setGlobeReach). */
  private globeReach = GLOBE_MAX_DISTANCE;
  /** The last turn of the stars (see setStarsTurn), not wrapped. */
  private starsTurned = 0;
  /** Whether the planet is lit with its star's colour. */
  private starLightOn = false;
  /** Whether the world has an atmosphere to draw. */
  private hasSky = true;
  private cloudRequest = 0;
  /** The cloud layer of the world, and its download once started. */
  private cloudSource: {
    url: string;
    tint: [number, number, number];
    signal: AbortSignal | undefined;
    request: number;
    loading: Promise<void> | null;
  } | null = null;
  /**
   * Whether the cloud layer is uploaded, and whether it is shown (hidden
   * until told otherwise, so a world shown without clouds never loads it).
   */
  private hasClouds = false;
  private cloudsOn = false;
  /** Whether the cloud layer drifts, the last drift update, and its redraw timer. */
  private cloudsDrifting = false;
  private cloudDriftAt: number | null = null;
  /** The rings of the giant shown, if any. */
  private rings: Mesh<RingGeometry, ShaderMaterial> | null = null;
  /** The faint dust rings its moons feed (see setDustRings). */
  private dustRings: Mesh<RingGeometry, ShaderMaterial>[] = [];
  /**
   * The body's shape as setShape loaded it, for the guides drawn out of its
   * surface: its radii (row 0 north), and the most it reaches within 15°
   * of either pole; null for a sphere.
   */
  private shapeRadii: { radii: Float32Array; width: number; height: number; poles: number } | null = null;
  /** Its highest point over the coastline, in elevation units (see setData). */
  private peakAbove = 0;
  /** The rocks of the giant's rings round a ring moon (see setRingRocks), null until some are. */
  private ringRocks: Mesh<InstancedBufferGeometry, ShaderMaterial> | null = null;
  /** The grain of the rings beyond the rocks (see setRingRocks). */
  private ringGrains: Points<BufferGeometry, ShaderMaterial> | null = null;
  private ringRocksOn = false;
  private cloudDriftTimer: ReturnType<typeof setTimeout> | null = null;
  /** The world's sky before the lift over the relief, Earth's until a world sets it. */
  private worldSky: Sky = {
    top: 1.04,
    betaR: [4.7, 11.4, 27.8],
    betaM: 4.6,
    mieAlbedo: [1, 1, 1],
    heightR: 0.009,
    heightM: 0.0043,
    g: 0.76,
  };
  /** Scattering uniforms of the sky shader (see applySky). */
  private readonly sky = {
    uTop: { value: 1.04 } as IUniform<number>,
    uBetaR: { value: new Vector3(4.7, 11.4, 27.8) } as IUniform<Vector3>,
    uBetaM: { value: 4.6 } as IUniform<number>,
    uMieAlbedo: { value: new Vector3(1, 1, 1) } as IUniform<Vector3>,
    uHeightR: { value: 0.009 } as IUniform<number>,
    uHeightM: { value: 0.0043 } as IUniform<number>,
    uG: { value: 0.76 } as IUniform<number>,
    uSunDepth: { value: sunDepthTexture(1.04, 0.009, 0.0043) } as IUniform<DataTexture>,
  };
  private mode: ViewMode = 'globe';
  private morph = 1;
  /**
   * The morph between globe and map under way. While `follow` holds, the
   * camera follows the point at the centre of the screen as it moves from
   * `centreFrom` to `centreTo`, from `altitude` above the globe to
   * `mapHeight` above the map (see viewTransfer.ts).
   */
  private animation: {
    from: number;
    to: number;
    start: number;
    follow: boolean;
    centreFrom: LonLat;
    centreTo: LonLat;
    altitude: number;
    mapHeight: number;
  } | null = null;
  /** Distance from the camera to the point it follows during the morph. */
  private morphDistance = 0;
  private pointerDown: Vector2 | null = null;
  /** Last pointer position over the canvas, hovered on the next frame (when `hovering`). */
  private readonly hoverAt = new Vector2();
  private hovering = false;
  private hoverPending = false;
  /** Scratch objects of picking, reused by every hover. */
  private readonly raycaster = new Raycaster();
  private readonly ndc = new Vector2();
  private readonly ray: { origin: [number, number, number]; direction: [number, number, number] } = {
    origin: [0, 0, 0],
    direction: [0, 0, 0],
  };
  private readonly turn = new Quaternion();
  /** Whether the user is steering this view (pointer over it): only then it reports its camera. */
  private steering = false;
  private applyingCamera = false;

  /** Called with the picked point, or null when a click misses the planet. */
  onPick: (point: LonLat | null) => void = () => {};
  /** Called with the point under the pointer (at most once a frame), or null off the planet. */
  onHover: (point: LonLat | null) => void = () => {};
  /** Called when the user moves the camera of this view. */
  onCameraChange: (state: CameraState) => void = () => {};
  /** Called when the canvas gets a size or loses it (see `visible`). */
  onVisibilityChange: (visible: boolean) => void = () => {};
  /** Called after each frame with the world's axes as the camera sees them (the axes gizmo). */
  onAxes: (axes: ScreenAxis[]) => void = () => {};
  /**
   * Whether the canvas has a size. A canvas without one (in an element with
   * `display: none`, such as the viewer of a narrow screen showing the world
   * list) draws nothing, and the cloud drift stops, until it shows again.
   */
  private shown = false;

  get visible(): boolean {
    return this.shown;
  }

  /**
   * `quality` is what the view draws (see lib/viewer/quality.ts), kept for
   * its life: switching it reloads the page.
   */
  constructor(
    private readonly canvas: HTMLCanvasElement,
    readonly quality: QualityProfile = PROFILES.high,
  ) {
    // The stencil lets detail patches hide the base surface (see TileLayer).
    this.renderer = new WebGLRenderer({ canvas, antialias: quality.antialias, stencil: true });
    this.renderer.setPixelRatio(fullPixelRatio(window.devicePixelRatio, quality));
    this.scene.background = new Color('#05070d');

    // The stars, behind everything (see starsVertex).
    const field = starField();
    const starGeometry = new BufferGeometry();
    starGeometry.setAttribute('position', new BufferAttribute(field.directions, 3));
    starGeometry.setAttribute('colour', new BufferAttribute(field.colours, 3));
    starGeometry.setAttribute('size', new BufferAttribute(field.sizes, 1));
    this.stars = new Points(
      starGeometry,
      new ShaderMaterial({
        vertexShader: starsVertex,
        fragmentShader: starsFragment,
        uniforms: { uPixelRatio: { value: 1 }, uOpacity: { value: 1 } },
        blending: AdditiveBlending,
        transparent: true,
        depthWrite: false,
      }),
    );
    // Directions, not positions: the geometry bounds are meaningless.
    this.stars.frustumCulled = false;
    // The sky, farthest first: the star field, the host star, then the
    // bodies of the system, which hide both (all at infinity, drawn in
    // this order after the surface, which hides them by depth).
    this.stars.renderOrder = -3;
    this.scene.add(this.stars);

    // The host star, at the sun's direction (see hostStarVertex).
    const hostGeometry = new BufferGeometry();
    hostGeometry.setAttribute('position', new BufferAttribute(new Float32Array([0, 0, 1]), 3));
    this.hostStar = new Points(
      hostGeometry,
      new ShaderMaterial({
        vertexShader: hostStarVertex,
        fragmentShader: hostStarFragment,
        uniforms: {
          uColor: { value: new Vector3(1, 1, 1) },
          uSize: { value: 64 },
          uDisk: { value: 8 },
          uGlow: { value: 0.5 },
          uOpacity: { value: 1 },
        },
        blending: AdditiveBlending,
        transparent: true,
        depthWrite: false,
      }),
    );
    this.hostStar.frustumCulled = false;
    this.hostStar.renderOrder = -2;
    this.hostStar.visible = false;
    this.scene.add(this.hostStar);

    // 2 vertices per degree in the high profile: enough for the relief to
    // follow the data. On a CPU renderer the vertices cost more than the
    // pixels: the light profile's coarser globe (240) halves the frame.
    this.u.uMeshSegments.value = quality.surfaceSegments;
    this.u.uCloudShadows.value = quality.cloudShadows ? 1 : 0;
    this.surface = new Mesh(
      new PlaneGeometry(1, 1, quality.surfaceSegments, quality.surfaceSegments / 2),
      new ShaderMaterial({
        vertexShader: surfaceVertex,
        fragmentShader: surfaceFragment,
        uniforms: { ...this.shared, ...this.u },
        side: DoubleSide,
      }),
    );
    // Positions come from the shader: the geometry bounds are meaningless.
    this.surface.frustumCulled = false;
    // Drawn only where no detail patch was drawn this frame.
    this.surface.material.stencilWrite = true;
    this.surface.material.stencilRef = DETAIL_STENCIL;
    this.surface.material.stencilFunc = NotEqualStencilFunc;
    this.scene.add(this.surface);
    this.scene.add(this.guides.spin, this.guides.host, this.hostSpin.spin);

    this.grid = new LineSegments(
      graticule(30),
      new ShaderMaterial({
        vertexShader: surfaceVertex,
        fragmentShader: lineFragment,
        uniforms: {
          ...this.shared,
          uLift: { value: 0.003 },
          uTileRect: { value: new Vector4(0, 0, 1, 1) },
          uColor: { value: new Color('#ffffff') },
          uOpacity: { value: 0.25 },
        },
        transparent: true,
        depthWrite: false,
      }),
    );
    this.grid.frustumCulled = false;
    this.grid.visible = false;
    // Overlays draw over the sky, which veils the surface only.
    this.grid.renderOrder = 2;
    this.scene.add(this.grid);


    // A unit sphere scaled to the top of the world's atmosphere, drawn after
    // the surface from its far side, twice (see atmosphereFragment): where
    // the surface is in front of it (the depth test passes on greater
    // depths) to veil the ground, and elsewhere to draw the sky. Relief
    // rising above the sphere at the limb thus hides the sky behind it.
    const shell = new SphereGeometry(1, 96, 48);
    const skyPass = (ground: boolean): Mesh<SphereGeometry, ShaderMaterial> => {
      const mesh = new Mesh(
        shell,
        new ShaderMaterial({
          vertexShader: atmosphereVertex,
          // The light profile's halo has no march (see atmosphereHaloFragment).
          fragmentShader: quality.atmosphere === 'halo' ? atmosphereHaloFragment : atmosphereFragment,
          uniforms: {
            uOpacity: this.atmosphereOpacity,
            uLightMode: this.u.uLightMode,
            uSunDir: this.u.uSunDir,
            uLightDir: this.u.uLightDir,
            uLightTint: this.u.uLightTint,
            ...this.sky,
            uGround: { value: ground ? 1 : 0 },
          },
          side: BackSide,
          blending: CustomBlending,
          blendEquation: AddEquation,
          blendSrc: OneFactor,
          blendDst: SrcAlphaFactor,
          transparent: true,
          depthFunc: ground ? GreaterDepth : LessEqualDepth,
          depthWrite: false,
        }),
      );
      mesh.renderOrder = 1;
      mesh.scale.setScalar(this.worldSky.top);
      this.scene.add(mesh);
      return mesh;
    };
    this.atmosphere = [skyPass(true), skyPass(false)];

    this.path = new LineSegments(
      new BufferGeometry(),
      new ShaderMaterial({
        vertexShader: surfaceVertex,
        fragmentShader: lineFragment,
        uniforms: {
          ...this.shared,
          uLift: { value: 0.004 },
          uTileRect: { value: new Vector4(0, 0, 1, 1) },
          uColor: { value: new Color('#ffd166') },
          uOpacity: { value: 1 },
        },
        transparent: true,
        depthWrite: false,
      }),
    );
    this.path.frustumCulled = false;
    this.path.visible = false;
    this.path.renderOrder = 2;
    this.scene.add(this.path);

    this.tiles = new TileLayer(
      this.scene,
      () =>
        new ShaderMaterial({
          vertexShader: surfaceVertex,
          fragmentShader: surfaceFragment,
          // Shared uniform objects, except the patch's own tile and rectangle.
          uniforms: {
            ...this.shared,
            ...this.u,
            uTileRect: { value: new Vector4(0, 0, 1, 1) },
            uMeshSegments: { value: PATCH_SEGMENTS },
            uTileOn: { value: 0 },
            uTile: { value: null },
            uTexRect: { value: new Vector4(0, 0, 1, 1) },
          },
          // Front faces only: with DoubleSide, the far side of a patch
          // would mark the stencil and punch a hole in the near globe.
          side: FrontSide,
        }),
      () => this.requestRender(),
    );

    this.controls = new OrbitControls(this.camera, canvas);
    // Without inertia in the light profile: it keeps drawing frames after
    // the pointer stops.
    this.controls.enableDamping = quality.damping;
    this.applyModeControls();
    this.camera.position.set(0, 0, GLOBE_DISTANCE);

    canvas.addEventListener('wheel', this.handleWheel, { passive: false });
    canvas.addEventListener('pointerdown', this.handlePointerDown);
    canvas.addEventListener('pointerup', this.handlePointerUp);
    canvas.addEventListener('pointermove', this.handlePointerMove);
    canvas.addEventListener('pointerenter', this.handlePointerEnter);
    canvas.addEventListener('pointerleave', this.handlePointerLeave);
    this.controls.addEventListener('change', this.handleControlsChange);
    this.resizeObserver = new ResizeObserver(() => this.resize());
    this.resizeObserver.observe(canvas);
    this.resize();
    this.requestRender();
    liveViews.add(this);
    balanceTileBudgets();
  }

  /**
   * Sets how many detail tile textures this view keeps on the GPU. Views
   * set it themselves: MAX_TEXTURES alone, half as many each while two are
   * open, times their profile's tileBudget.
   */
  setTileBudget(count: number): void {
    this.tiles.setTextureBudget(count);
    this.requestRender();
  }

  /** Draws the scene on the next frame. */
  private requestRender(): void {
    this.dirty = true;
    this.schedule();
  }

  /** Runs a frame soon (to render if dirty, or to process the hover). */
  private schedule(): void {
    if (this.frame === null && !this.disposed) {
      this.frame = requestAnimationFrame(this.render);
    }
  }

  /**
   * Shows a cloud cover layer (8-bit `clouds.png`) tinted by `tint` (linear
   * RGB), or none with a null url. The layer is downloaded and decoded only
   * while clouds are shown (see setCloudsVisible): the promise resolves at
   * once while they are hidden. Aborting `signal` drops the load, also one
   * started later by setCloudsVisible. Like the base map, only the latest
   * call takes effect.
   */
  setClouds(url: string | null, tint: [number, number, number], signal?: AbortSignal): Promise<void> {
    this.cloudRequest += 1;
    // The previous world's clouds would not match the new map.
    this.disposeClouds();
    this.hasClouds = false;
    this.requestRender();
    this.cloudSource = url ? { url, tint, signal, request: this.cloudRequest, loading: null } : null;
    return this.cloudsOn ? this.loadClouds() : Promise.resolve();
  }

  /**
   * Downloads and uploads the cloud layer of the current source, once:
   * every caller gets the same promise. A failed load can be tried again.
   */
  private loadClouds(): Promise<void> {
    const source = this.cloudSource;
    if (!source) {
      return Promise.resolve();
    }
    source.loading ??= (async () => {
      try {
        const image = await loadGrey8(source.url, this.renderer.capabilities.maxTextureSize, source.signal);
        if (this.disposed || source.request !== this.cloudRequest) {
          return;
        }
        this.disposeClouds();
        this.u.uClouds.value = cloudTexture(image);
        this.u.uCloudTint.value.set(...source.tint);
        this.hasClouds = true;
        this.requestRender();
      } catch (error) {
        source.loading = null;
        throw error;
      }
    })();
    return source.loading;
  }

  /**
   * Gives the globe the shape of an irregular body (a small moon): its
   * 16-bit `shape.png` at `url`, the radius as a share of the mean one
   * between `min` and `max`; null for a round body.
   */
  async setShape(shape: { url: string; min: number; max: number } | null, signal?: AbortSignal): Promise<void> {
    this.shapeRequest += 1;
    const request = this.shapeRequest;
    this.shared.uHasShape.value = 0;
    this.shared.uShape.value.dispose();
    this.shared.uShape.value = emptyTexture();
    this.shapeRadii = null;
    this.requestRender();
    if (!shape) {
      return;
    }
    const response = await fetch(shape.url, signal ? { signal } : {});
    if (!response.ok) {
      throw new Error(`${response.status}`);
    }
    const image = decodePng(await response.arrayBuffer());
    if (this.disposed || request !== this.shapeRequest) {
      return;
    }
    const scale = (shape.max - shape.min) / 65535;
    const values = new Uint16Array(image.width * image.height);
    const radii = new Float32Array(values.length);
    for (let i = 0; i < values.length; i++) {
      radii[i] = shape.min + (image.data[i] ?? 0) * scale;
      values[i] = DataUtils.toHalfFloat(radii[i]!);
    }
    // The caps the spin axis and its arrow stand over.
    const capRows = Math.max(1, Math.round((image.height * 15) / 180));
    let poles = 0;
    for (let row = 0; row < image.height; row++) {
      if (row >= capRows && row < image.height - capRows) continue;
      for (let i = row * image.width; i < (row + 1) * image.width; i++) poles = Math.max(poles, radii[i]!);
    }
    const texture = new DataTexture(values, image.width, image.height, RedFormat, HalfFloatType);
    texture.magFilter = LinearFilter;
    texture.minFilter = LinearFilter;
    texture.wrapS = RepeatWrapping;
    texture.wrapT = ClampToEdgeWrapping;
    texture.generateMipmaps = false;
    texture.needsUpdate = true;
    this.shared.uShape.value = texture;
    this.shared.uShapeTexel.value.set(1 / image.width, 1 / image.height);
    this.shared.uHasShape.value = 1;
    this.shapeRadii = { radii, width: image.width, height: image.height, poles };
    this.requestRender();
  }

  /** Shows or hides the cloud layer; the first time it shows, it is downloaded. */
  setCloudsVisible(visible: boolean): void {
    this.requestRender();
    this.cloudsOn = visible;
    const source = this.cloudSource;
    if (visible && source && !source.loading) {
      this.loadClouds().catch((e: unknown) => {
        if (!source.signal?.aborted) {
          console.warn('Could not load the clouds', e);
        }
      });
    }
  }

  /**
   * A giant's storms (see lib/giantStorms.ts), found in its biome map:
   * each drifts whole at the wind of its centre, rather than sheared with
   * its bands. Empty for none.
   */
  setGiantStorms(storms: readonly GiantStorm[]): void {
    const slots = this.u.uStorms.value;
    storms.slice(0, MAX_STORMS).forEach((s, k) => slots[k]!.set(s.u, s.v, s.halfU, s.halfV));
    this.u.uStormCount.value = Math.min(storms.length, MAX_STORMS);
    this.requestRender();
  }

  /**
   * Shows a giant planet (null for a rocky world): its globe flattened by
   * its spin, its temperatures on a range of their own, its wind on the
   * moisture layer, its bands drifting each with its wind (with the cloud
   * drift), and its rings.
   */
  setGiant(info: GiantInfo | null): void {
    this.shared.uFlattening.value = info?.flattening ?? 0;
    this.u.uGiant.value = info ? 1 : 0;
    // Clouds above about 1 100 K glow red, brighter the hotter.
    const t = info ? Math.min(Math.max((info.effective_k - 1100) / 1100, 0), 1) : 0;
    this.u.uHotGlow.value = 0.6 * t * t * (3 - 2 * t);
    const [low, high] = info ? giantTemperatureRange(info) : [-30, 40];
    this.u.uTempRange.value.set(low, high - low);
    const reach = info ? Math.max(info.wind_east, info.wind_west, 1) : 1;
    // The wind every 2°: Jupiter's jets are only 4° to 8° wide, and turn
    // east and west in turn.
    this.u.uWind.value = Array.from({ length: 91 }, (_, k) => (info?.wind_profile[k] ?? 0) / reach);
    for (const pass of this.atmosphere) {
      pass.scale.y = pass.scale.x * (1 - (info?.flattening ?? 0));
    }
    if (this.rings) {
      this.scene.remove(this.rings);
      this.rings.geometry.dispose();
      this.rings.material.dispose();
      this.rings = null;
    }
    const r = info?.rings;
    // Their shadow on the globe (the surface shader).
    this.u.uRingInner.value = r?.inner ?? 0;
    this.u.uRingOuter.value = r?.outer ?? 0;
    this.u.uRingOpacity.value = r?.opacity ?? 0;
    this.u.uRingGapCount.value = Math.min(r?.gaps.length ?? 0, 4);
    this.u.uRingGaps.value.forEach((gap, k) => gap.set(...(r?.gaps[k] ?? [0, 0])));
    if (r) {
      const gaps = Array.from({ length: 4 }, (_, k) => new Vector2(...(r.gaps[k] ?? [0, 0])));
      this.rings = new Mesh(
        new RingGeometry(r.inner, r.outer, 256, 4),
        new ShaderMaterial({
          vertexShader: ringVertex,
          fragmentShader: ringFragment,
          uniforms: {
            uColour: { value: new Color().setRGB(...r.colour, SRGBColorSpace) },
            uInner: { value: r.inner },
            uOuter: { value: r.outer },
            uOpacity: { value: r.opacity },
            uGaps: { value: gaps },
            uGapCount: { value: Math.min(r.gaps.length, 4) },
            uGapPixels: { value: SKY_BODY_MIN_PX },
            uSeed: { value: (info.mass_earth * 7.31) % 13 },
            uLightMode: this.u.uLightMode,
            uSunDir: this.u.uSunDir,
            uFade: { value: 1 },
          },
          transparent: true,
          depthWrite: false,
          side: DoubleSide,
        }),
      );
      // In the equatorial plane (y is the spin axis).
      this.rings.rotation.x = -Math.PI / 2;
      this.rings.renderOrder = 1;
      this.scene.add(this.rings);
    }
    this.requestRender();
  }

  /**
   * The dust rings of the giant shown, as its planetary system drew them
   * (`DustRing` in crates/mapgen/src/rings.rs): broad, smooth and faint, in
   * its equatorial plane like the main rings; none clears them.
   */
  setDustRings(rings: readonly DustRing[]): void {
    for (const mesh of this.dustRings) {
      this.scene.remove(mesh);
      mesh.geometry.dispose();
      mesh.material.dispose();
    }
    this.dustRings = rings.map((ring) => {
      const mesh = new Mesh(
        new RingGeometry(ring.inner, ring.outer, 256, 8),
        new ShaderMaterial({
          vertexShader: dustRingVertex,
          fragmentShader: dustRingFragment,
          uniforms: {
            uColour: { value: new Color().setRGB(...ring.colour, SRGBColorSpace) },
            uInner: { value: ring.inner },
            uOuter: { value: ring.outer },
            uPeak: { value: ring.peak },
            uOpacity: { value: ring.opacity },
            uLightMode: this.u.uLightMode,
            uSunDir: this.u.uSunDir,
            uFade: { value: 1 },
          },
          transparent: true,
          depthWrite: false,
          side: DoubleSide,
        }),
      );
      mesh.rotation.x = -Math.PI / 2;
      mesh.frustumCulled = false;
      // Behind the main rings where they overlap.
      mesh.renderOrder = 0;
      this.scene.add(mesh);
      return mesh;
    });
    this.requestRender();
  }

  /**
   * The rocks of its giant's rings round the ring moon shown (see
   * lib/ringRocks.ts), `scale` times their true size (1: true size, more
   * to see them from afar), under Sunlight, in the profiles that draw them
   * (QualityProfile.ringRocks); null hides them.
   */
  setRingRocks(rocks: RingRocks | null, scale = 1): void {
    this.requestRender();
    this.ringRocksOn = rocks !== null && this.quality.ringRocks;
    if (!rocks || !this.quality.ringRocks) {
      if (this.ringRocks) this.ringRocks.visible = false;
      if (this.ringGrains) this.ringGrains.visible = false;
      return;
    }
    if (!this.ringRocks) {
      // One low rock (20 flat faces), drawn once per slot of the window.
      const rock = new IcosahedronGeometry(1, 0);
      const geometry = new InstancedBufferGeometry();
      geometry.setAttribute('position', rock.getAttribute('position'));
      geometry.setAttribute('normal', rock.getAttribute('normal'));
      geometry.setAttribute('aSlot', new InstancedBufferAttribute(latticeSlots(ROWS, WINDOW, ROCKS_PER_CELL), 3));
      geometry.instanceCount = RING_ROCK_SLOTS;
      this.ringRocks = new Mesh(
        geometry,
        new ShaderMaterial({
          vertexShader: ringRockVertex,
          fragmentShader: ringRockFragment,
          uniforms: {
            ...this.latticeUniforms(ROWS, CELL_KM),
            uBoulderM: { value: BOULDER_M.bright },
            uScale: { value: 1 },
          },
          // Faint while under a pixel; drawn after the rings of the sky.
          transparent: true,
        }),
      );
      this.ringRocks.frustumCulled = false;
      this.ringRocks.renderOrder = SKY_FRONT_ORDER + 1;
      this.scene.add(this.ringRocks);
      // The grain beyond them: a point per slot of the coarser window.
      const grains = new BufferGeometry();
      grains.setAttribute('aSlot', new BufferAttribute(latticeSlots(GRAIN_ROWS, GRAIN_WINDOW, GRAINS_PER_CELL), 3));
      // three.js needs a position attribute to count the points.
      grains.setAttribute('position', new BufferAttribute(new Float32Array(RING_GRAIN_SLOTS * 3), 3));
      this.ringGrains = new Points(
        grains,
        new ShaderMaterial({
          vertexShader: ringGrainVertex,
          fragmentShader: ringGrainFragment,
          uniforms: {
            ...this.latticeUniforms(GRAIN_ROWS, GRAIN_CELL_KM),
            uPointSize: { value: 1 },
            uBoulderM: { value: BOULDER_M.bright },
            uScale: { value: 1 },
          },
          transparent: true,
          depthWrite: false,
        }),
      );
      this.ringGrains.frustumCulled = false;
      this.ringGrains.renderOrder = SKY_FRONT_ORDER + 1;
      this.scene.add(this.ringGrains);
    }
    const lattices: [Record<string, IUniform>, LatticeWindow][] = [
      [this.ringRocks.material.uniforms, rocks],
      [this.ringGrains!.material.uniforms, rocks.grains],
    ];
    for (const [u, window] of lattices) {
      window.rows.forEach((row, i) => (u['uRows']!.value as Vector4[])[i]!.set(row.offsetKm, row.cell, row.into, row.cells));
      u['uFirstRow']!.value = window.firstRow;
      u['uAKm']!.value = rocks.aKm;
      u['uGiantKm']!.value = rocks.giantRadiusKm;
      u['uMoonKm']!.value = rocks.moonRadiusKm;
      (u['uFoot']!.value as Vector3).set(...rocks.foot);
      (u['uOut']!.value as Vector3).set(...rocks.out);
      (u['uAhead']!.value as Vector3).set(...rocks.ahead);
      (u['uNormal']!.value as Vector3).set(...rocks.normal);
      (u['uGiantCentre']!.value as Vector3).set(...rocks.giantCentre);
      u['uGiantRadius']!.value = rocks.giantRadius;
      (u['uColour']!.value as Color).setRGB(...rocks.rings.colour, SRGBColorSpace);
      // The rings' own gaps, not widened to a pixel: no rock in them.
      setRingUniforms(u, rocks.rings, 0);
    }
    for (const [u] of lattices) {
      u['uBoulderM']!.value = rocks.rings.bright ? BOULDER_M.bright : BOULDER_M.faint;
      u['uScale']!.value = scale;
    }
  }

  /** The uniforms of a window of a ring's lattice of `rows` rows of cells `cellKm` wide (see ringRockShaders). */
  private latticeUniforms(rows: number, cellKm: number): Record<string, IUniform> {
    return {
      uRows: { value: Array.from({ length: rows }, () => new Vector4()) },
      uFirstRow: { value: 0 },
      uCellKm: { value: cellKm },
      uAKm: { value: 1 },
      uGiantKm: { value: 1 },
      uMoonKm: { value: 1 },
      uFoot: { value: new Vector3() },
      uOut: { value: new Vector3() },
      uAhead: { value: new Vector3() },
      uNormal: { value: new Vector3() },
      uGiantCentre: { value: new Vector3() },
      uGiantRadius: { value: 1 },
      uSunDir: this.u.uSunDir,
      uPixel: { value: 0.001 },
      uFar: { value: 100 },
      uFade: { value: 1 },
      uColour: { value: new Color() },
      uRingInner: { value: 0 },
      uRingOuter: { value: 0 },
      uRingOpacity: { value: 0 },
      uRingGaps: { value: Array.from({ length: 4 }, () => new Vector2()) },
      uRingGapCount: { value: 0 },
      uRingGapLeast: { value: 0 },
    };
  }

  /** Shows the ring rocks and grain this frame, if any, at the camera's pixel and far plane. */
  private drawRingRocks(): void {
    const visible = this.ringRocksOn && this.u.uLightMode.value === 1 && this.morph > 0.01;
    for (const lattice of [this.ringRocks, this.ringGrains]) {
      if (!lattice) continue;
      lattice.visible = visible;
      if (!visible) continue;
      const u = lattice.material.uniforms;
      u['uPixel']!.value = (2 * Math.tan((this.camera.fov * Math.PI) / 360)) / Math.max(this.renderer.domElement.height, 1);
      u['uFar']!.value = this.camera.far;
      u['uFade']!.value = this.morph ** 3;
    }
    if (visible && this.ringGrains) this.ringGrains.material.uniforms['uPointSize']!.value = this.renderer.getPixelRatio();
  }

  /** Lets the cloud layer drift slowly eastwards around the planet, or stops it where it is. */
  setCloudsDrifting(drifting: boolean): void {
    this.cloudsDrifting = drifting;
    this.requestRender();
  }

  /**
   * Moves the drifting clouds to the current time and, while they show,
   * schedules the next redraw; stops the clock otherwise.
   */
  private driftClouds(): void {
    // A giant's bands drift too, each with its own wind.
    const visible = this.u.uCloudsOn.value > 0 || this.u.uGiant.value > 0;
    if (!this.cloudsDrifting || !visible) {
      this.cloudDriftAt = null;
      return;
    }
    const now = performance.now();
    if (this.cloudDriftAt !== null) {
      // A hidden tab draws no frames: resume where the clouds were.
      const seconds = Math.min(now - this.cloudDriftAt, 1000) / 1000;
      this.u.uCloudShift.value = (this.u.uCloudShift.value + seconds / CLOUD_DRIFT_PERIOD_S) % 1;
      this.u.uBandTime.value += seconds;
    }
    this.cloudDriftAt = now;
    if (this.cloudDriftTimer === null) {
      this.cloudDriftTimer = setTimeout(() => {
        this.cloudDriftTimer = null;
        this.requestRender();
      }, CLOUD_DRIFT_FRAME_MS);
    }
  }

  /** Frees the cloud texture. */
  private disposeClouds(): void {
    this.u.uClouds.value.dispose();
    this.u.uClouds.value = emptyTexture();
  }

  /** Downloads and decodes an equirectangular image, north up, within the GPU's texture size. */
  private async decodeImage(
    url: string,
    signal?: AbortSignal,
    onProgress?: (progress: LoadProgress) => void,
  ): Promise<ImageBitmap> {
    const response = await fetch(url, signal ? { signal } : {});
    if (!response.ok) {
      throw new Error(`${response.status} ${response.statusText}`);
    }
    const label = fileName(url);
    const blob = onProgress
      ? new Blob([
          await readWithProgress(response, ({ received, total }) =>
            onProgress({ stage: 'download', label, done: received, total }),
          ),
        ])
      : await response.blob();
    signal?.throwIfAborted();
    onProgress?.({ stage: 'decode', label, done: 0, total: null });
    // ImageBitmaps ignore texture.flipY: flip while decoding instead, so
    // row 0 (north) ends at the top of the texture, as geo.ts expects.
    let bitmap = await createImageBitmap(blob, { imageOrientation: 'flipY' });
    const factor = fitFactor(bitmap.width, bitmap.height, this.renderer.capabilities.maxTextureSize);
    if (factor > 1) {
      // Too large for this GPU: decode again, scaled down.
      const { width, height } = bitmap;
      bitmap.close();
      bitmap = await createImageBitmap(blob, {
        imageOrientation: 'flipY',
        resizeWidth: Math.ceil(width / factor),
        resizeHeight: Math.ceil(height / factor),
        resizeQuality: 'high',
      });
    }
    return bitmap;
  }

  /**
   * Shows a world map (an equirectangular image URL). The image is decoded
   * off the main thread. Aborting `signal` drops the load: the promise
   * rejects and the current map stays. A later call does not cancel an
   * earlier one by itself, but the view keeps only the map of the latest call.
   * `onProgress` follows the download (bytes received) and the decoding.
   */
  async setTexture(url: string, signal?: AbortSignal, onProgress?: (progress: LoadProgress) => void): Promise<void> {
    this.textureRequest += 1;
    const request = this.textureRequest;
    const bitmap = await this.decodeImage(url, signal, onProgress);
    if (this.disposed || request !== this.textureRequest || signal?.aborted) {
      bitmap.close();
      signal?.throwIfAborted();
      return;
    }
    const texture = new Texture(bitmap);
    texture.flipY = false;
    texture.colorSpace = SRGBColorSpace;
    // Repeated across longitudes: a giant's drifting bands read it past
    // the seam (see the surface shader), continuously, so that no line of
    // its coarsest mip level shows where they wrap.
    texture.wrapS = RepeatWrapping;
    texture.anisotropy = this.renderer.capabilities.getMaxAnisotropy();
    texture.needsUpdate = true;
    // The bitmap is closed once uploaded: only the GPU copy stays.
    releaseOnUpload(texture);
    this.disposeTexture();
    this.texture = texture;
    this.u.uMap.value = texture;
    this.requestRender();
  }

  /** Frees the GPU copy of the base map, and its decoded image if it was never uploaded. */
  private disposeTexture(): void {
    if (this.texture) {
      this.texture.dispose();
      (this.texture.image as ImageBitmap).close();
      this.texture = null;
    }
  }

  /**
   * Uploads the decoded layers, or clears them with null. `coast` is the
   * elevation of the coastline: the world's sea level plus its preset
   * waterline. `liquidSea` is false on worlds whose seas have dried up: their
   * basins are then shaded and raised like land, without sun glint.
   * `reliefScale` is the body's radii per elevation unit at its true relief
   * (lib/relief.ts), which the relief exaggeration multiplies.
   */
  setData(layers: WorldLayers | null, coast: number, liquidSea = true, reliefScale = 0): void {
    this.requestRender();
    this.shared.uData.value.dispose();
    this.u.uExtra.value.dispose();
    this.u.uBiomes.value.dispose();
    this.u.uExtra.value = emptyTexture();
    this.pendingExtra = null;
    this.u.uBiomes.value = emptyTexture();
    this.u.uHasExtra.value = 0;
    this.u.uHasBiomes.value = 0;
    if (!layers) {
      this.shared.uData.value = emptyTexture();
      this.shared.uHasData.value = 0;
      this.peakAbove = 0;
      return;
    }
    // Its highest point over the coastline, in elevation units, for the
    // guides drawn out of its surface (see render).
    let highest = 0;
    for (const raw of layers.elevation.data) if (raw > highest) highest = raw;
    this.peakAbove = Math.max(0, layers.elevation.min + highest * layers.elevation.scale - coast);
    // Worlds wider than the GPU allows are shown at a lower resolution (the
    // point readout still uses every pixel).
    const factor = fitFactor(layers.width, layers.height, this.renderer.capabilities.maxTextureSize);
    const fit = <T extends Uint8Array | Uint16Array>(data: T, channels: number) =>
      downsample(data, layers.width, layers.height, channels, factor);
    const ids = fit(layers.biomes, 1);
    const biomes = new DataTexture(ids.data, ids.width, ids.height, RedFormat, UnsignedByteType);
    // Rows of an odd width are not 4-byte aligned.
    biomes.unpackAlignment = 1;
    biomes.magFilter = NearestFilter;
    biomes.minFilter = NearestFilter;
    biomes.wrapS = RepeatWrapping;
    biomes.wrapT = ClampToEdgeWrapping;
    biomes.generateMipmaps = false;
    biomes.needsUpdate = true;
    releaseOnUpload(biomes, factor > 1);
    this.u.uBiomes.value = biomes;
    this.u.uHasBiomes.value = 1;
    // The layers of format version 2 are uploaded on first use: most views
    // never show them, and they take as much GPU memory as uData.
    this.pendingExtra = layers.extra
      ? { packed: layers.extra.packed, width: layers.width, height: layers.height, factor }
      : null;
    if (layers.extra) {
      this.u.uHasExtra.value = 1;
      this.u.uCoastMax.value = layers.extra.coastMaxKm;
    }
    this.uploadExtraIfShown();
    const surface = fit(layers.surface, 4);
    const texture = halfFloatTexture(surface.data, surface.width, surface.height, factor > 1, this.halfFloatMipmaps());
    this.shared.uData.value = texture;
    this.shared.uHasData.value = 1;
    this.shared.uCoast.value = coast;
    this.shared.uReliefScale.value = reliefScale;
    this.applySky();
    this.shared.uLiquidSea.value = liquidSea ? 1 : 0;
    this.u.uTexel.value.set(1 / surface.width, 1 / surface.height);
    this.u.uMapSize.value.set(layers.width, layers.height);
  }

  /** Whether this GPU can build mipmaps of half-float textures (it must render to them). */
  private halfFloatMipmaps(): boolean {
    const extensions = this.renderer.extensions;
    return extensions.has('EXT_color_buffer_half_float') || extensions.has('EXT_color_buffer_float');
  }

  /** Uploads the layers of format version 2 when the layer shown needs them and they are not on the GPU yet. */
  private uploadExtraIfShown(): void {
    const pending = this.pendingExtra;
    if (!pending || !EXTRA_LAYERS.has(this.layer)) {
      return;
    }
    const packed = downsample(pending.packed, pending.width, pending.height, 4, pending.factor);
    this.u.uExtra.value.dispose();
    this.u.uExtra.value = halfFloatTexture(packed.data, packed.width, packed.height, pending.factor > 1, this.halfFloatMipmaps());
    // Only the GPU copy is needed from now on.
    this.pendingExtra = null;
  }

  setLayer(layer: Layer): void {
    this.requestRender();
    this.u.uLayer.value = LAYER_IDS[layer];
    this.layer = layer;
    this.uploadExtraIfShown();
    this.tiles.setEnabled(layer === 'biomes' && !this.hexOn);
  }

  /**
   * Streams detail tiles of a world when zooming in (`revision` keys the
   * browser cache), at an epoch in Ma when set, or stops with null.
   */
  setDetail(world: { id: string; revision: number; baseWidth: number; epochMa?: number } | null): void {
    this.requestRender();
    this.tiles.setWorld(world);
  }

  /** Relief exaggeration: 0 is flat, 1 the true relief (see lib/relief.ts). */
  setRelief(relief: number): void {
    this.requestRender();
    this.shared.uRelief.value = relief;
    this.applySky();
  }

  setLighting(lighting: Lighting): void {
    this.requestRender();
    this.u.uLightMode.value = lighting.kind === 'sun' ? 1 : 0;
    if (lighting.kind === 'sun') {
      const declination =
        lighting.latitudeDeg !== undefined
          ? (lighting.latitudeDeg * Math.PI) / 180
          : ((lighting.axialTiltDeg * Math.PI) / 180) * Math.sin(2 * Math.PI * lighting.season);
      const [x, y, z] = lonLatToSphere({ lon: (lighting.longitudeDeg * Math.PI) / 180, lat: declination });
      this.u.uSunDir.value.set(x, y, z);
    }
  }

  /**
   * The world's star as seen from the planet: its colour (linear RGB, the
   * Sun white), apparent diameter in degrees and the light the planet gets
   * relative to Earth's. Drawn with **Sunlight**, at the sun's direction.
   */
  setHostStar(star: HostStar): void {
    this.requestRender();
    this.host = star;
    this.u.uSunRadius.value = ((star.angularDiameterDeg / 2) * Math.PI) / 180;
    const u = this.hostStar.material.uniforms;
    (u['uColor']!.value as Vector3).set(...star.colour);
    this.applyLightTint();
  }

  /**
   * Where the host star is, in radii of the body shown and in its frame,
   * with its radius: drawn from there, so that the camera moving far out
   * sees it at the centre of the orbits round it. Null draws it at
   * infinity, at the sun's direction.
   */
  setStarPlace(place: { position: Vec3; radius: number } | null): void {
    this.starPlace = place ? { position: new Vector3(...place.position), radius: place.radius } : null;
    this.requestRender();
  }

  /**
   * How far out the camera may go on the globe, in radii of the body
   * from its centre: at least the usual reach, more to see a system of
   * moons whole. The camera is brought back in if it was further.
   */
  setGlobeReach(radii: number): void {
    const reach = Math.max(GLOBE_MAX_DISTANCE, radii);
    if (reach === this.globeReach) return;
    this.globeReach = reach;
    if (this.mode === 'globe') {
      this.controls.maxDistance = reach;
      if (this.camera.position.length() > reach) {
        this.camera.position.setLength(reach);
        this.controls.update();
      }
    }
    this.requestRender();
  }

  /**
   * Lights the planet and its sky with its star's light ("Star light"),
   * white-balanced on the Sun, or with white light.
   */
  setStarLight(on: boolean): void {
    this.requestRender();
    this.starLightOn = on;
    this.applyLightTint();
  }

  private applyLightTint(): void {
    const [r, g, b] = this.starLightOn ? this.host.colour : [1, 1, 1];
    this.u.uLightTint.value.set(r, g, b);
    const [cool, hot] = this.starLightOn ? vegetationShift(this.host.teffK) : [0, 0];
    this.u.uVegShift.value.set(cool, hot);
  }

  /** Share of vegetation of each biome id (0 to 1), for "Star light". */
  setVegetation(shares: readonly number[]): void {
    this.requestRender();
    const value = this.u.uVegetation.value;
    value.fill(0);
    shares.slice(0, value.length).forEach((share, id) => (value[id] = share));
  }

  /** Shows or hides the star field around the globe. */
  setStars(visible: boolean): void {
    this.requestRender();
    this.starsOn = visible;
  }

  /**
   * Turns the star field about the spin axis by `radians`: a moon locked to
   * its planet turns once per orbit against the stars, its planet fixed in
   * its sky.
   */
  setStarsTurn(radians: number, follow = false): void {
    // Not wrapped: a body turning more than half a turn between two frames
    // keeps its sense of rotation.
    const delta = radians - this.starsTurned;
    this.starsTurned = radians;
    const wrapped = radians - 2 * Math.PI * Math.floor(radians / (2 * Math.PI));
    this.stars.rotation.y = wrapped;
    // The arrow of the sense of rotation is fixed against the stars, not
    // turning with the globe.
    if (this.spinAxis) this.spinAxis.rotation.y = wrapped;
    this.guides.spin.rotation.y = wrapped;
    // The camera turned with them about the spin axis, on the globe: the
    // camera is still against the stars, the sun and the orbits, and the
    // globe turns under it.
    if (follow && delta !== 0 && this.mode === 'globe' && !this.animation) {
      const turn = new Quaternion().setFromAxisAngle(new Vector3(0, 1, 0), delta);
      this.camera.position.applyQuaternion(turn);
      this.camera.up.applyQuaternion(turn);
      this.controls.target.applyQuaternion(turn);
      this.camera.lookAt(this.controls.target);
      this.controls.update();
    }
    this.requestRender();
  }

  setAtmosphere(visible: boolean): void {
    this.requestRender();
    this.atmosphereOn = visible;
  }

  /** The world's sky (see skyFromWorld), or null for an airless world. */
  setSky(sky: Sky | null): void {
    this.requestRender();
    this.hasSky = sky !== null;
    if (sky) {
      this.worldSky = sky;
      this.applySky();
    }
  }

  /** How far the relief shown rises at most, in radii: an elevation unit above the coastline, as displaced. */
  private reliefLift(): number {
    return this.shared.uRelief.value * this.shared.uReliefScale.value;
  }

  /** Sets the sky uniforms from the world's sky, raised above the displaced relief. */
  private applySky(): void {
    const sky = liftSky(this.worldSky, this.reliefLift());
    for (const pass of this.atmosphere) {
      pass.scale.setScalar(sky.top);
      pass.scale.y *= 1 - this.shared.uFlattening.value;
    }
    const s = this.sky;
    if (s.uTop.value !== sky.top || s.uHeightR.value !== sky.heightR || s.uHeightM.value !== sky.heightM) {
      s.uSunDepth.value.dispose();
      s.uSunDepth.value = sunDepthTexture(sky.top, sky.heightR, sky.heightM);
    }
    this.sky.uTop.value = sky.top;
    this.sky.uBetaR.value.set(...sky.betaR);
    this.sky.uBetaM.value = sky.betaM;
    this.sky.uMieAlbedo.value.set(...sky.mieAlbedo);
    this.sky.uHeightR.value = sky.heightR;
    this.sky.uHeightM.value = sky.heightM;
    this.sky.uG.value = sky.g;
  }

  /** Shows a hex grid over the map (majority biome per hex), or hides it with null. */
  setHex(map: HexMap | null): void {
    this.requestRender();
    this.u.uHex.value.dispose();
    if (!map) {
      this.u.uHex.value = emptyTexture();
      this.u.uHexOn.value = 0;
      this.hexOn = false;
      this.tiles.setEnabled(this.layer === 'biomes');
      return;
    }
    this.hexOn = true;
    this.tiles.setEnabled(false);
    this.u.uHex.value = hexTexture(map);
    this.u.uHexGrid.value = [map.columns, map.rows, map.hex_width_px, map.hex_size_px];
    this.u.uMapSize.value.set(map.source.width, map.source.height);
    this.u.uHexOn.value = 1;
  }

  setMode(mode: ViewMode): void {
    this.requestRender();
    if (mode === this.mode) {
      return;
    }
    // The place at the centre of the screen stays there, at an equivalent
    // zoom: the camera follows it through the morph.
    const centre = this.viewCentre();
    const relief = this.reliefLift();
    let altitude: number;
    let mapHeight: number;
    if (this.mode === 'globe') {
      altitude = this.altitude();
      mapHeight = Math.min(MAP_MAX_DISTANCE, Math.max(MAP_MIN_DISTANCE, mapHeightForAltitude(altitude)));
    } else {
      mapHeight = this.altitude();
      altitude = Math.min(GLOBE_MAX_DISTANCE - 1, Math.max(MIN_ALTITUDE + relief, altitudeForMapHeight(mapHeight)));
    }
    let centreTo = centre;
    if (mode === 'map') {
      // Zoomed out, the map stays in view rather than half off screen.
      const halfHeight = (mapHeight * Math.tan((this.camera.fov * Math.PI) / 360)) / this.camera.zoom;
      const [x, y] = mapPoint(centre);
      const [cx, cy] = clampMapCentre(x, y, halfHeight * this.camera.aspect, halfHeight);
      centreTo = planeToLonLat(cx, cy) ?? centre;
    }
    this.mode = mode;
    this.animation = {
      from: this.morph,
      to: mode === 'globe' ? 1 : 0,
      start: performance.now(),
      follow: true,
      centreFrom: centre,
      centreTo,
      altitude,
      mapHeight,
    };
    this.applyModeControls();
  }

  /** Longitude and latitude at the centre of the screen (the closest point of the map when off it). */
  private viewCentre(): LonLat {
    this.ndc.set(0, 0);
    this.raycaster.setFromCamera(this.ndc, this.camera);
    const origin = this.raycaster.ray.origin.toArray();
    const direction = this.raycaster.ray.direction.toArray();
    if (this.mode === 'globe') {
      const t = raySphere(origin, direction, 1);
      const hit = t === null ? this.camera.position : new Vector3(...origin).addScaledVector(new Vector3(...direction), t);
      return sphereToLonLat(hit.toArray());
    }
    const t = rayPlaneZ0(origin, direction) ?? 0;
    const x = Math.min(Math.PI, Math.max(-Math.PI, origin[0] + direction[0] * t));
    const y = Math.min(Math.PI / 2, Math.max(-Math.PI / 2, origin[1] + direction[1] * t));
    return planeToLonLat(x, y) ?? { lon: 0, lat: 0 };
  }

  /**
   * Places the camera for the morph under way at `progress` (0 to 1) of the
   * way, or at its end in the controls' own terms (the globe orbits its
   * centre, the map pans with a target on the plane).
   */
  private followMorph(progress: number): void {
    const animation = this.animation!;
    // Not quite at the poles, where "north up" has no direction.
    const centre = lerpLonLat(animation.centreFrom, animation.centreTo, progress);
    centre.lat = Math.max(-POLE_LIMIT, Math.min(POLE_LIMIT, centre.lat));
    const { target, position } = morphCamera(centre, animation.mapHeight, animation.altitude, this.morph);
    this.camera.position.set(...position);
    this.camera.up.set(0, 1, 0);
    if (progress < 1) {
      this.controls.target.set(...target);
      this.morphDistance = this.camera.position.distanceTo(this.controls.target);
    } else if (this.mode === 'globe') {
      this.controls.target.set(0, 0, 0);
    } else {
      this.controls.target.set(target[0], target[1], 0);
    }
    this.camera.lookAt(this.controls.target);
  }

  setGraticule(visible: boolean): void {
    this.requestRender();
    this.grid.visible = visible;
  }

  /**
   * Sets the axial tilt (degrees) the equator, tropics and polar circles
   * are drawn for, or null when no world is shown (none drawn).
   */
  setReferenceTilt(tilt: number | null): void {
    this.referenceTilt = tilt;
    this.applyReferences();
  }

  /** Shows or hides the equator, tropics and polar circles. */
  setReferenceLines(visible: boolean): void {
    this.referencesOn = visible;
    this.applyReferences();
  }

  private applyReferences(): void {
    this.u.uTilt.value = this.referenceTilt ?? 0;
    this.u.uTropics.value = this.referencesOn && this.referenceTilt !== null ? 1 : 0;
    this.requestRender();
  }

  /** Shows or hides the depth contours under water. */
  setSeaContours(visible: boolean): void {
    this.u.uSeaContours.value = visible ? 1 : 0;
    this.requestRender();
  }

  /** Shows or hides the altitude contours on land. */
  setLandContours(visible: boolean): void {
    this.u.uLandContours.value = visible ? 1 : 0;
    this.requestRender();
  }

  /** Highlights one biome (id), fading the others; null shows them all. */
  setHighlight(biome: number | null): void {
    this.requestRender();
    this.u.uHighlight.value = biome ?? -1;
  }

  /** Draws a path of great-circle legs through the points (fewer than one hides it). */
  setPath(points: readonly LonLat[]): void {
    this.requestRender();
    this.path.geometry.dispose();
    this.path.geometry = pathGeometry(points);
    this.path.visible = points.length > 0;
  }

  /** Back to the starting view of the current mode: whole planet, north up. */
  resetView(): void {
    this.requestRender();
    if (this.animation) {
      this.animation.follow = false;
    }
    this.controls.target.set(0, 0, 0);
    this.camera.position.set(0, 0, this.mode === 'globe' ? GLOBE_DISTANCE : MAP_DISTANCE);
    this.camera.up.set(0, 1, 0);
    this.camera.lookAt(0, 0, 0);
    this.controls.update();
  }

  getCamera(): CameraState {
    return {
      mode: this.mode,
      position: this.camera.position.toArray(),
      target: this.controls.target.toArray(),
      up: this.camera.up.toArray(),
    };
  }

  /**
   * Moves the camera to a state reported by another view, or read from a
   * link, switching the mode at once if it differs. A morph under way is
   * cut short: it would carry the camera elsewhere.
   */
  setCamera(state: CameraState): void {
    this.requestRender();
    this.applyingCamera = true;
    if (state.mode !== this.mode) {
      this.setMode(state.mode);
    }
    // Skip the morph: the other view has finished it already.
    this.animation = null;
    this.morph = state.mode === 'globe' ? 1 : 0;
    this.camera.position.fromArray(state.position);
    this.controls.target.fromArray(state.target);
    this.camera.up.fromArray(state.up);
    this.camera.lookAt(this.controls.target);
    this.controls.update();
    this.applyingCamera = false;
  }

  /** The current view as a PNG (without the HTML overlays). */
  screenshot(): Promise<Blob | null> {
    // The drawing buffer is only valid right after rendering: render and
    // read it in the same task.
    this.renderer.render(this.scene, this.camera);
    return new Promise((resolve) => this.canvas.toBlob(resolve, 'image/png'));
  }

  dispose(): void {
    this.disposed = true;
    liveViews.delete(this);
    balanceTileBudgets();
    if (this.frame !== null) {
      cancelAnimationFrame(this.frame);
      this.frame = null;
    }
    this.resizeObserver.disconnect();
    this.canvas.removeEventListener('wheel', this.handleWheel);
    this.canvas.removeEventListener('pointerdown', this.handlePointerDown);
    this.canvas.removeEventListener('pointerup', this.handlePointerUp);
    this.canvas.removeEventListener('pointermove', this.handlePointerMove);
    this.canvas.removeEventListener('pointerenter', this.handlePointerEnter);
    this.canvas.removeEventListener('pointerleave', this.handlePointerLeave);
    this.controls.removeEventListener('change', this.handleControlsChange);
    this.controls.dispose();
    this.tiles.dispose();
    this.disposeTexture();
    this.disposeClouds();
    if (this.cloudDriftTimer !== null) {
      clearTimeout(this.cloudDriftTimer);
    }
    this.shared.uShape.value.dispose();
    this.shared.uData.value.dispose();
    this.u.uHex.value.dispose();
    this.u.uExtra.value.dispose();
    this.u.uBiomes.value.dispose();
    this.sky.uSunDepth.value.dispose();
    this.disposeSkyBodies();
    this.skyOrbits?.geometry.dispose();
    this.skyOrbits?.material.dispose();
    this.skyOrbitsNear?.material.dispose();
    this.skyBelts?.geometry.dispose();
    this.skyBelts?.material.dispose();
    this.skyBeltsNear?.material.dispose();
    this.spinAxis?.geometry.dispose();
    this.guides.dispose();
    this.hostSpin.dispose();
    this.previewGlobe?.geometry.dispose();
    this.previewGlobe?.material.dispose();
    this.spinAxis?.material.dispose();
    for (const object of [this.surface, this.grid, this.stars, this.hostStar, ...this.atmosphere, this.path]) {
      object.geometry.dispose();
      object.material.dispose();
    }
    this.rings?.geometry.dispose();
    this.rings?.material.dispose();
    this.ringRocks?.geometry.dispose();
    this.ringRocks?.material.dispose();
    this.ringGrains?.geometry.dispose();
    this.ringGrains?.material.dispose();
    for (const mesh of this.dustRings) {
      mesh.geometry.dispose();
      mesh.material.dispose();
    }
    this.renderer.dispose();
    // Free the WebGL context now: browsers cap live contexts (about 16), and
    // compare panes come and go.
    this.renderer.forceContextLoss();
  }

  private applyModeControls(): void {
    const globe = this.mode === 'globe';
    this.controls.enableRotate = globe;
    // On the globe the right button frees the camera, as in the star view:
    // it pans on the screen's two axes, the view then turning round the
    // point reached (resetView centres it again).
    this.controls.enablePan = true;
    // On the globe the wheel is handled here, by altitude (see handleWheel);
    // on the map, OrbitControls zooms towards the cursor.
    this.controls.enableZoom = !globe;
    this.controls.zoomToCursor = !globe;
    this.controls.screenSpacePanning = true;
    this.controls.minDistance = globe ? 1 + MIN_ALTITUDE : MAP_MIN_DISTANCE;
    this.controls.maxDistance = globe ? this.globeReach : MAP_MAX_DISTANCE;
    this.controls.mouseButtons = globe
      ? { LEFT: MOUSE.ROTATE, MIDDLE: MOUSE.DOLLY, RIGHT: MOUSE.PAN }
      : { LEFT: MOUSE.PAN, MIDDLE: MOUSE.DOLLY, RIGHT: MOUSE.PAN };
  }

  private resize(): void {
    const { clientWidth, clientHeight } = this.canvas;
    const visible = clientWidth > 0 && clientHeight > 0;
    if (visible !== this.shown) {
      this.shown = visible;
      this.onVisibilityChange(visible);
    }
    if (!visible) {
      return;
    }
    this.renderer.setSize(clientWidth, clientHeight, false);
    this.camera.aspect = clientWidth / clientHeight;
    // In portrait views, zoom out so the planet fits the width as well.
    this.camera.zoom = Math.min(1, this.camera.aspect);
    this.camera.updateProjectionMatrix();
    this.requestRender();
  }

  private readonly render = (now: number): void => {
    this.frame = null;
    // Hidden: keep the pending changes (`dirty`) for the frame that the
    // resize to a visible size requests. No frame means no drift timer.
    if (!this.shown) {
      return;
    }
    // At most the profile's frames per second (see QualityProfile.maxFps):
    // too early, the frame waits for the next one, its changes pending.
    if (this.quality.maxFps !== null && now - this.lastDrawn < 1000 / this.quality.maxFps - 2) {
      this.frame = requestAnimationFrame(this.render);
      return;
    }
    this.lastDrawn = now;
    // While the camera follows the morph, it is placed here: the controls
    // (whose distance limits are those of the mode reached) wait for the end.
    let following = false;
    if (this.animation) {
      const t = Math.min(1, (now - this.animation.start) / MORPH_MS);
      const eased = t < 0.5 ? 4 * t * t * t : 1 - (-2 * t + 2) ** 3 / 2;
      this.morph = this.animation.from + (this.animation.to - this.animation.from) * eased;
      if (this.animation.follow) {
        this.followMorph(t === 1 ? 1 : eased);
        following = t < 1;
      }
      if (t === 1) {
        this.animation = null;
      }
      this.dirty = true;
    }
    // Near the surface: slower rotation, and a near plane that follows the
    // altitude so depth precision holds down to a few hundred metres.
    const altitude = this.altitude();
    this.controls.rotateSpeed = Math.min(1, Math.max(0.002, altitude * 0.6));
    // The damping keeps moving the camera after the pointer stops: frames
    // continue until it has settled. OrbitControls' own change threshold is
    // too coarse near the surface, where its residue would show as a jump at
    // the next interaction: compare the camera itself.
    if (!following) {
      this.controls.update();
      this.keepAboveGround();
    }
    const moving =
      this.lastPosition.distanceToSquared(this.camera.position) > 1e-16 ||
      1 - Math.abs(this.lastQuaternion.dot(this.camera.quaternion)) > 1e-14;
    this.lastPosition.copy(this.camera.position);
    this.lastQuaternion.copy(this.camera.quaternion);
    if (moving) {
      this.dirty = true;
    }
    // A lower resolution while the view moves; the frame after it settles
    // is drawn at full resolution again.
    if (moving || this.animation) {
      this.lastMotion = now;
    }
    const ratio = pixelRatio(window.devicePixelRatio, now - this.lastMotion, this.quality);
    const settling = ratio < fullPixelRatio(window.devicePixelRatio, this.quality);
    if (ratio !== this.renderer.getPixelRatio()) {
      // Resizes the drawing buffer (not the canvas box): redraw it now.
      this.renderer.setPixelRatio(ratio);
      this.dirty = true;
    }
    if (this.dirty) {
      this.dirty = false;
      this.draw();
    }
    if (this.hoverPending) {
      this.hoverPending = false;
      this.onHover(this.hovering ? this.pickAt(this.hoverAt.x, this.hoverAt.y) : null);
    }
    if (moving || this.animation || settling) {
      this.schedule();
    }
  };

  /**
   * Draws other bodies in the sky (with Sunlight, on the globe): the planet
   * seen from one of its moons, the moons seen from their planet, each lit
   * by the sun, so it shows its phase; at their true apparent size, at
   * least a few pixels. Their maps load in the background.
   */
  setSkyBodies(bodies: readonly SkyBody[], signal?: AbortSignal): void {
    this.skyRequest += 1;
    const request = this.skyRequest;
    // The same bodies at new places keep their meshes and maps.
    const same =
      bodies.length === this.skyBodies.length &&
      bodies.every((b, i) => {
        const old = this.skyBodies[i]?.body;
        return b.texture === old?.texture && (b.clouds ?? null) === (old?.clouds ?? null);
      });
    if (same) {
      bodies.forEach((body, i) => {
        const entry = this.skyBodies[i]!;
        entry.body = body;
        applyPreview(entry.mesh.material.uniforms, body);
      });
      this.requestRender();
      return;
    }
    this.disposeSkyBodies();
    this.skyBodies = bodies.map((body) => {
      const mesh = new Mesh(
        new PlaneGeometry(1, 1),
        new ShaderMaterial({
          vertexShader: skyBodyVertex,
          fragmentShader: skyBodyFragment,
          uniforms: {
            uDir: { value: new Vector3() },
            uRight: { value: new Vector3() },
            uUp: { value: new Vector3() },
            uHalf: { value: 0.01 },
            uSunDir: this.u.uSunDir,
            uColor: { value: new Vector3(...body.colour) },
            uColor2: { value: new Vector3() },
            uColor3: { value: new Vector3() },
            uBands: { value: 0 },
            uLand: { value: 0 },
            uPreview: { value: 0 },
            uGlow: { value: 0 },
            uHasMap: { value: 0 },
            uMap: { value: emptyTexture() },
            uHasClouds: { value: 0 },
            uClouds: { value: emptyTexture() },
            uCloudTint: { value: new Vector3(1, 1, 1) },
            uHaloColor: { value: new Vector3() },
            uHaloHeight: { value: 0 },
            uHaloStrength: { value: 0 },
            uGrow: { value: 1 },
            uRing: { value: 0 },
            uSpinAxis: { value: 0 },
            uSpinArrow: { value: 1 },
            uOpacity: { value: 1 },
            uAxisX: { value: new Vector3(1, 0, 0) },
            uAxisY: { value: new Vector3(0, 1, 0) },
            uAxisZ: { value: new Vector3(0, 0, 1) },
            uGrid: { value: 0 },
            uTropics: { value: 0 },
            uTilt: { value: 0 },
            uRingInner: { value: 0 },
            uRingOuter: { value: 0 },
            uRingOpacity: { value: 0 },
            uRingGaps: { value: Array.from({ length: 4 }, () => new Vector2()) },
            uRingGapCount: { value: 0 },
            uRingGapLeast: { value: 0 },
          },
          // Drawn with the star field and the host star (transparent ones
          // come after the surface), over them: a body hides the stars
          // and its star behind it.
          transparent: true,
          depthWrite: false,
        }),
      );
      mesh.frustumCulled = false;
      mesh.renderOrder = -1;
      mesh.visible = false;
      this.scene.add(mesh);
      const entry = { mesh, body, maps: [] as Texture[], rings: null as Mesh<BufferGeometry, ShaderMaterial> | null };
      const load = (url: string, map: string, has: string) => {
        void this.decodeImage(url, signal)
          .then((bitmap) => {
            if (this.disposed || request !== this.skyRequest) {
              bitmap.close();
              return;
            }
            const texture = new Texture(bitmap);
            texture.flipY = false;
            texture.colorSpace = map === 'uMap' ? SRGBColorSpace : NoColorSpace;
            // Repeated across longitudes: the shader's u runs past the seam.
            texture.wrapS = RepeatWrapping;
            texture.needsUpdate = true;
            releaseOnUpload(texture);
            entry.maps.push(texture);
            mesh.material.uniforms[map]!.value = texture;
            mesh.material.uniforms[has]!.value = 1;
            this.requestRender();
          })
          .catch((e: unknown) => {
            if (!signal?.aborted) console.warn('Could not load a body of the sky', e);
          });
      };
      applyPreview(mesh.material.uniforms, body);
      if (body.texture) load(body.texture, 'uMap', 'uHasMap');
      if (body.clouds) load(body.clouds, 'uClouds', 'uHasClouds');
      return entry;
    });
    this.requestRender();
  }

  /**
   * Bodies whose shadow can fall on the surface with Sunlight (eclipses):
   * their centres and radii in radii of the body shown (see lib/skyBodies).
   */
  setShadowCasters(casters: readonly ShadowCaster[]): void {
    const slots = this.u.uCasters.value;
    casters.slice(0, MAX_CASTERS).forEach((c, i) => slots[i]!.set(...c.position, c.radius));
    this.u.uCasterCount.value = Math.min(casters.length, MAX_CASTERS);
    this.requestRender();
  }

  /**
   * Draws orbits in the sky, at infinity like the sky bodies (on the
   * globe): each loop a closed line through points in radii of the body
   * shown, in its frame (see lib/skyBodies `orbitLoops`), each drawn in its
   * direction from the camera. `dashed[i]` dashes loop `i` (the body
   * shown's own orbit, so that it is seen sliding along it). Empty to clear.
   */
  setSkyOrbits(loops: readonly (readonly Vec3[])[], dashed: readonly boolean[] = []): void {
    if (!this.skyOrbits) {
      [this.skyOrbits, this.skyOrbitsNear] = this.skyLines(new Color(0.55, 0.62, 0.75), 0.45);
    }
    this.skyOrbitsOn = fillSkyLines(this.skyOrbits.geometry, loops, dashed);
    this.skyOrbitDashed = loops.reduce((most, loop, k) => (dashed[k] ? Math.max(most, loopLength(loop)) : most), 0);
    this.applySkyOrbitDash();
  }

  /**
   * How many times a second the body shown goes round its dashed orbit
   * (0 while the orbits stand still): its dashes lengthen with the pace so
   * that they are seen moving (see skyOrbitDash).
   */
  setSkyOrbitPace(orbitsPerSecond: number): void {
    this.skyOrbitPace = orbitsPerSecond;
    this.applySkyOrbitDash();
  }

  private skyOrbitDashed = 0;
  private skyOrbitPace = 0;

  private applySkyOrbitDash(): void {
    const dash = skyOrbitDash(this.skyOrbitDashed, this.skyOrbitPace);
    for (const lines of [this.skyOrbits, this.skyOrbitsNear]) {
      if (lines) lines.material.uniforms['uDash']!.value = dash;
    }
    this.requestRender();
  }

  /**
   * The edges of the system's belts in the sky (see smallBodiesSky in
   * lib/systemSky.ts), loops of points in radii of the body shown, drawn
   * as the orbits are, fainter and warmer; none hides them.
   */
  setSkyBelts(loops: readonly (readonly Vec3[])[]): void {
    if (!this.skyBelts) {
      [this.skyBelts, this.skyBeltsNear] = this.skyLines(new Color(0.85, 0.72, 0.5), 0.3);
    }
    this.skyBeltsOn = fillSkyLines(this.skyBelts.geometry, loops, []);
    this.requestRender();
  }

  /**
   * A pair of line sets drawn over the sky: its far part under the sky
   * bodies, its near part after the globe (see skyOrbitFragment).
   */
  private skyLines(
    colour: Color,
    opacity: number,
  ): [LineSegments<BufferGeometry, ShaderMaterial>, LineSegments<BufferGeometry, ShaderMaterial>] {
    const far = new LineSegments(
      new BufferGeometry(),
      new ShaderMaterial({
        vertexShader: skyOrbitVertex,
        fragmentShader: skyOrbitFragment,
        uniforms: {
          uColor: { value: colour },
          uOpacity: { value: opacity },
          uNear: { value: 0 },
          uDash: { value: SKY_ORBIT_DASH },
          uScreenPx: { value: 1000 },
        },
        transparent: true,
        depthWrite: false,
      }),
    );
    far.frustumCulled = false;
    // Over the stars and the host star, under the bodies.
    far.renderOrder = -1.5;
    this.scene.add(far);
    const material = far.material.clone();
    material.uniforms['uNear']!.value = 1;
    const near = new LineSegments(far.geometry, material);
    near.frustumCulled = false;
    // After the globe and its air.
    near.renderOrder = 10;
    this.scene.add(near);
    return [far, near];
  }

  /**
   * Sky points to follow on the screen (see skyScreenPositions): each mark
   * a list of candidates, of which the one in view nearest its preferred
   * place on the screen is given (`prefer`, fractions of its width and
   * height; the centre by default): a swarm's centre, or points along a
   * belt's edges, each belt preferring its own place so that their labels
   * spread along the band.
   */
  setSkyMarks(marks: readonly { candidates: readonly Vec3[]; prefer?: readonly [number, number] }[]): void {
    this.skyMarks = marks;
    this.requestRender();
  }

  private skyMarks: readonly { candidates: readonly Vec3[]; prefer?: readonly [number, number] }[] = [];
  private skyBelts: LineSegments<BufferGeometry, ShaderMaterial> | null = null;
  private skyBeltsNear: LineSegments<BufferGeometry, ShaderMaterial> | null = null;
  private skyBeltsOn = false;

  private skyOrbitsOn = false;

  /**
   * Draws the spin axes, on the globe: the body shown's out of its poles,
   * and each sky body's out of its disk, along its own axis (see
   * SkyBody.axes), so that their tilts show against their orbits.
   */
  setSpinAxes(on: boolean): void {
    this.spinAxesOn = on;
    if (on && !this.spinAxis) {
      const geometry = new BufferGeometry();
      const [n, s] = [SPIN_AXIS_NORTH, SPIN_AXIS_SOUTH];
      const points = [0, 1, 0, 0, SPIN_AXIS_LENGTH, 0, 0, -1, 0, 0, -SPIN_AXIS_LENGTH, 0];
      const colours = [...n, ...n, ...s, ...s];
      geometry.setAttribute('position', new Float32BufferAttribute(points, 3));
      geometry.setAttribute('color', new Float32BufferAttribute(colours, 3));
      this.spinAxis = new LineSegments(geometry, new LineBasicMaterial({ vertexColors: true, transparent: true }));
      this.spinAxis.frustumCulled = false;
      this.spinAxis.renderOrder = 2;
      this.spinAxis.rotation.y = this.stars.rotation.y;
      this.scene.add(this.spinAxis);
    }
    this.requestRender();
  }

  /**
   * How the body shown turns, for its spin arrow: on its own (false), or
   * locked to what it orbits (true), drawn as a closed ring; null for none.
   */
  setSpinLocked(locked: boolean | null): void {
    this.guides.setSpin(locked);
    this.requestRender();
  }

  /**
   * The host planet of the body shown (a moon's planet), for a dotted line
   * to it and the arrows of its pull and of the body's velocity; null for
   * none. Shown with the spin axes.
   */
  setHostGuide(guide: HostGuide | null): void {
    this.guides.setHost(guide);
    this.hostGuideOn = guide !== null;
    this.requestRender();
  }

  private disposeSkyBodies(): void {
    for (const { mesh, maps, rings } of this.skyBodies) {
      for (const m of rings ? [mesh, rings] : [mesh]) {
        this.scene.remove(m);
        m.geometry.dispose();
        m.material.dispose();
      }
      for (const map of maps) map.dispose();
    }
    this.skyBodies = [];
  }

  /** Places the sky bodies for the current camera (see setSkyBodies). */
  /**
   * Where the bodies of the sky (in the order setSkyBodies got them) and the
   * host star are on the screen, CSS px from the canvas' top left; null for
   * one not drawn, behind the camera or hidden by the globe. Called back
   * through onSkyPositions after each frame while it is set.
   */
  skyScreenPositions(): {
    bodies: ({ x: number; y: number } | null)[];
    star: { x: number; y: number } | null;
    /** Where the body's guides end (see lib/viewer/bodyGuides), null when not drawn or hidden by the globe. */
    guides: Record<keyof GuidePoints | 'hostSpin', { x: number; y: number } | null>;
    /** Where each mark of setSkyMarks is: its candidate in view nearest the screen's centre, null when none is. */
    marks: ({ x: number; y: number } | null)[];
  } {
    const shown = this.u.uLightMode.value === 1 && this.morph > 0.99;
    const camera = this.camera.position;
    const forward = new Vector3();
    this.camera.getWorldDirection(forward);
    const width = this.canvas.clientWidth;
    const height = this.canvas.clientHeight;
    const at = (direction: Vector3, front = false): { x: number; y: number } | null => {
      if (!shown) return null;
      const d = direction.clone().normalize();
      if (d.dot(forward) <= 0) return null;
      // Hidden by the globe (radius 1 at the origin) along the ray, unless in front of it?
      const b = camera.dot(d);
      const disc = b * b - (camera.lengthSq() - 1);
      if (!front && disc > 0 && -b - Math.sqrt(disc) > 0) return null;
      const p = camera.clone().addScaledVector(d, 1e3).project(this.camera);
      if (Math.abs(p.x) > 1.2 || Math.abs(p.y) > 1.2) return null;
      return { x: ((p.x + 1) / 2) * width, y: ((1 - p.y) / 2) * height };
    };
    // A point near the globe: hidden when the globe lies in front of it,
    // or when it is off the screen by more than `margin` (clip space).
    const near = (point: Vector3 | null, margin = 1.2): { x: number; y: number } | null => {
      if (!point || this.morph < 0.99) return null;
      const to = point.clone().sub(camera);
      const distance = to.length();
      const d = to.divideScalar(distance);
      if (d.dot(forward) <= 0) return null;
      const b = camera.dot(d);
      const disc = b * b - (camera.lengthSq() - 1);
      if (disc > 0 && -b - Math.sqrt(disc) < distance - 1e-3) return null;
      const p = point.clone().project(this.camera);
      if (Math.abs(p.x) > margin || Math.abs(p.y) > margin) return null;
      return { x: ((p.x + 1) / 2) * width, y: ((1 - p.y) / 2) * height };
    };
    const points = this.guides.points();
    // The host's distance goes on the part of its line in view, wherever
    // that is as the camera moves.
    const hostLine = points.host?.map((point) => near(point, 0.9)) ?? [];
    const hostAt = hostLine[labelIndex(hostLine.map((p) => p !== null))] ?? null;
    return {
      bodies: this.skyBodies.map(({ body }) => at(this.skyPlace(body).direction, this.inFront(body))),
      star: at(
        this.starPlace ? this.starPlace.position.clone().sub(this.camera.position) : this.u.uSunDir.value,
      ),
      guides: {
        spin: near(points.spin),
        host: hostAt,
        pull: near(points.pull),
        velocity: near(points.velocity),
        hostSpin: this.hostSpinTip(near),
      },
      marks: this.skyMarks.map(({ candidates, prefer = [0.5, 0.5] }) => {
        let best: { x: number; y: number } | null = null;
        let bestDistance = Infinity;
        for (const point of candidates) {
          const p = at(new Vector3(...point).sub(camera));
          if (!p) continue;
          const distance = Math.hypot(p.x - prefer[0] * width, p.y - prefer[1] * height);
          if (distance < bestDistance && Math.abs(p.x - width / 2) < 0.45 * width && Math.abs(p.y - height / 2) < 0.45 * height) {
            best = p;
            bestDistance = distance;
          }
        }
        return best;
      }),
    };
  }

  /** Receives the sky's screen positions after each frame (see skyScreenPositions); null stops it. */
  onSkyPositions: ((positions: ReturnType<PlanetView['skyScreenPositions']>) => void) | null = null;

  /**
   * A sky body's direction from the camera and its angular radius: from
   * its position when it has one (see SkyBody.position), so that it stays
   * on its orbit line wherever the camera is; else at infinity.
   */
  private skyPlace(body: SkyBody): { direction: Vector3; half: number } {
    if (body.position && body.radius !== undefined) {
      const d = new Vector3(...body.position).sub(this.camera.position);
      const distance = Math.max(d.length(), 1e-9);
      return { direction: d.divideScalar(distance), half: Math.asin(Math.min(body.radius / distance, 0.999)) };
    }
    return {
      direction: new Vector3(...body.direction).normalize(),
      half: ((body.angularDiameterDeg / 2) * Math.PI) / 180,
    };
  }

  /** Whether a body of the sky lies between the camera and the body shown (see inFrontOfGlobe). */
  private inFront(body: SkyBody): boolean {
    if (!body.position || body.radius === undefined) return false;
    const c = this.camera.position;
    return inFrontOfGlobe([c.x, c.y, c.z], body.position, body.radius);
  }

  /**
   * The host planet in a moon's sky (the body of the sky at `index`), its
   * spin drawn in 3D round it with Spin axes, as the body shown's: an arrow
   * going the way it turns, or a ring when `locked` to its star; null for
   * none (a body without a host).
   */
  setHostSpin(host: { index: number; locked: boolean } | null): void {
    this.hostSpinOf = host;
    this.hostSpin.setSpin(host ? host.locked : null);
    // Far beyond the far plane, which its depth is held short of: never
    // culled by the camera's frustum.
    this.hostSpin.spin.traverse((object) => (object.frustumCulled = false));
    this.requestRender();
  }

  /** Where the host planet's spin arrow ends on the screen, through `near`, null when it is not drawn or the planet hides it. */
  private hostSpinTip(near: (point: Vector3 | null, margin?: number) => { x: number; y: number } | null): { x: number; y: number } | null {
    const tip = this.hostSpin.points().spin;
    if (!tip) return null;
    // Behind the planet from the camera: hidden, as its arrow is.
    const { uHostCentre, uHostRadius } = this.hostSpinUniforms;
    const to = tip.clone().sub(this.camera.position);
    const distance = to.length();
    const d = to.divideScalar(distance);
    const oc = this.camera.position.clone().sub(uHostCentre.value);
    const b = oc.dot(d);
    const h = b * b - (oc.lengthSq() - uHostRadius.value ** 2);
    if (h > 0 && -b - Math.sqrt(h) > 0 && -b - Math.sqrt(h) < distance) return null;
    return near(tip);
  }

  /** Places the host planet's spin round it (see setHostSpin), its arc starting on the side facing the body shown. */
  private drawHostSpin(visible: boolean): void {
    const entry = this.hostSpinOf ? this.skyBodies[this.hostSpinOf.index] : undefined;
    const body = entry?.body;
    const on = visible && this.spinAxesOn && !!body?.position && body.radius !== undefined && entry!.mesh.visible;
    if (on && body?.position && body.radius !== undefined) {
      const centre = new Vector3(...body.position);
      const y = new Vector3(...(body.axes ?? IDENTITY_AXES)[1]).normalize();
      const toward = centre.clone().negate();
      const z = toward.sub(y.clone().multiplyScalar(toward.dot(y)));
      if (z.lengthSq() < 1e-12) z.set(0, 0, 1).cross(y);
      z.normalize();
      const x = y.clone().cross(z);
      const spin = this.hostSpin.spin;
      spin.position.copy(centre);
      spin.quaternion.setFromRotationMatrix(new Matrix4().makeBasis(x, y, z));
      spin.scale.setScalar(body.radius);
      this.hostSpinUniforms.uHostCentre.value.copy(centre);
      this.hostSpinUniforms.uHostRadius.value = body.radius;
    }
    this.hostSpin.update(on, false, this.morph);
  }

  private drawSkyBodies(): void {
    const visible = this.u.uLightMode.value === 1 && this.morph > 0.01;
    const radiansPerPixel = ((this.camera.fov * Math.PI) / 180) / Math.max(this.renderer.domElement.height, 1);
    const smallest = SKY_BODY_MIN_PX * this.renderer.getPixelRatio() * radiansPerPixel;
    this.skySmallest = smallest;
    for (const entry of this.skyBodies) {
      const { mesh, body } = entry;
      mesh.visible = visible;
      if (!visible) continue;
      // In front of the body shown: drawn after it, over it; else with the
      // sky, behind it.
      const front = this.inFront(body);
      mesh.renderOrder = front ? SKY_FRONT_ORDER : -1;
      mesh.material.depthTest = !front;
      const u = mesh.material.uniforms;
      const place = this.skyPlace(body);
      const dir = (u['uDir']!.value as Vector3).copy(place.direction);
      // North (the spin axis) up on the disk.
      const right = (u['uRight']!.value as Vector3).set(0, 1, 0).cross(dir).negate();
      if (right.lengthSq() < 1e-8) right.set(1, 0, 0);
      right.normalize();
      (u['uUp']!.value as Vector3).copy(right).cross(dir).normalize();
      const tan = Math.tan(Math.min(place.half, 1.4));
      u['uHalf']!.value = Math.max(tan, smallest);
      // A body smaller than its few pixels gets a faint ring, so that it
      // is not taken for a star.
      const tiny = tan < smallest;
      u['uRing']!.value = tiny ? 1 : 0;
      u['uOpacity']!.value = this.morph ** 3;
      const axes = body.axes ?? IDENTITY_AXES;
      (u['uAxisX']!.value as Vector3).set(...axes[0]);
      (u['uAxisY']!.value as Vector3).set(...axes[1]);
      (u['uAxisZ']!.value as Vector3).set(...axes[2]);
      (u['uCloudTint']!.value as Vector3).set(...(body.cloudTint ?? [1, 1, 1]));
      const show = body.show;
      u['uGrid']!.value = show?.grid ? 1 : 0;
      u['uTropics']!.value = show?.tropics ? 1 : 0;
      u['uTilt']!.value = body.axialTiltDeg ?? 0;
      // Clouds loaded once, shown by the switch.
      u['uHasClouds']!.value = entry.maps.some((m) => m === u['uClouds']!.value) && show?.clouds !== false ? 1 : 0;
      // The halo a few pixels high at least, so that it shows on a small disk.
      const halo = show?.atmosphere === false ? null : (body.halo ?? null);
      const height = halo ? Math.max(halo.thickness, (1.5 * smallest) / Math.max(u['uHalf']!.value as number, 1e-9)) : 0;
      (u['uHaloColor']!.value as Vector3).set(...(halo?.colour ?? [0, 0, 0]));
      u['uHaloHeight']!.value = height;
      u['uHaloStrength']!.value = halo?.strength ?? 0;
      u['uSpinAxis']!.value = this.spinAxesOn ? 1 : 0;
      u['uSpinArrow']!.value = this.hostSpinOf?.index === this.skyBodies.indexOf(entry) ? 0 : 1;
      u['uGrow']!.value = Math.max(1 + 3 * height, tiny ? 3 : 1, this.spinAxesOn ? SPIN_AXIS_LENGTH + 0.1 : 1);
      setRingUniforms(u, body.rings ?? null, ringGapLeast(smallest, place.half));
      this.drawSkyRings(entry, place);
    }
    this.drawHostSpin(visible);
  }

  /**
   * A body of the sky's rings, if it has any: over the whole screen, each
   * pixel's ray meeting their plane (see skyRingFragment), drawn after
   * its disk so that they pass in front of it.
   */
  private drawSkyRings(entry: (typeof this.skyBodies)[number], place: { direction: Vector3; half: number }): void {
    const { body, mesh } = entry;
    const rings = body.rings ?? null;
    if (!rings) {
      if (entry.rings) entry.rings.visible = false;
      return;
    }
    if (!entry.rings) {
      const geometry = new BufferGeometry();
      // One triangle covering the screen (see skyRingVertex).
      geometry.setAttribute('position', new BufferAttribute(new Float32Array([-1, -1, 0, 3, -1, 0, -1, 3, 0]), 3));
      entry.rings = new Mesh(
        geometry,
        new ShaderMaterial({
          vertexShader: skyRingVertex,
          fragmentShader: skyRingFragment,
          uniforms: {
            uProjectionInverse: { value: this.camera.projectionMatrixInverse },
            uProjection: { value: this.camera.projectionMatrix },
            uCentre: { value: new Vector3() },
            uRadius: { value: 1 },
            uNormal: { value: new Vector3(0, 1, 0) },
            uColour: { value: new Color() },
            uSunDir: this.u.uSunDir,
            uFade: { value: 1 },
            uRingInner: { value: 0 },
            uRingOuter: { value: 0 },
            uRingOpacity: { value: 0 },
            uRingGaps: { value: Array.from({ length: 4 }, () => new Vector2()) },
            uRingGapCount: { value: 0 },
            uRingGapLeast: { value: 0 },
          },
          transparent: true,
          depthWrite: false,
        }),
      );
      entry.rings.frustumCulled = false;
      this.scene.add(entry.rings);
    }
    const ring = entry.rings;
    ring.visible = mesh.visible;
    // Just after its disk, each pixel at the depth of the point of the
    // rings it shows (see skyRingFragment): round a ring moon they pass in
    // front of the globe and behind it.
    ring.renderOrder = mesh.renderOrder + 0.1;
    const u = ring.material.uniforms;
    setRingUniforms(u, rings, ringGapLeast(this.skySmallest, place.half));
    // Its centre and radius from the camera: from its place when it has
    // one, else at infinity in its direction (one radius per sin(half)).
    if (body.position && body.radius !== undefined) {
      (u['uCentre']!.value as Vector3).set(...body.position).sub(this.camera.position);
      u['uRadius']!.value = body.radius;
    } else {
      (u['uCentre']!.value as Vector3).copy(place.direction).multiplyScalar(1 / Math.max(Math.sin(place.half), 1e-9));
      u['uRadius']!.value = 1;
    }
    (u['uNormal']!.value as Vector3).set(...(body.axes ?? IDENTITY_AXES)[1]);
    (u['uColour']!.value as Color).setRGB(...rings.colour, SRGBColorSpace);
    u['uFade']!.value = mesh.material.uniforms['uOpacity']!.value as number;
  }

  /** The body shown drawn from its preview (see setPreviewBody), null until one is. */
  private previewGlobe: Mesh<SphereGeometry, ShaderMaterial> | null = null;

  /**
   * Draws the body shown from its preview, a lit sphere of the colours of
   * its type, while it has no maps (a body of a system not generated yet);
   * null hides it.
   */
  setPreviewBody(preview: Preview | null): void {
    if (preview && !this.previewGlobe) {
      this.previewGlobe = new Mesh(
        new SphereGeometry(1, 96, 64),
        new ShaderMaterial({
          vertexShader: previewGlobeVertex,
          fragmentShader: previewGlobeFragment,
          uniforms: {
            uColor: { value: new Vector3() },
            uColor2: { value: new Vector3() },
            uColor3: { value: new Vector3() },
            uBands: { value: 0 },
            uLand: { value: 0 },
            uGlow: { value: 0 },
            uOpacity: { value: 1 },
            uSunDir: this.u.uSunDir,
            uLightMode: this.u.uLightMode,
          },
        }),
      );
      this.scene.add(this.previewGlobe);
    }
    // The maps of the body shown before stay hidden under it.
    this.surface.visible = preview === null;
    if (this.previewGlobe) {
      this.previewGlobe.visible = preview !== null;
      if (preview) {
        const u = this.previewGlobe.material.uniforms;
        const c = preview.colours;
        (u['uColor']!.value as Vector3).set(...(c[0] ?? [0.4, 0.4, 0.4]));
        (u['uColor2']!.value as Vector3).set(...(c[1] ?? [0, 0, 0]));
        (u['uColor3']!.value as Vector3).set(...(c[2] ?? [0, 0, 0]));
        u['uBands']!.value = preview.bands;
        u['uLand']!.value = preview.bands === 0 && c.length > 1 ? 1 : 0;
        u['uGlow']!.value = preview.glow ? 1 : 0;
      }
    }
    this.requestRender();
  }

  /**
   * Places and sizes the host star: at the sun's direction, its true
   * apparent diameter in pixels (at least a few), in a glow growing with
   * the light the planet gets. Shown with Sunlight on the globe only.
   */
  private drawHostStar(): void {
    const visible = this.u.uLightMode.value === 1 && this.morph > 0.01;
    this.hostStar.visible = visible;
    if (!visible) {
      return;
    }
    const position = this.hostStar.geometry.getAttribute('position') as BufferAttribute;
    // From its place when it has one, so that it stays at the centre of
    // the planets' orbits wherever the camera goes; else at infinity.
    let diameterDeg = this.host.angularDiameterDeg;
    if (this.starPlace) {
      const d = this.starPlace.position.clone().sub(this.camera.position);
      const distance = Math.max(d.length(), 1e-9);
      position.setXYZ(0, ...d.divideScalar(distance).toArray());
      diameterDeg = (2 * Math.asin(Math.min(this.starPlace.radius / distance, 0.999)) * 180) / Math.PI;
    } else {
      position.setXYZ(0, ...this.u.uSunDir.value.toArray());
    }
    position.needsUpdate = true;
    const pixelsPerDegree = this.renderer.domElement.height / this.camera.fov;
    const ratio = this.renderer.getPixelRatio();
    const disk = Math.max(diameterDeg * pixelsPerDegree, 3 * ratio);
    const glow = Math.min(1.2, 0.55 * this.host.insolation ** 0.25);
    const u = this.hostStar.material.uniforms;
    u['uDisk']!.value = disk;
    u['uSize']!.value = Math.min(disk * 6 + 24 * ratio, 512);
    u['uGlow']!.value = glow;
    u['uOpacity']!.value = this.morph ** 3;
  }

  private draw(): void {
    this.shared.uMorph.value = this.morph;
    // The atmosphere only makes sense around the globe: fade it with the morph.
    const skyVisible = this.atmosphereOn && this.hasSky && this.morph > 0.01;
    for (const pass of this.atmosphere) {
      pass.visible = skyVisible;
    }
    this.atmosphereOpacity.value = this.morph ** 3;
    // Rings belong to the globe, like the atmosphere.
    for (const mesh of this.dustRings) {
      mesh.visible = this.morph > 0.01;
      mesh.material.uniforms['uFade']!.value = this.morph ** 3;
    }
    if (this.rings) {
      this.rings.visible = this.morph > 0.01;
      this.rings.material.uniforms['uFade']!.value = this.morph ** 3;
      this.rings.material.uniforms['uGapPixels']!.value = SKY_BODY_MIN_PX * this.renderer.getPixelRatio();
    }
    // The stars belong to the globe's sky, like the atmosphere.
    this.stars.visible = this.starsOn && this.morph > 0.01;
    this.drawHostStar();
    this.drawSkyBodies();
    // The axis and the guides start from the farthest of the surface: an
    // irregular moon's longest semi-axis, with its relief as displayed.
    const relief = this.shared.uHasData.value > 0.5 ? this.peakAbove * this.reliefLift() : 0;
    const shape = this.shared.uHasShape.value > 0.5 ? this.shapeRadii : null;
    const poles = (shape?.poles ?? 1) + relief;
    this.guides.setSurface(poles, (d) => (shape ? shapeRadiusAt(shape, d) : 1) + relief);
    if (this.spinAxis) {
      this.spinAxis.visible = this.spinAxesOn && this.morph > 0.01;
      this.spinAxis.material.opacity = this.morph ** 3;
      this.spinAxis.scale.setScalar(poles);
    }
    this.guides.update(this.spinAxesOn, this.spinAxesOn && this.hostGuideOn, this.morph);
    for (const orbits of [this.skyOrbits, this.skyOrbitsNear]) {
      if (!orbits) continue;
      orbits.visible = this.skyOrbitsOn && this.morph > 0.01;
      orbits.material.uniforms['uOpacity']!.value = 0.45 * this.morph ** 3;
      orbits.material.uniforms['uScreenPx']!.value = this.renderer.domElement.height;
    }
    for (const belts of [this.skyBelts, this.skyBeltsNear]) {
      if (!belts) continue;
      belts.visible = this.skyBeltsOn && this.morph > 0.01;
      belts.material.uniforms['uOpacity']!.value = 0.3 * this.morph ** 3;
    }
    this.stars.material.uniforms['uOpacity']!.value = this.morph ** 3;
    this.stars.material.uniforms['uPixelRatio']!.value = this.renderer.getPixelRatio();
    const altitude = this.altitude();
    // Clouds fade out as the camera nears the ground, before the detail
    // tiles get sharper than the cloud layer.
    this.u.uCloudsOn.value =
      this.cloudsOn && this.hasClouds ? smoothstepJs(CLOUD_FADE[0], CLOUD_FADE[1], altitude) : 0;
    this.driftClouds();
    this.camera.near = Math.max(altitude * 0.1, 1e-6);
    this.camera.far = altitude + 4;
    this.drawRingRocks();
    this.camera.updateProjectionMatrix();
    this.u.uLightDir.value.copy(HEADLIGHT).applyQuaternion(this.camera.quaternion);
    if (!this.animation) {
      // Tiles are picked for the full resolution, whatever the frame's:
      // moving would otherwise swap them for coarser ones and back.
      this.tiles.update(this.camera, this.morph, this.canvas.clientHeight * fullPixelRatio(window.devicePixelRatio, this.quality));
    }
    this.renderer.render(this.scene, this.camera);
    this.onSkyPositions?.(this.skyScreenPositions());
    this.onAxes(axesOnScreen(this.gizmoFrame()));
  }

  /**
   * Whether the axes gizmo shows the sky's axes on the globe (the camera
   * kept still against the sky while the clock plays) or the body's (the
   * camera turning with it): the axes of the frame the camera stays in.
   */
  setGizmoOnSky(on: boolean): void {
    this.gizmoOnSky = on;
    this.requestRender();
  }

  private gizmoOnSky = false;

  /**
   * The camera's turn as the axes gizmo shows it: on the globe, against the
   * sky (the body's axes turned with the stars, see setStarsTurn) when the
   * camera stays still against it, so that the gizmo stays still while the
   * globe turns; else against the body, or the map.
   */
  private gizmoFrame(): Quaternion {
    if (!this.gizmoOnSky || this.mode !== 'globe' || this.starsTurned === 0) return this.camera.quaternion;
    return new Quaternion().setFromAxisAngle(new Vector3(0, 1, 0), -this.starsTurned).multiply(this.camera.quaternion);
  }

  /** Height of the camera above the surface (globe) or the map plane. */
  private altitude(): number {
    if (this.animation?.follow) {
      return this.morphDistance;
    }
    return this.mode === 'globe'
      ? this.camera.position.length() - 1
      : Math.abs(this.camera.position.z);
  }

  /**
   * On the globe, keeps the camera at least the lowest altitude over the
   * ground: a camera freed by the right button turns round a point other
   * than the centre, which the controls' distance limits do not guard.
   */
  private keepAboveGround(): void {
    if (this.mode !== 'globe' || this.animation?.follow) return;
    const floor = 1 + MIN_ALTITUDE + this.reliefLift();
    if (this.camera.position.length() < floor) this.camera.position.setLength(floor);
  }

  // On the globe each wheel step scales the altitude, so zooming stays
  // smooth from orbit down to the deepest tiles.
  private readonly handleWheel = (event: WheelEvent): void => {
    if (this.mode !== 'globe') {
      return;
    }
    event.preventDefault();
    const relief = this.reliefLift();
    // A camera freed by the right button: the wheel brings it nearer the
    // point it turns round, or takes it further, kept off the ground.
    if (this.controls.target.lengthSq() > 1e-12) {
      const target = this.controls.target;
      const offset = this.camera.position.clone().sub(target);
      offset.setLength(Math.min(this.globeReach, Math.max(1e-6, offset.length() * Math.exp(event.deltaY * 0.0015))));
      this.camera.position.copy(target).add(offset);
      this.keepAboveGround();
      this.camera.lookAt(target);
      this.camera.updateMatrixWorld();
      this.reportCamera();
      this.hoverPending = true;
      this.requestRender();
      return;
    }
    // The point of the globe under the cursor stays under it while zooming.
    const anchor = this.globePointAt(event.clientX, event.clientY);
    const altitude = Math.min(
      Math.max(GLOBE_DISTANCE * 2.5, this.globeReach - 1),
      Math.max(MIN_ALTITUDE + relief, this.altitude() * Math.exp(event.deltaY * 0.0015)),
    );
    this.camera.position.setLength(1 + altitude);
    this.camera.updateMatrixWorld();
    if (!anchor) {
      this.reportCamera();
      this.requestRender();
      return;
    }
    // Turn the camera around the centre until the anchor is back under the
    // cursor (two passes absorb the change of view direction).
    for (let pass = 0; pass < 2; pass += 1) {
      const now = this.globePointAt(event.clientX, event.clientY);
      if (!now) {
        break;
      }
      this.camera.position.applyQuaternion(this.turn.setFromUnitVectors(now, anchor));
      this.camera.lookAt(0, 0, 0);
      this.camera.updateMatrixWorld();
    }
    this.reportCamera();
    this.hoverPending = true;
    this.requestRender();
  };

  private reportCamera(): void {
    if (this.steering && !this.applyingCamera) {
      this.onCameraChange(this.getCamera());
    }
  }

  private readonly handleControlsChange = (): void => {
    this.requestRender();
    this.reportCamera();
  };

  private readonly handlePointerEnter = (): void => {
    this.steering = true;
  };

  private readonly handlePointerLeave = (): void => {
    this.steering = false;
    this.hovering = false;
    this.hoverPending = true;
    this.schedule();
  };

  private readonly handlePointerMove = (event: PointerEvent): void => {
    this.steering = true;
    // While dragging, the point under the pointer is meaningless.
    this.hovering = event.buttons === 0;
    this.hoverAt.set(event.clientX, event.clientY);
    this.hoverPending = true;
    this.schedule();
  };

  /** Point of the planet under a point of the canvas, or null. */
  private pickAt(clientX: number, clientY: number): LonLat | null {
    if (this.animation) {
      return null;
    }
    const { origin, direction } = this.rayAt(clientX, clientY);
    return this.mode === 'globe' ? this.pickGlobe(origin, direction) : this.pickMap(origin, direction);
  }

  /**
   * Ray from the camera through a point of the canvas, in world space. The
   * arrays returned are reused by the next call: hovering calls it every
   * frame the pointer moves.
   */
  private rayAt(clientX: number, clientY: number): { origin: Vec3; direction: Vec3 } {
    const rect = this.canvas.getBoundingClientRect();
    this.ndc.set(((clientX - rect.left) / rect.width) * 2 - 1, -((clientY - rect.top) / rect.height) * 2 + 1);
    this.raycaster.setFromCamera(this.ndc, this.camera);
    this.raycaster.ray.origin.toArray(this.ray.origin);
    this.raycaster.ray.direction.toArray(this.ray.direction);
    return this.ray;
  }

  /** Unit-sphere point under a point of the canvas, or null off the globe. */
  private globePointAt(clientX: number, clientY: number): Vector3 | null {
    const { origin, direction } = this.rayAt(clientX, clientY);
    const t = raySphere(origin, direction, 1);
    if (t === null) {
      return null;
    }
    return new Vector3(...origin).addScaledVector(new Vector3(...direction), t).normalize();
  }

  private readonly handlePointerDown = (event: PointerEvent): void => {
    this.pointerDown = new Vector2(event.clientX, event.clientY);
  };

  private readonly handlePointerUp = (event: PointerEvent): void => {
    const down = this.pointerDown;
    this.pointerDown = null;
    if (!down || down.distanceTo(new Vector2(event.clientX, event.clientY)) > CLICK_TOLERANCE) {
      return;
    }
    // Mid-morph the surface is neither a sphere nor a plane: ignore clicks.
    if (this.animation) {
      return;
    }
    this.onPick(this.pickAt(event.clientX, event.clientY));
  };

  // Picking ignores the relief displacement: at most a few percent of the
  // radius, which shifts the picked point by a fraction of a degree.
  private pickGlobe(origin: Vec3, direction: Vec3): LonLat | null {
    const t = raySphere(origin, direction, 1);
    if (t === null) {
      return null;
    }
    const hit = new Vector3(...origin).addScaledVector(new Vector3(...direction), t);
    return sphereToLonLat(hit.toArray());
  }

  private pickMap(origin: Vec3, direction: Vec3): LonLat | null {
    const t = rayPlaneZ0(origin, direction);
    if (t === null) {
      return null;
    }
    return planeToLonLat(origin[0] + direction[0] * t, origin[1] + direction[1] * t);
  }
}

/**
 * Sets a sky body's colours, and its preview when it has one (a body not
 * generated yet: its bands, its land and caps, its glow).
 */
/**
 * Fills a sky line set's geometry with segments from each point to the
 * next round each loop, dashed where `dashed` says (see skyOrbitVertex);
 * false when there is nothing to draw.
 */
function loopLength(loop: readonly Vec3[]): number {
  return loop.reduce((sum, point, i) => {
    const next = loop[(i + 1) % loop.length]!;
    return sum + Math.hypot(next[0] - point[0], next[1] - point[1], next[2] - point[2]);
  }, 0);
}

function fillSkyLines(
  geometry: BufferGeometry,
  loops: readonly (readonly Vec3[])[],
  dashed: readonly boolean[],
): boolean {
  const count = loops.reduce((sum, loop) => sum + loop.length, 0);
  if (count === 0) return false;
  const directions = new Float32Array(count * 6);
  const along = new Float32Array(count * 2);
  let at = 0;
  loops.forEach((loop, k) => {
    let length = 0;
    loop.forEach((point, i) => {
      const next = loop[(i + 1) % loop.length]!;
      directions.set(point, at * 3);
      directions.set(next, at * 3 + 3);
      const step = Math.hypot(next[0] - point[0], next[1] - point[1], next[2] - point[2]);
      along[at] = dashed[k] ? length : -1;
      along[at + 1] = dashed[k] ? length + step : -1;
      length += step;
      at += 2;
    });
  });
  const old = geometry.getAttribute('point') as BufferAttribute | undefined;
  const oldAlong = geometry.getAttribute('along') as BufferAttribute | undefined;
  if (old && oldAlong && old.array.length === directions.length) {
    (old.array as Float32Array).set(directions);
    old.needsUpdate = true;
    (oldAlong.array as Float32Array).set(along);
    oldAlong.needsUpdate = true;
  } else {
    geometry.setAttribute('point', new BufferAttribute(directions, 3));
    geometry.setAttribute('along', new BufferAttribute(along, 1));
    // three.js needs a position attribute to count the vertices.
    geometry.setAttribute('position', new BufferAttribute(directions, 3));
  }
  return true;
}

function applyPreview(u: Record<string, IUniform>, body: SkyBody): void {
  const preview = body.preview;
  const colours = preview?.colours ?? [];
  (u['uColor']!.value as Vector3).set(...(colours[0] ?? body.colour));
  (u['uColor2']!.value as Vector3).set(...(colours[1] ?? [0, 0, 0]));
  (u['uColor3']!.value as Vector3).set(...(colours[2] ?? [0, 0, 0]));
  u['uPreview']!.value = preview ? 1 : 0;
  u['uBands']!.value = preview?.bands ?? 0;
  u['uLand']!.value = preview && preview.bands === 0 && colours.length > 1 ? 1 : 0;
  u['uGlow']!.value = preview?.glow ? 1 : 0;
}

/**
 * Half-width a gap in a sky body's rings keeps at least (see
 * skyRingDensityGlsl), in its radii: `smallest` radians, the half-size of a
 * small moon's marker, at the distance of a body `half` radians across.
 */
function ringGapLeast(smallest: number, half: number): number {
  return smallest / Math.max(Math.sin(half), 1e-9);
}

/** Sets a sky body's ring uniforms (see skyRingDensityGlsl) from its rings; none clears them. */
function setRingUniforms(u: Record<string, IUniform>, rings: RingSystem | null, gapLeast: number): void {
  u['uRingInner']!.value = rings?.inner ?? 0;
  u['uRingOuter']!.value = rings?.outer ?? 0;
  u['uRingOpacity']!.value = rings?.opacity ?? 0;
  u['uRingGapCount']!.value = Math.min(rings?.gaps.length ?? 0, 4);
  u['uRingGapLeast']!.value = gapLeast;
  (u['uRingGaps']!.value as Vector2[]).forEach((gap, k) => gap.set(...(rings?.gaps[k] ?? [0, 0])));
}

/**
 * The radius of an irregular body's shape along `direction` (its frame:
 * `[cos lat sin lon, sin lat, cos lat cos lon]`), from its map of radii,
 * row 0 north, as the surface shader reads it (shapeAt).
 */
function shapeRadiusAt(shape: { radii: Float32Array; width: number; height: number }, direction: Vector3): number {
  const d = direction.clone().normalize();
  const lon = Math.atan2(d.x, d.z);
  const lat = Math.asin(Math.max(-1, Math.min(1, d.y)));
  const u = (lon / (2 * Math.PI) + 1) % 1;
  const v = lat / Math.PI + 0.5;
  const x = Math.min(shape.width - 1, Math.floor(u * shape.width));
  const y = Math.min(shape.height - 1, Math.max(0, Math.floor((1 - v) * shape.height)));
  return shape.radii[y * shape.width + x] ?? 1;
}

/** The slots of a lattice's window: each row, each cell from -window to window, each rank. */
function latticeSlots(rows: number, window: number, perCell: number): Float32Array {
  const slots = new Float32Array(rows * (2 * window + 1) * perCell * 3);
  let k = 0;
  for (let row = 0; row < rows; row += 1) {
    for (let cell = -window; cell <= window; cell += 1) {
      for (let rank = 0; rank < perCell; rank += 1) {
        slots.set([row, cell, rank], k);
        k += 3;
      }
    }
  }
  return slots;
}

/**
 * Makes `material` draw far away, as the host planet's spin in a moon's
 * sky: its depth held at the camera's far plane (which only bounds the
 * globe's depth), so that the globe still hides it, and its fragments
 * dropped where the planet (`uniforms`: its centre and radius, in the
 * scene's frame) stands between them and the camera.
 */
function farGuide(material: MeshBasicMaterial, uniforms: { uHostCentre: IUniform<Vector3>; uHostRadius: IUniform<number> }): void {
  material.onBeforeCompile = (shader) => {
    Object.assign(shader.uniforms, uniforms);
    shader.vertexShader = `varying vec3 vFarWorld;\n${shader.vertexShader}`.replace(
      '#include <project_vertex>',
      `#include <project_vertex>
      vFarWorld = (modelMatrix * vec4(transformed, 1.0)).xyz;
      gl_Position.z = min(gl_Position.z, gl_Position.w * 0.99999);`,
    );
    shader.fragmentShader = `uniform vec3 uHostCentre;\nuniform float uHostRadius;\nvarying vec3 vFarWorld;\n${shader.fragmentShader}`.replace(
      'void main() {',
      `void main() {
      vec3 toPoint = vFarWorld - cameraPosition;
      float far = length(toPoint);
      vec3 ray = toPoint / far;
      vec3 oc = cameraPosition - uHostCentre;
      float b = dot(oc, ray);
      float h = b * b - dot(oc, oc) + uHostRadius * uHostRadius;
      if (h > 0.0) {
        float hit = -b - sqrt(h);
        if (hit > 0.0 && hit < far - 1e-3 * uHostRadius) discard;
      }`,
    );
  };
}
