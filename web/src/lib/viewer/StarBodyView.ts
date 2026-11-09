// The star view: a system's star alone, its spin axis up, close enough to
// see its granules (down to 1.02 star radii from its centre), with its
// coronal loops and corona in front of the star field. The system view is
// StarView; this one shares its surface (starSurface.ts), its loops
// (coronalLoops.ts) and its clock, in Earth days from the system's t = 0.
//
// Drawn on demand; with the animation on, thirty frames a second at most
// (fewer as the quality profile caps them). Nothing runs while the view is
// still and not animated: no requestAnimationFrame is left pending.
import {
  AdditiveBlending,
  BufferAttribute,
  BufferGeometry,
  Color,
  Group,
  MOUSE,
  Mesh,
  PerspectiveCamera,
  PlaneGeometry,
  Points,
  Raycaster,
  Scene,
  ShaderMaterial,
  SphereGeometry,
  Vector2,
  Vector3,
  WebGLRenderer,
} from 'three';
import { OrbitControls } from 'three/examples/jsm/controls/OrbitControls.js';
import { CameraSettling } from './cameraSettling';
import type { StarActivity } from '../api/types/StarActivity';
import type { StarInfo } from '../api/types/StarInfo';
import { CoronalLoops } from './coronalLoops';
import { Eruption, eruptive } from './eruption';
import { FrameStats } from './frameStats';
import { fullPixelRatio } from './pixelRatio';
import { PROFILES, type QualityProfile } from './quality';
import { starsFragment, starsVertex } from './shaders';
import {
  glowFragment,
  glowVertex,
  prominenceFragment,
  prominenceVertex,
  starSurfaceFragment,
  starSurfaceVertex,
} from './starShaders';
import { StarSurfaceState, starLook, starSurfaceUniforms } from './starSurface';
import { STAR_RADIUS } from './starSystem';
import { starField } from './stars';

/** Closest and farthest camera, star radii from the centre: close enough for granules, far enough for the corona. */
const MIN_DISTANCE = 1.02;
const MAX_DISTANCE = 12;
/** The camera's elevation above the star's equator when the view opens or is reset, radians. */
const START_ELEVATION = (20 * Math.PI) / 180;
/** Share of the half field of view (on the narrower side) the star's disc fills when the view opens. */
const START_FILL = 0.9;
/** Shortest time between two animated frames, ms (thirty frames a second at most). */
const FRAME_MS = 1000 / 30;
/** Longest step of the animation clock, s: a stall pauses the motion instead of making it jump. */
const MAX_STEP_S = 0.1;
/** Width of the corona's quad, in radii of the star's apparent disc. */
const GLOW_SIZE = 7;
/** The red of hydrogen (H-alpha), linear RGB. */
const H_ALPHA: [number, number, number] = [1, 0.2, 0.14];
/** Least height of the camera above the surface the detail is measured from, star radii. */
const MIN_ALTITUDE = 1e-3;

/**
 * Pixels per star radius at the point of the surface nearest the camera,
 * `distance` star radii from the centre, in a view `heightPx` CSS px tall
 * with a vertical field of view of `fovDeg`: that point lies `distance − 1`
 * radii away. The star view's detail (granules from 1.5 px each, see
 * surfaceDetail) follows it, so that up close, where the surface fills the
 * view, granules show at full contrast; the system view measures from the
 * centre instead, the same far away.
 */
export function surfaceRadiusPx(distance: number, fovDeg: number, heightPx: number): number {
  if (!(heightPx > 0)) return 0;
  const halfFov = ((fovDeg / 2) * Math.PI) / 180;
  return (1 / (Math.max(distance - 1, MIN_ALTITUDE) * Math.tan(halfFov))) * (heightPx / 2);
}

