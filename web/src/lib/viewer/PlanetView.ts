// three.js view of a world: one mesh morphing between the flat map and the
// globe in its vertex shader, data layers coloured on the GPU, relief, light
// (headlight or sun), atmosphere, graticule, hex overlay, biome highlight,
// measured paths, picking and hovering, screenshots and camera sharing.
// Coordinate conventions are documented in ../geo.ts, shaders in ./shaders.ts.
import {
  AdditiveBlending,
  BackSide,
  BufferGeometry,
  ClampToEdgeWrapping,
  Color,
  DataTexture,
  DoubleSide,
  FrontSide,
  Float32BufferAttribute,
  HalfFloatType,
  LinearFilter,
  LineSegments,
  Mesh,
  MOUSE,
  NotEqualStencilFunc,
  NearestFilter,
  PerspectiveCamera,
  PlaneGeometry,
  Quaternion,
  Raycaster,
  RedFormat,
  RepeatWrapping,
  RGBAFormat,
  Scene,
  ShaderMaterial,
  SphereGeometry,
  SRGBColorSpace,
  Texture,
  TextureLoader,
  UnsignedByteType,
  Vector2,
  Vector3,
  Vector4,
  WebGLRenderer,
  type IUniform,
} from 'three';
import { OrbitControls } from 'three/examples/jsm/controls/OrbitControls.js';
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
import type { WorldLayers } from '../layers';
import { DETAIL_STENCIL, TileLayer } from './TileLayer';
import {
  atmosphereFragment,
  atmosphereVertex,
  LAYER_IDS,
  type Layer,
  lineFragment,
  RELIEF_SCALE,
  surfaceFragment,
  surfaceVertex,
} from './shaders';

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
    };

const MORPH_MS = 1200;
/** Camera distances: the globe has radius 1, the map is 2π × π. */
const GLOBE_DISTANCE = 3.2;
const MAP_DISTANCE = 5.5;
/** Pointer moves larger than this (px) are drags, not clicks. */
const CLICK_TOLERANCE = 5;
/** Headlight direction in view space: above and to the left of the camera. */
const HEADLIGHT = new Vector3(-0.4, 0.5, 1).normalize();
/** Lowest camera altitude over the globe (radius 1): about 2 km on an Earth-sized planet. */
const MIN_ALTITUDE = 0.0003;

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

