// The star view: a world's star as a globe you can turn, with its system
// around it (the habitable zone, the planet's orbit and the planet), in
// front of the same star field as the planet view. Drawn on demand; with
// the animation on, ten frames a second.
import {
  AdditiveBlending,
  BufferAttribute,
  BufferGeometry,
  Color,
  DoubleSide,
  Line,
  LineBasicMaterial,
  Mesh,
  MeshBasicMaterial,
  PerspectiveCamera,
  PlaneGeometry,
  Points,
  RingGeometry,
  Scene,
  ShaderMaterial,
  SphereGeometry,
  Vector3,
  WebGLRenderer,
} from 'three';
import { OrbitControls } from 'three/examples/jsm/controls/OrbitControls.js';
import type { StarInfo } from '../api/types/StarInfo';
import { fullPixelRatio } from './pixelRatio';
import { starsFragment, starsVertex } from './shaders';
import { glowFragment, glowVertex, starSurfaceFragment, starSurfaceVertex } from './starShaders';
import { starField } from './stars';
import { keplerPosition, orbitDistance, STAR_RADIUS, starSurface, systemScale } from './starSystem';

/** Screen positions (CSS px) of the points the distances option labels, null when off screen. */
export interface SystemLabels {
  /** The planet now, and its distance from the star (AU). */
  planet: { x: number; y: number; au: number } | null;
  periapsis: { x: number; y: number } | null;
  apoapsis: { x: number; y: number } | null;
  inner: { x: number; y: number } | null;
  outer: { x: number; y: number } | null;
}

/** Redraw interval while the star is animated, ms. */
const FRAME_MS = 100;
/** Width of the glow quad, in star radii. */
const GLOW_SIZE = 7;

export class StarView {
  private readonly renderer: WebGLRenderer;
  private readonly scene = new Scene();
  private readonly camera = new PerspectiveCamera(40, 1, 0.05, 200);
  private readonly controls: OrbitControls;
  private readonly star: Mesh<SphereGeometry, ShaderMaterial>;
  private readonly glow: Mesh<PlaneGeometry, ShaderMaterial>;
  private readonly zone: Mesh<RingGeometry, MeshBasicMaterial>;
  private readonly orbit: Line<BufferGeometry, LineBasicMaterial>;
  private readonly planet: Mesh<SphereGeometry, MeshBasicMaterial>;
  private readonly sky: Points<BufferGeometry, ShaderMaterial>;
  private readonly resizeObserver: ResizeObserver;
  private frame: number | null = null;
  private timer: ReturnType<typeof setInterval> | null = null;
  private time = 0;
  private info: StarInfo | null = null;
  /** Orientation of the periapsis in the orbital plane, radians. */
  private omega = 0;
  private at: (au: number) => number = (au) => au;
  private distancesOn = false;
  private readonly link: Line<BufferGeometry, LineBasicMaterial>;
  private readonly apsides: Mesh<SphereGeometry, MeshBasicMaterial>[];
  /** Called after each frame with the labelled points, while distances are shown. */
  onLabels: (labels: SystemLabels | null) => void = () => {};
  private disposed = false;

