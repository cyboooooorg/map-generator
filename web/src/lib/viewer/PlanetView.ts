// three.js view of a world: one mesh morphing between the flat map and the
// globe in its vertex shader, data layers coloured on the GPU, relief, light
// (headlight or sun), atmosphere, graticule, hex overlay and picking.
// Coordinate conventions are documented in ../geo.ts, shaders in ./shaders.ts.
import {
  AdditiveBlending,
  BackSide,
  BufferGeometry,
  ClampToEdgeWrapping,
  Color,
  DataTexture,
  DoubleSide,
  Float32BufferAttribute,
  HalfFloatType,
  LinearFilter,
  LineSegments,
  Mesh,
  MOUSE,
  NearestFilter,
  PerspectiveCamera,
  PlaneGeometry,
  Raycaster,
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
  type LonLat,
  lonLatToSphere,
  planeToLonLat,
  rayPlaneZ0,
  raySphere,
  sphereToLonLat,
  type Vec3,
} from '../geo';
import type { WorldLayers } from '../layers';
import { TileLayer } from './TileLayer';
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
  };
  private atmosphereOpacity = { value: 1 } as IUniform<number>;
  private texture: Texture | null = null;
  private atmosphereOn = true;
  private mode: ViewMode = 'globe';
  private morph = 1;
  private animation: { from: number; to: number; start: number } | null = null;
  private pointerDown: Vector2 | null = null;

  /** Called with the picked point, or null when a click misses the planet. */
  onPick: (point: LonLat | null) => void = () => {};

  constructor(private readonly canvas: HTMLCanvasElement) {
    this.renderer = new WebGLRenderer({ canvas, antialias: true });
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
          side: DoubleSide,
        }),
    );

    this.controls = new OrbitControls(this.camera, canvas);
    this.controls.enableDamping = true;
    this.applyModeControls();
    this.camera.position.set(0, 0, GLOBE_DISTANCE);

    canvas.addEventListener('wheel', this.handleWheel, { passive: false });
    canvas.addEventListener('pointerdown', this.handlePointerDown);
    canvas.addEventListener('pointerup', this.handlePointerUp);
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
   * elevation of the coastline: the world's sea level plus its preset waterline.
   */
  setData(layers: WorldLayers | null, coast: number): void {
    this.shared.uData.value.dispose();
    if (!layers) {
      this.shared.uData.value = emptyTexture();
      this.shared.uHasData.value = 0;
      return;
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

  dispose(): void {
    this.renderer.setAnimationLoop(null);
    this.resizeObserver.disconnect();
    this.canvas.removeEventListener('wheel', this.handleWheel);
    this.canvas.removeEventListener('pointerdown', this.handlePointerDown);
    this.canvas.removeEventListener('pointerup', this.handlePointerUp);
    this.controls.dispose();
    this.tiles.dispose();
    this.texture?.dispose();
    this.shared.uData.value.dispose();
    this.u.uHex.value.dispose();
    for (const object of [this.surface, this.grid, this.atmosphere]) {
      object.geometry.dispose();
      object.material.dispose();
    }
    this.renderer.dispose();
  }

  private applyModeControls(): void {
    const globe = this.mode === 'globe';
    this.controls.enableRotate = globe;
    this.controls.enablePan = !globe;
    // On the globe the wheel is handled here, by altitude (see handleWheel).
    this.controls.enableZoom = !globe;
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
    const relief = this.shared.uRelief.value * RELIEF_SCALE;
    const altitude = Math.min(
      GLOBE_DISTANCE * 2.5,
      Math.max(MIN_ALTITUDE + relief, this.altitude() * Math.exp(event.deltaY * 0.0015)),
    );
    this.camera.position.setLength(1 + altitude);
  };

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
    const rect = this.canvas.getBoundingClientRect();
    const ndc = new Vector2(
      ((event.clientX - rect.left) / rect.width) * 2 - 1,
      -((event.clientY - rect.top) / rect.height) * 2 + 1,
    );
    const raycaster = new Raycaster();
    raycaster.setFromCamera(ndc, this.camera);
    const origin: Vec3 = raycaster.ray.origin.toArray();
    const direction: Vec3 = raycaster.ray.direction.toArray();
    this.onPick(this.mode === 'globe' ? this.pickGlobe(origin, direction) : this.pickMap(origin, direction));
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