export class StarBodyView {
  private readonly renderer: WebGLRenderer;
  /** Frame-time probe (`?frames=1`); a plain draw call otherwise. */
  private readonly frames = new FrameStats('star');
  private readonly scene = new Scene();
  private readonly camera = new PerspectiveCamera(40, 1, 0.002, 200);
  private readonly controls: OrbitControls;
  private readonly settling = new CameraSettling();
  private readonly star: Mesh<SphereGeometry, ShaderMaterial>;
  private readonly surface: StarSurfaceState;
  private readonly glow: Mesh<PlaneGeometry, ShaderMaterial>;
  private readonly sky: Points<BufferGeometry, ShaderMaterial>;
  private readonly arcs = new Group();
  private loops!: CoronalLoops;
  private readonly eruption: Eruption;
  private klass = 'G';
  private seed = 0;
  private readonly resizeObserver: ResizeObserver;
  /** The pending on-demand frame, and the animation loop: both null when the view is idle. */
  private frame: number | null = null;
  private loop: number | null = null;
  private lastTick = 0;
  private lastDrawn = -Infinity;
  private disposed = false;
  /** The day shown, Earth days from the system's t = 0 (any finite number). */
  private days = 0;
  /** How fast the clock runs, Earth days per second of animation. */
  private daysPerSecond = 1;
  /** Seconds of animation, for the loops' shimmer and the eruptions (not the surface). */
  private time = 0;
  private hasActivity = false;
  /** Called with the day on each animated frame and after setDay. */
  onClock: (days: number) => void = () => {};
  /** Called on a double click on the star; null ignores it. */
  onOpenSystem: (() => void) | null = null;

  /** Least time between frames: the animation's 30 per second, or fewer as the profile caps them. */
  private get frameMs(): number {
    return Math.max(FRAME_MS, this.quality.maxFps !== null ? 1000 / this.quality.maxFps : 0);
  }

  constructor(
    private readonly canvas: HTMLCanvasElement,
    readonly quality: QualityProfile = PROFILES.high,
  ) {
    this.renderer = new WebGLRenderer({ canvas, antialias: quality.antialias });
    this.renderer.setPixelRatio(fullPixelRatio(window.devicePixelRatio, quality));
    this.scene.background = new Color('#05070d');
    this.controls = new OrbitControls(this.camera, canvas);
    this.controls.enableDamping = quality.damping;
    // The star stays in the middle: no panning, the camera turns round it.
    this.controls.enablePan = false;
    this.controls.mouseButtons = { LEFT: MOUSE.ROTATE, MIDDLE: MOUSE.DOLLY, RIGHT: MOUSE.ROTATE };
    this.controls.minDistance = MIN_DISTANCE * STAR_RADIUS;
    this.controls.maxDistance = MAX_DISTANCE * STAR_RADIUS;
    this.controls.addEventListener('change', () => this.requestRender());
    // Only frames that follow a drag or a wheel advance the damping (see CameraSettling).
    this.controls.addEventListener('change', this.settling.mark);
    this.controls.addEventListener('start', this.settling.mark);
    canvas.addEventListener('contextmenu', (event) => event.preventDefault());
    canvas.addEventListener('dblclick', (event) => {
      if (this.onOpenSystem && this.hitsStar(event.clientX, event.clientY)) this.onOpenSystem();
    });

    const field = starField();
    const skyGeometry = new BufferGeometry();
    skyGeometry.setAttribute('position', new BufferAttribute(field.directions, 3));
    skyGeometry.setAttribute('colour', new BufferAttribute(field.colours, 3));
    skyGeometry.setAttribute('size', new BufferAttribute(field.sizes, 1));
    this.sky = new Points(
      skyGeometry,
      new ShaderMaterial({
        vertexShader: starsVertex,
        fragmentShader: starsFragment,
        uniforms: { uPixelRatio: { value: this.renderer.getPixelRatio() }, uOpacity: { value: 1 } },
        blending: AdditiveBlending,
        transparent: true,
        depthWrite: false,
      }),
    );
    this.sky.frustumCulled = false;
    this.sky.renderOrder = -1;
    this.scene.add(this.sky);

    // The photosphere: μ from the world normal and the direction to the
    // camera (starSurfaceFragment), which keeps the limb right this close.
    this.star = new Mesh(
      new SphereGeometry(STAR_RADIUS, quality.starSegments, quality.starSegments / 2),
      new ShaderMaterial({
        defines: { STAR_REGIONS: quality.starRegions },
        vertexShader: starSurfaceVertex,
        fragmentShader: starSurfaceFragment,
        uniforms: starSurfaceUniforms(quality.starRegions),
      }),
    );
    this.surface = new StarSurfaceState(this.star.material.uniforms, quality.starRegions);
    this.scene.add(this.star);

    // The corona: the system view's glow, on a quad facing the camera,
    // scaled to the disc as it appears (see render).
    this.glow = new Mesh(
      new PlaneGeometry(GLOW_SIZE * STAR_RADIUS, GLOW_SIZE * STAR_RADIUS),
      new ShaderMaterial({
        vertexShader: glowVertex,
        fragmentShader: glowFragment,
        uniforms: {
          uColor: { value: new Vector3(1, 1, 1) },
          uSize: { value: GLOW_SIZE },
          uStrength: { value: 1 },
        },
        blending: AdditiveBlending,
        transparent: true,
        depthWrite: false,
      }),
    );
    this.glow.renderOrder = 1;
    this.scene.add(this.glow);
    this.scene.add(this.arcs);
    this.eruption = new Eruption(this.arcs, quality.prominenceStrands, (seed) => this.loopMaterial(seed));
    this.buildLoops();

    this.resizeObserver = new ResizeObserver(() => this.resize());
    this.resizeObserver.observe(canvas);
    this.resize();
    this.resetView();
  }

