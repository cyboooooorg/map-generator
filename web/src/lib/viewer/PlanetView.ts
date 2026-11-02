// three.js view of a world: one mesh that morphs between the flat map and the
// globe in its vertex shader, a graticule drawn the same way, and picking.
// Coordinate conventions are documented in ../geo.ts.
import {
  BufferGeometry,
  Color,
  DoubleSide,
  Float32BufferAttribute,
  LineSegments,
  Mesh,
  MOUSE,
  PerspectiveCamera,
  PlaneGeometry,
  Raycaster,
  Scene,
  ShaderMaterial,
  SRGBColorSpace,
  Texture,
  TextureLoader,
  Vector2,
  Vector3,
  WebGLRenderer,
} from 'three';
import { OrbitControls } from 'three/examples/jsm/controls/OrbitControls.js';
import { type LonLat, planeToLonLat, rayPlaneZ0, raySphere, sphereToLonLat, type Vec3 } from '../geo';

export type ViewMode = 'globe' | 'map';

const MORPH_MS = 1200;
/** Camera distances: the globe has radius 1, the map is 2π × π. */
const GLOBE_DISTANCE = 3.2;
const MAP_DISTANCE = 5.5;
/** Pointer moves larger than this (px) are drags, not clicks. */
const CLICK_TOLERANCE = 5;

// Shared by the surface and the graticule: positions come from the uv
// coordinates, so both morph together. `uLift` raises lines above the surface.
const vertexShader = /* glsl */ `
  uniform float uMorph;
  uniform float uLift;
  varying vec2 vUv;
  varying vec3 vNormal;
  const float PI = 3.141592653589793;
  void main() {
    vUv = uv;
    float lon = uv.x * 2.0 * PI;
    float lat = (uv.y - 0.5) * PI;
    vec3 sphere = vec3(cos(lat) * sin(lon), sin(lat), cos(lat) * cos(lon));
    vec3 plane = vec3((uv.x - 0.5) * 2.0 * PI, (uv.y - 0.5) * PI, 0.0);
    vec3 normal = normalize(mix(vec3(0.0, 0.0, 1.0), sphere, uMorph));
    vec3 position = mix(plane, sphere, uMorph) + normal * uLift;
    vNormal = normalize(normalMatrix * normal);
    gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0);
  }
`;

// Light follows the camera (a headlight slightly above and to the left), and
// only shades the globe: the flat map keeps its colours untouched.
const surfaceFragment = /* glsl */ `
  uniform sampler2D uMap;
  uniform float uMorph;
  varying vec2 vUv;
  varying vec3 vNormal;
  void main() {
    vec4 colour = texture2D(uMap, vUv);
    float light = 0.35 + 0.75 * max(dot(vNormal, normalize(vec3(-0.4, 0.5, 1.0))), 0.0);
    gl_FragColor = vec4(colour.rgb * mix(1.0, light, uMorph), 1.0);
    #include <colorspace_fragment>
  }
`;

const lineFragment = /* glsl */ `
  uniform vec3 uColor;
  uniform float uOpacity;
  void main() {
    gl_FragColor = vec4(uColor, uOpacity);
  }
`;

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

export class PlanetView {
  private readonly renderer: WebGLRenderer;
  private readonly scene = new Scene();
  private readonly camera = new PerspectiveCamera(40, 1, 0.01, 100);
  private readonly controls: OrbitControls;
  private readonly surface: Mesh<PlaneGeometry, ShaderMaterial>;
  private readonly grid: LineSegments<BufferGeometry, ShaderMaterial>;
  private readonly resizeObserver: ResizeObserver;
  private texture: Texture | null = null;
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
      new PlaneGeometry(1, 1, 360, 180),
      new ShaderMaterial({
        vertexShader,
        fragmentShader: surfaceFragment,
        uniforms: { uMap: { value: null }, uMorph: { value: 1 }, uLift: { value: 0 } },
        side: DoubleSide,
      }),
    );
    // The plane geometry provides uv in [0, 1]; positions come from the shader.
    this.surface.frustumCulled = false;
    this.scene.add(this.surface);

    this.grid = new LineSegments(
      graticule(30),
      new ShaderMaterial({
        vertexShader,
        fragmentShader: lineFragment,
        uniforms: {
          uMorph: { value: 1 },
          uLift: { value: 0.002 },
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

    this.controls = new OrbitControls(this.camera, canvas);
    this.controls.enableDamping = true;
    this.applyModeControls();
    this.camera.position.set(0, 0, GLOBE_DISTANCE);

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
    this.surface.material.uniforms['uMap']!.value = texture;
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
    this.canvas.removeEventListener('pointerdown', this.handlePointerDown);
    this.canvas.removeEventListener('pointerup', this.handlePointerUp);
    this.controls.dispose();
    this.texture?.dispose();
    this.surface.geometry.dispose();
    this.surface.material.dispose();
    this.grid.geometry.dispose();
    this.grid.material.dispose();
    this.renderer.dispose();
  }

  private applyModeControls(): void {
    const globe = this.mode === 'globe';
    this.controls.enableRotate = globe;
    this.controls.enablePan = !globe;
    this.controls.screenSpacePanning = true;
    this.controls.minDistance = globe ? 1.15 : 0.3;
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
    this.surface.material.uniforms['uMorph']!.value = this.morph;
    this.grid.material.uniforms['uMorph']!.value = this.morph;
    this.controls.update();
    this.renderer.render(this.scene, this.camera);
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