  constructor(private readonly canvas: HTMLCanvasElement) {
    this.renderer = new WebGLRenderer({ canvas, antialias: true });
    this.renderer.setPixelRatio(fullPixelRatio(window.devicePixelRatio));
    this.scene.background = new Color('#05070d');
    this.controls = new OrbitControls(this.camera, canvas);
    this.controls.enableDamping = true;
    this.controls.enablePan = false;
    this.controls.minDistance = STAR_RADIUS * 1.4;
    this.controls.maxDistance = 80;
    this.controls.addEventListener('change', () => this.requestRender());

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

    this.star = new Mesh(
      new SphereGeometry(STAR_RADIUS, 128, 64),
      new ShaderMaterial({
        vertexShader: starSurfaceVertex,
        fragmentShader: starSurfaceFragment,
        uniforms: {
          uColor: { value: new Vector3(1, 1, 1) },
          uTime: { value: 0 },
          uLimb: { value: 0.6 },
          uGranulation: { value: 0.12 },
          uSpots: { value: 0.05 },
          uFlares: { value: 0 },
          uSeed: { value: 0 },
        },
      }),
    );
    this.scene.add(this.star);

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
    this.scene.add(this.glow);

    // The system in the orbital plane (x, z).
    this.zone = new Mesh(
      new RingGeometry(2, 3, 128),
      new MeshBasicMaterial({ color: '#3fbf6f', transparent: true, opacity: 0.22, side: DoubleSide, depthWrite: false }),
    );
    this.zone.rotation.x = -Math.PI / 2;
    this.scene.add(this.zone);
    const circle = new BufferGeometry().setFromPoints(
      Array.from({ length: 257 }, (_, i) => {
        const a = (i / 256) * 2 * Math.PI;
        return new Vector3(Math.cos(a), 0, Math.sin(a));
      }),
    );
    this.orbit = new Line(circle, new LineBasicMaterial({ color: '#c8d2e6', transparent: true, opacity: 0.7 }));
    this.scene.add(this.orbit);
    this.planet = new Mesh(new SphereGeometry(0.11, 32, 16), new MeshBasicMaterial({ color: '#7fb2ff' }));
    this.scene.add(this.planet);
    // Distances: the star–planet line and the periapsis and apoapsis.
    this.link = new Line(
      new BufferGeometry().setFromPoints([new Vector3(), new Vector3()]),
      new LineBasicMaterial({ color: '#ffd27a', transparent: true, opacity: 0.85 }),
    );
    this.link.visible = false;
    this.scene.add(this.link);
    this.apsides = [0, 1].map(() => {
      const marker = new Mesh(new SphereGeometry(0.05, 16, 8), new MeshBasicMaterial({ color: '#ffd27a' }));
      marker.visible = false;
      this.scene.add(marker);
      return marker;
    });

    this.resizeObserver = new ResizeObserver(() => this.resize());
    this.resizeObserver.observe(canvas);
    this.resize();
    this.resetView();
  }

  /** Shows a world's star and system; `omega` orients the orbit's periapsis (radians). */
  setStar(info: StarInfo, omega = 0): void {
    this.info = info;
    this.omega = omega;
    const u = this.star.material.uniforms;
    (u['uColor']!.value as Vector3).set(...info.light);
    (this.glow.material.uniforms['uColor']!.value as Vector3).set(...info.light);
    const surface = starSurface(info.teff_k, info.type[0] ?? 'G');
    u['uLimb']!.value = surface.limb;
    u['uGranulation']!.value = surface.granulation;
    u['uSpots']!.value = surface.spots;
    u['uFlares']!.value = surface.flares;
    u['uSeed']!.value = (info.teff_k * 0.37) % 17;
    // Brighter stars glow more, within bounds.
    this.glow.material.uniforms['uStrength']!.value = Math.min(1.4, 0.6 + 0.12 * Math.log10(info.luminosity + 1e-4) + 0.5);

    const at = systemScale(info);
    this.at = at;
    this.zone.geometry.dispose();
    this.zone.geometry = new RingGeometry(at(info.hz_inner_au), at(info.hz_outer_au), 128);
    // The ellipse, the star at a focus.
    const e = info.eccentricity ?? 0;
    this.orbit.geometry.dispose();
    this.orbit.geometry = new BufferGeometry().setFromPoints(
      Array.from({ length: 361 }, (_, i) => this.inPlane((i / 360) * 2 * Math.PI, info.orbit_au * orbitDistance(e, (i / 360) * 2 * Math.PI))),
    );
    this.apsides[0]!.position.copy(this.inPlane(0, info.orbit_au * (1 - e)));
    this.apsides[1]!.position.copy(this.inPlane(Math.PI, info.orbit_au * (1 + e)));
    this.placePlanet();
    this.requestRender();
  }

  /** Shows the star–planet line, the periapsis and apoapsis, and reports their places (see onLabels). */
  setDistances(on: boolean): void {
    this.distancesOn = on;
    this.link.visible = on;
    for (const marker of this.apsides) marker.visible = on;
    if (!on) this.onLabels(null);
    this.requestRender();
  }

  /** Turns and boils the star, moves the planet along its orbit (ten frames a second), or stops them. */
  setAnimated(on: boolean): void {
    if (on && this.timer === null) {
      this.timer = setInterval(() => {
        this.time += FRAME_MS / 1000;
        this.star.material.uniforms['uTime']!.value = this.time;
        this.placePlanet();
        this.requestRender();
      }, FRAME_MS);
    } else if (!on && this.timer !== null) {
      clearInterval(this.timer);
      this.timer = null;
    }
  }