  /**
   * Shows a star: `activity` is its living surface (spots, loops, the
   * cycle); without one it shows its limb and granules only.
   */
  setStar(info: StarInfo, activity: StarActivity | null): void {
    (this.glow.material.uniforms['uColor']!.value as Vector3).set(...info.light);
    this.glow.material.uniforms['uStrength']!.value = Math.min(1.4, 0.6 + 0.12 * Math.log10(info.luminosity + 1e-4) + 0.5);
    this.surface.setStar(starLook(info, activity), activity);
    this.hasActivity = activity !== null && activity.envelope !== 'radiative';
    this.klass = info.type[0] ?? 'G';
    this.seed = Math.round(info.teff_k * 7 + info.orbit_au * 1000);
    this.buildLoops();
    this.updateSurface();
    this.requestRender();
  }

  /** The day shown, Earth days from the system's t = 0. */
  get day(): number {
    return this.days;
  }

  /** Moves the clock to `days` (any finite day, negative included); the surface is the same however it got there. */
  setDay(days: number): void {
    if (!Number.isFinite(days)) return;
    this.days = days;
    this.updateSurface();
    this.onClock(this.days);
    this.requestRender();
  }

  /** How fast the clock runs, Earth days per second of animation. */
  setSpeed(daysPerSecond: number): void {
    if (!Number.isFinite(daysPerSecond)) return;
    this.daysPerSecond = daysPerSecond;
    // The fades depend on the speed (a long exposure): draw them again.
    this.updateSurface();
    this.requestRender();
  }

  /** Runs the clock (thirty frames a second at most), or stops it: stopped, no frame is drawn until something changes. */
  setAnimated(on: boolean): void {
    if (on && this.loop === null && !this.disposed) {
      this.lastTick = performance.now();
      this.loop = requestAnimationFrame(this.tick);
    } else if (!on && this.loop !== null) {
      cancelAnimationFrame(this.loop);
      this.loop = null;
      // The eruption is an animation: paused, none hangs half-risen.
      this.eruption.remove();
    }
  }

  /** Shows or hides the coronal loops (and the eruptions). */
  setLoops(on: boolean): void {
    this.arcs.visible = on;
    this.loops.update(on ? this.surface.loopRegions : [], this.days, this.surface.loopFade);
    this.eruption.follow(this.loops);
    this.requestRender();
  }

  /** Back to the starting view: the whole star, 20° above its equator. */
  resetView(): void {
    const halfFov = ((this.camera.fov / 2) * Math.PI) / 180;
    // The narrower side's half field of view.
    const half = Math.atan(Math.tan(halfFov) * Math.min(1, this.camera.aspect));
    const distance = Math.min(MAX_DISTANCE, Math.max(MIN_DISTANCE, 1 / Math.sin(half * START_FILL))) * STAR_RADIUS;
    this.camera.position.set(0, distance * Math.sin(START_ELEVATION), distance * Math.cos(START_ELEVATION));
    this.controls.target.set(0, 0, 0);
    this.controls.update();
    this.requestRender();
  }

  dispose(): void {
    this.disposed = true;
    this.setAnimated(false);
    if (this.frame !== null) cancelAnimationFrame(this.frame);
    this.frame = null;
    this.frames.dispose();
    this.eruption.remove();
    this.loops.dispose();
    this.resizeObserver.disconnect();
    this.controls.dispose();
    for (const object of [this.star, this.glow, this.sky]) {
      object.geometry.dispose();
      object.material.dispose();
    }
    this.renderer.dispose();
    this.renderer.forceContextLoss();
  }

  private readonly tick = (now: number): void => {
    this.loop = requestAnimationFrame(this.tick);
    const elapsed = now - this.lastTick;
    if (elapsed < this.frameMs - 4) return;
    this.lastTick = now;
    const step = Math.min(elapsed / 1000, MAX_STEP_S);
    this.time += step;
    this.days += step * this.daysPerSecond;
    this.updateSurface();
    this.loops.setTime(this.time);
    this.eruption.animate(this.loops, this.time, this.hasActivity && eruptive(this.klass), this.seed, this.surface.loopFade);
    this.onClock(this.days);
    this.render(now);
  };