function emptyTexture(): DataTexture {
  const texture = new DataTexture(new Uint8Array(4), 1, 1, RGBAFormat, UnsignedByteType);
  texture.needsUpdate = true;
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

export class PlanetView {
  private readonly renderer: WebGLRenderer;
  private readonly scene = new Scene();
  private readonly camera = new PerspectiveCamera(40, 1, 0.01, 100);
  private readonly controls: OrbitControls;
  private readonly surface: Mesh<PlaneGeometry, ShaderMaterial>;
  private readonly grid: LineSegments<BufferGeometry, ShaderMaterial>;
  private readonly atmosphere: Mesh<SphereGeometry, ShaderMaterial>;
  private readonly resizeObserver: ResizeObserver;
  private readonly tiles: TileLayer;
  private layer: Layer = 'biomes';
  private hexOn = false;
  /** Uniforms shared by the surface and the graticule (the same objects). */
  private readonly shared = {
    uMorph: { value: 1 } as IUniform<number>,
    uRelief: { value: 0 } as IUniform<number>,
    uCoast: { value: 0 } as IUniform<number>,
    uLiquidSea: { value: 1 } as IUniform<number>,
    uHasData: { value: 0 } as IUniform<number>,
    uData: { value: emptyTexture() } as IUniform<Texture>,
  };
  /** Surface-only uniforms. */
  private readonly u = {
    uLift: { value: 0 } as IUniform<number>,
    uMap: { value: emptyTexture() } as IUniform<Texture>,
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
    uTileOn: { value: 0 } as IUniform<number>,
    uTile: { value: emptyTexture() } as IUniform<Texture>,
    uTexRect: { value: new Vector4(0, 0, 1, 1) } as IUniform<Vector4>,
    uHasExtra: { value: 0 } as IUniform<number>,
    uExtra: { value: emptyTexture() } as IUniform<Texture>,
    uCoastMax: { value: 2000 } as IUniform<number>,
    uHasBiomes: { value: 0 } as IUniform<number>,
    uBiomes: { value: emptyTexture() } as IUniform<Texture>,
    uHighlight: { value: -1 } as IUniform<number>,
  };
  private readonly path: LineSegments<BufferGeometry, ShaderMaterial>;
  private atmosphereOpacity = { value: 1 } as IUniform<number>;
  private texture: Texture | null = null;
  private atmosphereOn = true;
  private mode: ViewMode = 'globe';
  private morph = 1;
  private animation: { from: number; to: number; start: number } | null = null;
  private pointerDown: Vector2 | null = null;
  /** Last pointer position over the canvas, hovered on the next frame. */
  private hoverAt: Vector2 | null = null;
  private hoverPending = false;
  /** Whether the user is steering this view (pointer over it): only then it reports its camera. */
  private steering = false;
  private applyingCamera = false;

  /** Called with the picked point, or null when a click misses the planet. */
  onPick: (point: LonLat | null) => void = () => {};
  /** Called with the point under the pointer (at most once a frame), or null off the planet. */
  onHover: (point: LonLat | null) => void = () => {};
  /** Called when the user moves the camera of this view. */
  onCameraChange: (state: CameraState) => void = () => {};

  constructor(private readonly canvas: HTMLCanvasElement) {
    // The stencil lets detail patches hide the base surface (see TileLayer).
    this.renderer = new WebGLRenderer({ canvas, antialias: true, stencil: true });
    this.renderer.setPixelRatio(window.devicePixelRatio);
    this.scene.background = new Color('#05070d');

    this.surface = new Mesh(
      // 2 vertices per degree: enough for the relief to follow the data.
      new PlaneGeometry(1, 1, 720, 360),
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
    this.scene.add(this.grid);

    this.atmosphere = new Mesh(
      new SphereGeometry(1.06, 96, 48),
      new ShaderMaterial({
        vertexShader: atmosphereVertex,
        fragmentShader: atmosphereFragment,
        uniforms: {
          uOpacity: this.atmosphereOpacity,
          uColor: { value: new Color('#5c9dff') },
          uLightMode: this.u.uLightMode,
          uSunDir: this.u.uSunDir,
        },
        side: BackSide,
        blending: AdditiveBlending,
        transparent: true,
        depthWrite: false,
      }),
    );
    this.scene.add(this.atmosphere);

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
            uTileOn: { value: 0 },
            uTile: { value: null },
            uTexRect: { value: new Vector4(0, 0, 1, 1) },
          },
          // Front faces only: with DoubleSide, the far side of a patch
          // would mark the stencil and punch a hole in the near globe.
          side: FrontSide,
        }),
    );

    this.controls = new OrbitControls(this.camera, canvas);
    this.controls.enableDamping = true;
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
    this.renderer.setAnimationLoop(this.render);
  }

  /** Shows a world map (an equirectangular image URL). */
  async setTexture(url: string): Promise<void> {
    const texture = await new TextureLoader().loadAsync(url);
    texture.colorSpace = SRGBColorSpace;
    texture.anisotropy = this.renderer.capabilities.getMaxAnisotropy();
    this.texture?.dispose();
    this.texture = texture;
    this.u.uMap.value = texture;
  }

  /**
   * Uploads the decoded layers, or clears them with null. `coast` is the
   * elevation of the coastline: the world's sea level plus its preset
   * waterline. `liquidSea` is false on worlds whose seas have dried up: their
   * basins are then shaded and raised like land, without sun glint.
   */
  setData(layers: WorldLayers | null, coast: number, liquidSea = true): void {
    this.shared.uData.value.dispose();
    this.u.uExtra.value.dispose();
    this.u.uBiomes.value.dispose();
    this.u.uExtra.value = emptyTexture();
    this.u.uBiomes.value = emptyTexture();
    this.u.uHasExtra.value = 0;
    this.u.uHasBiomes.value = 0;
    if (!layers) {
      this.shared.uData.value = emptyTexture();
      this.shared.uHasData.value = 0;
      return;
    }
    const biomes = new DataTexture(layers.biomes, layers.width, layers.height, RedFormat, UnsignedByteType);
    // Rows of an odd width are not 4-byte aligned.
    biomes.unpackAlignment = 1;
    biomes.magFilter = NearestFilter;
    biomes.minFilter = NearestFilter;
    biomes.wrapS = RepeatWrapping;
    biomes.wrapT = ClampToEdgeWrapping;
    biomes.generateMipmaps = false;
    biomes.needsUpdate = true;
    this.u.uBiomes.value = biomes;
    this.u.uHasBiomes.value = 1;
    if (layers.extra) {
      const extra = new DataTexture(layers.extra.packed, layers.width, layers.height, RGBAFormat, HalfFloatType);
      extra.magFilter = LinearFilter;
      extra.minFilter = LinearFilter;
      extra.wrapS = RepeatWrapping;
      extra.wrapT = ClampToEdgeWrapping;
      extra.generateMipmaps = false;
      extra.needsUpdate = true;
      this.u.uExtra.value = extra;
      this.u.uHasExtra.value = 1;
      this.u.uCoastMax.value = layers.extra.coastMaxKm;
    }
    const texture = new DataTexture(layers.surface, layers.width, layers.height, RGBAFormat, HalfFloatType);
    texture.magFilter = LinearFilter;
    texture.minFilter = LinearFilter;
    texture.wrapS = RepeatWrapping;
    texture.wrapT = ClampToEdgeWrapping;
    texture.generateMipmaps = false;
    texture.needsUpdate = true;
    this.shared.uData.value = texture;
    this.shared.uHasData.value = 1;
    this.shared.uCoast.value = coast;
    this.shared.uLiquidSea.value = liquidSea ? 1 : 0;
    this.u.uTexel.value.set(1 / layers.width, 1 / layers.height);
    this.u.uMapSize.value.set(layers.width, layers.height);
  }

  setLayer(layer: Layer): void {
    this.u.uLayer.value = LAYER_IDS[layer];
    this.layer = layer;
    this.tiles.setEnabled(layer === 'biomes' && !this.hexOn);
  }

  /**
   * Streams detail tiles of a world when zooming in (`revision` keys the
   * browser cache), or stops with null.
   */
  setDetail(world: { id: string; revision: number; baseWidth: number } | null): void {
    this.tiles.setWorld(world);
  }

  /** Relief exaggeration in [0, 1]: 0 is flat. */
  setRelief(relief: number): void {
    this.shared.uRelief.value = relief;
  }

  setLighting(lighting: Lighting): void {
    this.u.uLightMode.value = lighting.kind === 'sun' ? 1 : 0;
    if (lighting.kind === 'sun') {
      const declination = ((lighting.axialTiltDeg * Math.PI) / 180) * Math.sin(2 * Math.PI * lighting.season);
      const [x, y, z] = lonLatToSphere({ lon: (lighting.longitudeDeg * Math.PI) / 180, lat: declination });
      this.u.uSunDir.value.set(x, y, z);
    }
  }

  setAtmosphere(visible: boolean): void {
    this.atmosphereOn = visible;
  }

  /** Shows a hex grid over the map (majority biome per hex), or hides it with null. */
  setHex(map: HexMap | null): void {
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
    if (mode === this.mode) {
      return;
    }
    this.mode = mode;
    this.animation = { from: this.morph, to: mode === 'globe' ? 1 : 0, start: performance.now() };
    this.controls.target.set(0, 0, 0);
    this.camera.position.set(0, 0, mode === 'globe' ? GLOBE_DISTANCE : MAP_DISTANCE);
    this.camera.up.set(0, 1, 0);
    this.applyModeControls();
  }

  setGraticule(visible: boolean): void {
    this.grid.visible = visible;
  }

  /** Highlights one biome (id), fading the others; null shows them all. */
  setHighlight(biome: number | null): void {
    this.u.uHighlight.value = biome ?? -1;
  }

  /** Draws a path of great-circle legs through the points (fewer than one hides it). */
  setPath(points: readonly LonLat[]): void {
    this.path.geometry.dispose();
    this.path.geometry = pathGeometry(points);
    this.path.visible = points.length > 0;
  }

  /** Back to the starting view of the current mode: whole planet, north up. */
  resetView(): void {
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

  /** Moves the camera to a state reported by another view (same mode expected). */
  setCamera(state: CameraState): void {
    this.applyingCamera = true;
    if (state.mode !== this.mode) {
      this.setMode(state.mode);
      // Skip the morph: the other view has finished it already.
      this.animation = null;
      this.morph = state.mode === 'globe' ? 1 : 0;
    }
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
    this.renderer.setAnimationLoop(null);
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
    this.texture?.dispose();
    this.shared.uData.value.dispose();
    this.u.uHex.value.dispose();
    this.u.uExtra.value.dispose();
    this.u.uBiomes.value.dispose();
    for (const object of [this.surface, this.grid, this.atmosphere, this.path]) {
      object.geometry.dispose();
      object.material.dispose();
    }
    this.renderer.dispose();
  }

  private applyModeControls(): void {
    const globe = this.mode === 'globe';
    this.controls.enableRotate = globe;
    this.controls.enablePan = !globe;
    // On the globe the wheel is handled here, by altitude (see handleWheel);
    // on the map, OrbitControls zooms towards the cursor.
    this.controls.enableZoom = !globe;
    this.controls.zoomToCursor = !globe;
    this.controls.screenSpacePanning = true;
    this.controls.minDistance = globe ? 1 + MIN_ALTITUDE : 0.002;
    this.controls.maxDistance = globe ? 8 : 12;
    this.controls.mouseButtons = globe
      ? { LEFT: MOUSE.ROTATE, MIDDLE: MOUSE.DOLLY, RIGHT: MOUSE.ROTATE }
      : { LEFT: MOUSE.PAN, MIDDLE: MOUSE.DOLLY, RIGHT: MOUSE.PAN };
  }

  private resize(): void {
    const { clientWidth, clientHeight } = this.canvas;
    if (clientWidth === 0 || clientHeight === 0) {
      return;
    }
    this.renderer.setSize(clientWidth, clientHeight, false);
    this.camera.aspect = clientWidth / clientHeight;
    // In portrait views, zoom out so the planet fits the width as well.
    this.camera.zoom = Math.min(1, this.camera.aspect);
    this.camera.updateProjectionMatrix();
  }

  private readonly render = (now: number): void => {
    if (this.animation) {
      const t = Math.min(1, (now - this.animation.start) / MORPH_MS);
      const eased = t < 0.5 ? 4 * t * t * t : 1 - (-2 * t + 2) ** 3 / 2;
      this.morph = this.animation.from + (this.animation.to - this.animation.from) * eased;
      if (t === 1) {
        this.animation = null;
      }
    }
    this.shared.uMorph.value = this.morph;
    // The atmosphere only makes sense around the globe: fade it with the morph.
    this.atmosphere.visible = this.atmosphereOn && this.morph > 0.01;
    this.atmosphereOpacity.value = this.morph ** 3;
    // Near the surface: slower rotation, and a near plane that follows the
    // altitude so depth precision holds down to a few hundred metres.
    const altitude = this.altitude();
    this.controls.rotateSpeed = Math.min(1, Math.max(0.002, altitude * 0.6));
    this.camera.near = Math.max(altitude * 0.1, 1e-6);
    this.camera.far = altitude + 4;
    this.camera.updateProjectionMatrix();
    this.controls.update();
    this.u.uLightDir.value.copy(HEADLIGHT).applyQuaternion(this.camera.quaternion);
    if (!this.animation) {
      this.tiles.update(this.camera, this.morph, this.renderer.domElement.height);
    }
    this.renderer.render(this.scene, this.camera);
    if (this.hoverPending) {
      this.hoverPending = false;
      this.onHover(this.hoverAt ? this.pickAt(this.hoverAt.x, this.hoverAt.y) : null);
    }
  };

  /** Height of the camera above the surface (globe) or the map plane. */
  private altitude(): number {
    return this.mode === 'globe'
      ? this.camera.position.length() - 1
      : Math.abs(this.camera.position.z);
  }

  // On the globe each wheel step scales the altitude, so zooming stays
  // smooth from orbit down to the deepest tiles.
  private readonly handleWheel = (event: WheelEvent): void => {
    if (this.mode !== 'globe') {
      return;
    }
    event.preventDefault();
    // The point of the globe under the cursor stays under it while zooming.
    const anchor = this.globePointAt(event.clientX, event.clientY);
    const relief = this.shared.uRelief.value * RELIEF_SCALE;
    const altitude = Math.min(
      GLOBE_DISTANCE * 2.5,
      Math.max(MIN_ALTITUDE + relief, this.altitude() * Math.exp(event.deltaY * 0.0015)),
    );
    this.camera.position.setLength(1 + altitude);
    this.camera.updateMatrixWorld();
    if (!anchor) {
      this.reportCamera();
      return;
    }
    // Turn the camera around the centre until the anchor is back under the
    // cursor (two passes absorb the change of view direction).
    for (let pass = 0; pass < 2; pass += 1) {
      const now = this.globePointAt(event.clientX, event.clientY);
      if (!now) {
        break;
      }
      this.camera.position.applyQuaternion(new Quaternion().setFromUnitVectors(now, anchor));
      this.camera.lookAt(0, 0, 0);
      this.camera.updateMatrixWorld();
    }
    this.reportCamera();
    this.hoverPending = true;
  };

  private reportCamera(): void {
    if (this.steering && !this.applyingCamera) {
      this.onCameraChange(this.getCamera());
    }
  }

  private readonly handleControlsChange = (): void => {
    this.reportCamera();
  };

  private readonly handlePointerEnter = (): void => {
    this.steering = true;
  };

  private readonly handlePointerLeave = (): void => {
    this.steering = false;
    this.hoverAt = null;
    this.hoverPending = true;
  };

  private readonly handlePointerMove = (event: PointerEvent): void => {
    this.steering = true;
    // While dragging, the point under the pointer is meaningless.
    this.hoverAt = event.buttons === 0 ? new Vector2(event.clientX, event.clientY) : null;
    this.hoverPending = true;
  };

  /** Point of the planet under a point of the canvas, or null. */
  private pickAt(clientX: number, clientY: number): LonLat | null {
    if (this.animation) {
      return null;
    }
    const { origin, direction } = this.rayAt(clientX, clientY);
    return this.mode === 'globe' ? this.pickGlobe(origin, direction) : this.pickMap(origin, direction);
  }

  /** Ray from the camera through a point of the canvas, in world space. */
  private rayAt(clientX: number, clientY: number): { origin: Vec3; direction: Vec3 } {
    const rect = this.canvas.getBoundingClientRect();
    const ndc = new Vector2(
      ((clientX - rect.left) / rect.width) * 2 - 1,
      -((clientY - rect.top) / rect.height) * 2 + 1,
    );
    const raycaster = new Raycaster();
    raycaster.setFromCamera(ndc, this.camera);
    return { origin: raycaster.ray.origin.toArray(), direction: raycaster.ray.direction.toArray() };
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