  /** Back to the starting view: the whole system, seen from above its plane. */
  resetView(): void {
    // Far enough for the system (out to about 7.6 star radii) to fit the
    // narrower side of the view.
    const halfFov = ((this.camera.fov / 2) * Math.PI) / 180;
    const distance = (8.2 / Math.tan(halfFov)) / Math.min(1, this.camera.aspect);
    this.camera.position.set(0, distance * 0.55, distance * 0.83);
    this.controls.target.set(0, 0, 0);
    this.controls.update();
    this.requestRender();
  }

  dispose(): void {
    this.disposed = true;
    this.setAnimated(false);
    if (this.frame !== null) cancelAnimationFrame(this.frame);
    this.resizeObserver.disconnect();
    this.controls.dispose();
    for (const object of [this.star, this.glow, this.zone, this.orbit, this.planet, this.sky, this.link, ...this.apsides]) {
      object.geometry.dispose();
      object.material.dispose();
    }
    this.renderer.dispose();
    this.renderer.forceContextLoss();
  }

  /** A point of the orbital plane at a true anomaly and distance (AU), in view units. */
  private inPlane(trueAnomaly: number, au: number): Vector3 {
    const angle = trueAnomaly + this.omega;
    const r = this.at(au);
    return new Vector3(Math.cos(angle) * r, 0, -Math.sin(angle) * r);
  }

  /** Distance of the planet from the star now, AU. */
  private planetAu = 1;

  private placePlanet(): void {
    if (!this.info) return;
    // A turn in a minute when animated (not the real year), at Kepler's
    // pace: faster near the star.
    const e = this.info.eccentricity ?? 0;
    const { trueAnomaly, distance } = keplerPosition(e, 0.6 + this.time * ((2 * Math.PI) / 60));
    this.planetAu = this.info.orbit_au * distance;
    this.planet.position.copy(this.inPlane(trueAnomaly, this.planetAu));
    const points = this.link.geometry.getAttribute('position') as BufferAttribute;
    points.setXYZ(1, this.planet.position.x, this.planet.position.y, this.planet.position.z);
    points.needsUpdate = true;
  }

  /** Screen position of a point of the scene, CSS px, or null behind the camera. */
  private screen(point: Vector3): { x: number; y: number } | null {
    const p = point.clone().project(this.camera);
    if (p.z > 1) return null;
    return { x: ((p.x + 1) / 2) * this.canvas.clientWidth, y: ((1 - p.y) / 2) * this.canvas.clientHeight };
  }

  private resize(): void {
    const { clientWidth, clientHeight } = this.canvas;
    if (clientWidth === 0 || clientHeight === 0) {
      return;
    }
    this.renderer.setSize(clientWidth, clientHeight, false);
    this.camera.aspect = clientWidth / clientHeight;
    this.camera.updateProjectionMatrix();
    this.sky.material.uniforms['uPixelRatio']!.value = this.renderer.getPixelRatio();
    this.requestRender();
  }

  private requestRender(): void {
    if (this.frame === null && !this.disposed) {
      this.frame = requestAnimationFrame(this.render);
    }
  }

  private readonly render = (): void => {
    this.frame = null;
    const moving = this.controls.update();
    // The glow faces the camera.
    this.glow.quaternion.copy(this.camera.quaternion);
    this.renderer.render(this.scene, this.camera);
    if (this.distancesOn && this.info) {
      // The zone's edges are labelled across the apsides line, away from
      // the periapsis and apoapsis labels.
      const edge = (au: number) => this.screen(this.inPlane(Math.PI / 2, au));
      const planet = this.screen(this.planet.position);
      this.onLabels({
        planet: planet && { ...planet, au: this.planetAu },
        periapsis: this.screen(this.apsides[0]!.position),
        apoapsis: this.screen(this.apsides[1]!.position),
        inner: edge(this.info.hz_inner_au),
        outer: edge(this.info.hz_outer_au),
      });
    }
    if (moving) {
      this.requestRender();
    }
  };
}