  /** Pixels per star radius at the surface's nearest point (see surfaceRadiusPx). */
  private radiusPx(): number {
    return surfaceRadiusPx(this.camera.position.length() / STAR_RADIUS, this.camera.fov, this.canvas.clientHeight);
  }

  /** Brings the surface and the loops to the day, with the detail the star's size on screen allows. */
  private updateSurface(): void {
    this.surface.update(this.days, { radiusPx: this.radiusPx(), daysPerSecond: this.daysPerSecond });
    this.loops.update(this.arcs.visible ? this.surface.loopRegions : [], this.days, this.surface.loopFade);
    // Whoever moved the clock (the tick, setDay, a zoom), the eruption stays on its region.
    this.eruption.follow(this.loops);
  }

  /** Whether the ray through a point of the screen (client px) meets the star. */
  private hitsStar(clientX: number, clientY: number): boolean {
    const rect = this.canvas.getBoundingClientRect();
    if (rect.width === 0 || rect.height === 0) return false;
    const ray = new Raycaster();
    ray.setFromCamera(
      new Vector2(((clientX - rect.left) / rect.width) * 2 - 1, -((clientY - rect.top) / rect.height) * 2 + 1),
      this.camera,
    );
    return ray.intersectObject(this.star, false).length > 0;
  }

  private buildLoops(): void {
    this.eruption.reset();
    if (this.loops) {
      this.arcs.remove(this.loops.group);
      this.loops.dispose();
    }
    this.loops = new CoronalLoops(this.quality, (index) => this.loopMaterial(index));
    this.arcs.add(this.loops.group);
  }

  /** The material of a loop (and of an eruption): glowing H-alpha strands. */
  private loopMaterial(index: number): ShaderMaterial {
    return new ShaderMaterial({
      vertexShader: prominenceVertex,
      fragmentShader: prominenceFragment,
      uniforms: {
        uColor: { value: new Vector3(...H_ALPHA) },
        uOpacity: { value: 0.9 },
        uLift: { value: 1 },
        uFray: { value: 0 },
        uTime: { value: this.time },
        uSeed: { value: index * 1.7 },
        uRadius: { value: STAR_RADIUS },
      },
      transparent: true,
      depthWrite: false,
    });
  }

  private resize(): void {
    const { clientWidth, clientHeight } = this.canvas;
    if (clientWidth === 0 || clientHeight === 0) return;
    this.renderer.setSize(clientWidth, clientHeight, false);
    this.camera.aspect = clientWidth / clientHeight;
    this.camera.updateProjectionMatrix();
    this.sky.material.uniforms['uPixelRatio']!.value = this.renderer.getPixelRatio();
    this.requestRender();
  }

  private requestRender(): void {
    if (this.frame === null && !this.disposed) this.frame = requestAnimationFrame(this.render);
  }

  /** Draws the scene; bound once so the frame probe gets no new closure each frame. */
  private readonly draw = (): void => this.renderer.render(this.scene, this.camera);
  private readonly render = (now: number = performance.now()): void => {
    if (this.frame !== null) {
      cancelAnimationFrame(this.frame);
      this.frame = null;
    }
    if (this.disposed) return;
    // At most the profile's frames per second: too early, the frame waits for the next one.
    if (now - this.lastDrawn < this.frameMs - 2 && this.quality.maxFps !== null) {
      this.frame = requestAnimationFrame(this.render);
      return;
    }
    this.lastDrawn = now;
    const distance = this.camera.position.length() / STAR_RADIUS;
    // Close to the surface a drag turns the camera by about what it sees,
    // not by the whole star.
    this.controls.rotateSpeed = Math.min(1, Math.max(0.005, 0.5 * (distance - 1)));
    const moving = this.settling.step(() => this.controls.update());
    // The surface's detail follows the surface's size on screen as the camera zooms.
    this.updateSurface();
    // The corona faces the camera, scaled to the disc as it appears: seen
    // from `d` radii, the limb lies d / sqrt(d² − 1) radii from the centre
    // in the plane through it.
    this.glow.quaternion.copy(this.camera.quaternion);
    this.glow.scale.setScalar(distance / Math.sqrt(Math.max(distance * distance - 1, 1e-6)));
    this.frames.measure(this.renderer.getContext(), this.draw);
    if (moving) this.requestRender();
  };
}
