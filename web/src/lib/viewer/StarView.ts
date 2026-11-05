// The star view: a world's star as a globe you can turn, with its system
// around it (the habitable zone, the planet's orbit and the planet), in
// front of the same star field as the planet view. Drawn on demand; with
// the animation on, thirty frames a second at most.
import {
  AdditiveBlending,
  Group,
  BufferAttribute,
  BufferGeometry,
  Color,
  DoubleSide,
  Line,
  LineBasicMaterial,
  LineDashedMaterial,
  Mesh,
  MeshBasicMaterial,
  PerspectiveCamera,
  PlaneGeometry,
  Points,
  Raycaster,
  Vector2,
  RingGeometry,
  SRGBColorSpace,
  Scene,
  ShaderMaterial,
  SphereGeometry,
  Vector3,
  WebGLRenderer,
} from 'three';
import { OrbitControls } from 'three/examples/jsm/controls/OrbitControls.js';
import type { Giant } from '../api/types/Giant';
import type { GiantInfo } from '../api/types/GiantInfo';
import type { StarInfo } from '../api/types/StarInfo';
import { fullPixelRatio } from './pixelRatio';
import { starsFragment, starsVertex } from './shaders';
import {
  glowFragment,
  glowVertex,
  prominenceFragment,
  prominenceVertex,
  starSurfaceFragment,
  starSurfaceVertex,
} from './starShaders';
import { starField } from './stars';
import { prominenceGeometry } from './prominenceGeometry';
import {
  keplerPosition,
  type MoonOrbit,
  orbitDistance,
  PLANET_MARKER,
  compressedFrom,
  giantMarkerSize,
  type Prominence,
  prominences,
  STAR_RADIUS,
  starSurface,
  orbitalSpeed,
  systemCompressedFrom,
  systemScale,
  velocityScale,
} from './starSystem';

/** Seconds between two eruptions, while animated. */
const ERUPTION_S = 9;
/** The red of hydrogen (H-alpha), linear RGB. */
const H_ALPHA: [number, number, number] = [1, 0.2, 0.14];

/** Screen positions (CSS px) of the points the distances option labels, null when off screen. */
export interface SystemLabels {
  /** The planet now, and its distance from the star (AU). */
  planet: { x: number; y: number; au: number } | null;
  periapsis: { x: number; y: number } | null;
  apoapsis: { x: number; y: number } | null;
  inner: { x: number; y: number } | null;
  outer: { x: number; y: number } | null;
  /** The planet now, while the live distances are off (its name and mean distance label it). */
  self: { x: number; y: number } | null;
  /**
   * The giant planets now (every planet of a planetary system, the
   * selected one included), with their mean distance from the star and
   * their distance now (`distance`), AU, and their index in setStar's
   * `giants` (empty when hidden).
   */
  giants: {
    x: number;
    y: number;
    name: string;
    au: number;
    distance: number;
    index: number;
    /** Its speed now, km/s, and the middle of its velocity vector on screen (null when hidden or off screen). */
    speed: number;
    arrow: { x: number; y: number } | null;
  }[];
}

/** Shortest time between two animated frames, ms (thirty frames a second at most). */
const FRAME_MS = 1000 / 30;
/** Longest step of the animation clock, s: a stall (a world loading, a hidden tab) pauses the motion instead of making it jump. */
const MAX_STEP_S = 0.1;
/** Seconds of animation for one orbit of the planet. */
export const YEAR_S = 60;
/**
 * Seconds of animation for one orbit of a planetary system's innermost
 * planet: the whole system runs on that clock, whichever planet is
 * selected.
 */
export const SYSTEM_INNER_YEAR_S = 10;
/** Mean anomaly of the planet when the animation clock reads zero, radians. */
const START_ANOMALY = 0.6;
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
  private loop: number | null = null;
  private lastTick = 0;
  private time = 0;
  private info: StarInfo | null = null;
  /** Orientation of the periapsis in the orbital plane, radians. */
  private omega = 0;
  private at: (au: number) => number = (au) => au;
  private distancesOn = false;
  /** The prominences, turning with the surface; the eruption under way. */
  private readonly arcs = new Group();
  private eruption: Mesh<BufferGeometry, ShaderMaterial> | null = null;
  private klass = 'G';
  private seed = 0;
  private orbitOn = true;
  private zoneOn = true;
  private readonly link: Line<BufferGeometry, LineBasicMaterial>;
  private readonly apsides: Mesh<SphereGeometry, MeshBasicMaterial>[];
  /** Called after each frame with the labelled points, while distances are shown. */
  onLabels: (labels: SystemLabels | null) => void = () => {};
  /**
   * Called on each animated frame with the share of the year elapsed since
   * the periapsis (0 to 1) and the Earth days simulated since the start.
   */
  onClock: (yearFraction: number, days: number) => void = () => {};
  /** Called with a planet's index (in setStar's `giants`) when it is clicked; null ignores clicks. */
  onPickPlanet: ((index: number) => void) | null = null;
  /** Where a press started, to tell a click from a drag. */
  private pressAt: { x: number; y: number } | null = null;
  private disposed = false;
  /** The planet's moons around it (see setMoons), centred on the barycentre. */
  private readonly moonGroup = new Group();
  /** The giant planets of the system, their orbits and rings (see setStar). */
  private readonly giantGroup = new Group();
  private giants: Giant[] = [];
  private giantMarkers: Mesh<SphereGeometry, MeshBasicMaterial>[] = [];
  /** The giants' distances from the star now, AU (see placeGiants). */
  private giantDistances: number[] = [];
  /**
   * In a planetary system: a dotted line from the star to each planet and
   * its velocity vector (see setPlanetLinks, setVelocities), with each
   * planet's speed now (km/s) and the middle of its vector.
   */
  private readonly linkGroup = new Group();
  private readonly arrowGroup = new Group();
  private planetLinks: Line<BufferGeometry, LineDashedMaterial>[] = [];
  private arrows: Line<BufferGeometry, LineBasicMaterial>[] = [];
  private giantSpeeds: number[] = [];
  private arrowMids: Vector3[] = [];
  /** View units per km/s of the velocity vectors (see velocityScale). */
  private arrowScale = 0;
  private linksOn = true;
  private velocitiesOn = true;
  private giantsOn = true;
  /** How far the system reaches from the star, view units (see resetView). */
  private reach = 6.8;
  /** Whether the user moved the camera since the view was last fitted. */
  private userMoved = false;
  private moonOrbits: MoonOrbit[] = [];
  private moonMarkers: Mesh<SphereGeometry, MeshBasicMaterial>[] = [];
  /**
   * A planetary system's selected planet, by its index in `giants`, or
   * null for a lone world's view. With one, every planet is drawn as a
   * giant, on a scale and a clock that do not depend on the selection.
   */
  private selected: number | null = null;
  /** The planets of the system shown, to restart the clock only when they change. */
  private systemKey = '';
  /** Earth days per second of animation, in a planetary system, at speed ×1. */
  private daysPerSecond = 0;
  /** Seconds of the system's clock (the animation's time times the speed). */
  private orbitTime = 0;
  /** How fast a planetary system's clock runs, a factor of its pace (see setSpeed). */
  private speed = 1;
  /** Where the distances start being compressed, AU. */
  private from = 1;

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
    // Only the user's drags and wheels start the controls, not resetView.
    this.controls.addEventListener('start', () => (this.userMoved = true));
    // A click on a planet's marker picks it (a drag turns the view).
    canvas.addEventListener('pointerdown', (event) => (this.pressAt = { x: event.clientX, y: event.clientY }));
    canvas.addEventListener('pointerup', (event) => {
      const at = this.pressAt;
      this.pressAt = null;
      if (!at || !this.onPickPlanet || Math.hypot(event.clientX - at.x, event.clientY - at.y) > 5) return;
      const index = this.pickPlanet(event.clientX, event.clientY);
      if (index !== null) this.onPickPlanet(index);
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
    // Glow and prominences share the star's centre, so sorting them by
    // distance is a tie that flips as the view turns: fix their order, the
    // prominences over the glow (drawn after it, the glow would wash them
    // out to white).
    this.glow.renderOrder = 1;
    this.scene.add(this.glow);
    this.scene.add(this.arcs);

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
    this.planet = new Mesh(new SphereGeometry(PLANET_MARKER, 32, 16), new MeshBasicMaterial({ color: '#7fb2ff' }));
    this.scene.add(this.planet);
    this.scene.add(this.moonGroup);
    this.scene.add(this.giantGroup);
    this.scene.add(this.linkGroup);
    this.scene.add(this.arrowGroup);
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

  /**
   * Shows a world's star and system; `omega` orients the orbit's periapsis
   * (radians); `giants` are the other planets of the system. In a
   * planetary system, `giants` lists every planet and `selected` is the
   * index of the one `info` describes: it keeps its orbit, its apsides and
   * its moons, on its own marker, while the scale, the clock and the
   * camera stay as they are when the selection changes.
   */
  setStar(info: StarInfo, omega = 0, giants: Giant[] = [], selected: number | null = null): void {
    const system = selected !== null && selected >= 0 && selected < giants.length;
    const key = system ? `${info.type} ${giants.map((g) => `${g.orbit_au}:${g.eccentricity}:${g.phase}`).join(' ')}` : '';
    if (system) {
      // Another system: its planets start from their drawn places.
      if (key !== this.systemKey) this.time = this.orbitTime = 0;
    } else if (this.info && (this.systemKey !== '' || this.info.type !== info.type || this.info.orbit_au !== info.orbit_au || this.omega !== omega)) {
      // Another system: its planet starts its orbit from the same point.
      this.time = this.orbitTime = 0;
    }
    this.systemKey = key;
    this.selected = system ? selected : null;
    this.daysPerSecond = system ? Math.min(...giants.map((g) => g.year_days)) / SYSTEM_INNER_YEAR_S : 0;
    this.planet.visible = !system;
    // The selected planet's moons circle its own marker, as much larger as it is.
    this.moonGroup.scale.setScalar(system ? giantMarkerSize(giants[selected]!.radius_earth) / PLANET_MARKER : 1);
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
    this.klass = info.type[0] ?? 'G';
    this.seed = Math.round(info.teff_k * 7 + info.orbit_au * 1000);
    this.buildProminences();
    // Brighter stars glow more, within bounds.
    this.glow.material.uniforms['uStrength']!.value = Math.min(1.4, 0.6 + 0.12 * Math.log10(info.luminosity + 1e-4) + 0.5);

    this.from = system ? systemCompressedFrom(info) : compressedFrom(info);
    const at = systemScale(info, giants, 6, this.from);
    this.at = at;
    this.giants = giants;
    this.buildGiants();
    this.buildVectors();
    const inner = system ? info.hz_outer_au : compressedFrom(info) / 1.15;
    const reach = Math.max(at(inner), ...giants.map((g) => at(g.orbit_au * (1 + g.eccentricity))));
    // A system that reaches further (or less far) is fitted again, unless
    // the user has moved the camera since the last fit.
    const refit = reach !== this.reach && !this.userMoved;
    this.reach = reach;
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
    if (refit) this.resetView();
    this.requestRender();
  }

  /**
   * Draws the planet's moons around it, on the orbits `moonOrbits` gives
   * (not to scale), moving with the animation; none with an empty list.
   */
  setMoons(orbits: MoonOrbit[]): void {
    this.disposeMoons();
    this.moonOrbits = orbits;
    for (const orbit of orbits) {
      const ring = new Line(
        new BufferGeometry().setFromPoints(
          Array.from({ length: 97 }, (_, i) => {
            const a = (i / 96) * 2 * Math.PI;
            return new Vector3(Math.cos(a) * orbit.radius, 0, -Math.sin(a) * orbit.radius);
          }),
        ),
        new LineBasicMaterial({ color: '#c8d2e6', transparent: true, opacity: 0.35 }),
      );
      ring.rotation.x = orbit.inclination;
      // Transparent: drawn in a fixed order with the glow and prominences.
      ring.renderOrder = 3;
      this.moonGroup.add(ring);
      const marker = new Mesh(new SphereGeometry(orbit.size, 16, 8), new MeshBasicMaterial({ color: '#d9d4c8' }));
      this.moonMarkers.push(marker);
      this.moonGroup.add(marker);
    }
    this.placePlanet();
    this.requestRender();
  }

  private disposeMoons(): void {
    for (const child of [...this.moonGroup.children]) {
      const object = child as Mesh<BufferGeometry, MeshBasicMaterial>;
      object.geometry.dispose();
      object.material.dispose();
      this.moonGroup.remove(child);
    }
    this.moonMarkers = [];
    this.moonOrbits = [];
  }

  /** Places the moons at the animation's time, and the planet around the barycentre of a binary. */
  private placeMoons(): void {
    this.moonGroup.position.copy(this.planet.position);
    // In a planetary system, the moons run on the system's clock.
    const time = this.selected !== null ? this.orbitTime : this.time;
    this.moonOrbits.forEach((orbit, k) => {
      const angle = orbit.phase + time * orbit.speed;
      const local = new Vector3(Math.cos(angle) * orbit.radius, 0, -Math.sin(angle) * orbit.radius);
      local.applyAxisAngle(new Vector3(1, 0, 0), orbit.inclination);
      this.moonMarkers[k]?.position.copy(local);
      if (orbit.swing > 0) {
        this.planet.position.addScaledVector(local, -orbit.swing * this.moonGroup.scale.x);
      }
    });
  }

  /** Shows the star–planet line, the periapsis and apoapsis, and reports their places (see onLabels). */
  setDistances(on: boolean): void {
    this.distancesOn = on;
    this.applyVisibility();
    this.requestRender();
  }

  /**
   * Draws the world itself as the giant it is (its colour, size and
   * rings), or as the usual blue marker of a rocky world (null).
   */
  setWorldGiant(info: GiantInfo | null): void {
    for (const child of [...this.planet.children]) {
      const mesh = child as Mesh<RingGeometry, MeshBasicMaterial>;
      mesh.geometry.dispose();
      mesh.material.dispose();
      this.planet.remove(child);
    }
    if (!info) {
      this.planet.scale.setScalar(1);
      this.planet.material.color.set('#7fb2ff');
      this.requestRender();
      return;
    }
    const colour = new Color().setRGB(...info.palette.zone, SRGBColorSpace);
    this.planet.material.color.copy(colour);
    this.planet.scale.setScalar(giantMarkerSize(info.radius_earth) / PLANET_MARKER);
    if (info.rings) {
      const ring = new Mesh(
        new RingGeometry(PLANET_MARKER * 1.35, PLANET_MARKER * 2.3, 64),
        new MeshBasicMaterial({
          color: new Color().setRGB(...info.rings.colour, SRGBColorSpace),
          transparent: true,
          opacity: info.rings.bright ? 0.6 : 0.22,
          side: DoubleSide,
          depthWrite: false,
        }),
      );
      ring.rotation.x = -Math.PI / 2 + 0.45;
      ring.renderOrder = 3;
      this.planet.add(ring);
    }
    this.requestRender();
  }

  /** Shows or hides the giant planets, their orbits and their labels. */
  setGiantsVisible(on: boolean): void {
    this.giantsOn = on;
    this.giantGroup.visible = on;
    this.applyVisibility();
    this.requestRender();
  }

  /**
   * The giants' orbits (ellipses with the star at a focus, faint, in their
   * own colour), their disks and rings, and a dashed circle where the
   * distances start being compressed.
   */
  private buildGiants(): void {
    for (const child of [...this.giantGroup.children]) {
      child.traverse((object) => {
        const mesh = object as Mesh<BufferGeometry, MeshBasicMaterial>;
        mesh.geometry?.dispose();
        mesh.material?.dispose();
      });
      this.giantGroup.remove(child);
    }
    this.giantMarkers = [];
    const from = this.from;
    if (this.giants.some((g) => g.orbit_au * (1 + g.eccentricity) > from)) {
      const boundary = new Line(
        new BufferGeometry().setFromPoints(
          Array.from({ length: 193 }, (_, i) => this.inPlaneAt((i / 192) * 2 * Math.PI, from)),
        ),
        new LineDashedMaterial({ color: '#8a93a8', dashSize: 0.12, gapSize: 0.12, transparent: true, opacity: 0.5 }),
      );
      boundary.computeLineDistances();
      boundary.renderOrder = 3;
      this.giantGroup.add(boundary);
    }
    for (const giant of this.giants) {
      const colour = new Color().setRGB(...giant.colour, SRGBColorSpace);
      const orbit = new Line(
        new BufferGeometry().setFromPoints(
          Array.from({ length: 361 }, (_, i) => {
            const nu = (i / 360) * 2 * Math.PI;
            return this.inPlaneAt(nu + giant.periapsis_angle, giant.orbit_au * orbitDistance(giant.eccentricity, nu));
          }),
        ),
        new LineBasicMaterial({ color: colour.clone().lerp(new Color('#c8d2e6'), 0.5), transparent: true, opacity: 0.45 }),
      );
      orbit.renderOrder = 3;
      this.giantGroup.add(orbit);
      const size = giantMarkerSize(giant.radius_earth);
      const marker = new Mesh(new SphereGeometry(size, 32, 16), new MeshBasicMaterial({ color: colour }));
      if (giant.rings !== 'none') {
        const ring = new Mesh(
          new RingGeometry(size * 1.35, size * 2.3, 64),
          new MeshBasicMaterial({
            color: colour.clone().lerp(new Color('#e8e0d0'), 0.5),
            transparent: true,
            opacity: giant.rings === 'bright' ? 0.6 : 0.22,
            side: DoubleSide,
            depthWrite: false,
          }),
        );
        // Tilted, as most giants' rings (and spins) are.
        ring.rotation.x = -Math.PI / 2 + 0.45;
        ring.renderOrder = 3;
        marker.add(ring);
      }
      this.giantMarkers.push(marker);
      this.giantGroup.add(marker);
    }
  }

  /**
   * A planetary system's dotted star–planet lines and velocity vectors,
   * one each per planet; none for a lone world.
   */
  private buildVectors(): void {
    for (const group of [this.linkGroup, this.arrowGroup]) {
      for (const child of [...group.children]) {
        const line = child as Line<BufferGeometry, LineBasicMaterial>;
        line.geometry.dispose();
        line.material.dispose();
        group.remove(child);
      }
    }
    this.planetLinks = [];
    this.arrows = [];
    this.giantSpeeds = [];
    this.arrowMids = [];
    if (this.selected === null || !this.info) return;
    this.arrowScale = velocityScale(this.info.mass, this.giants);
    for (const giant of this.giants) {
      const colour = new Color().setRGB(...giant.colour, SRGBColorSpace).lerp(new Color('#ffe3a8'), 0.6);
      const link = new Line(
        new BufferGeometry().setFromPoints([new Vector3(), new Vector3()]),
        new LineDashedMaterial({ color: colour, dashSize: 0.06, gapSize: 0.08, transparent: true, opacity: 0.6 }),
      );
      link.renderOrder = 3;
      link.frustumCulled = false;
      this.planetLinks.push(link);
      this.linkGroup.add(link);
      // The shaft, then the two barbs of the head: tip, barb, tip, barb.
      const arrow = new Line(
        new BufferGeometry().setFromPoints(Array.from({ length: 5 }, () => new Vector3())),
        new LineBasicMaterial({ color: '#9fe0ff', transparent: true, opacity: 0.9 }),
      );
      arrow.renderOrder = 3;
      arrow.frustumCulled = false;
      this.arrows.push(arrow);
      this.arrowGroup.add(arrow);
    }
    this.applyVisibility();
  }

  /** Shows or hides a planetary system's dotted star–planet lines, labelled with each planet's distance now. */
  setPlanetLinks(on: boolean): void {
    this.linksOn = on;
    this.applyVisibility();
    this.requestRender();
  }

  /** Shows or hides a planetary system's velocity vectors, labelled with each planet's speed now. */
  setVelocities(on: boolean): void {
    this.velocitiesOn = on;
    this.applyVisibility();
    this.requestRender();
  }

  /**
   * Moves giant `k`'s dotted line to its marker, and its velocity vector:
   * from the marker, along its orbit the way it goes (the tangent of the
   * drawn ellipse, compression included), as long as its speed.
   */
  private placeVector(k: number, giant: Giant, position: Vector3, distanceAu: number): void {
    const link = this.planetLinks[k];
    const arrow = this.arrows[k];
    if (!link || !arrow || !this.info) return;
    const ends = link.geometry.getAttribute('position') as BufferAttribute;
    ends.setXYZ(1, position.x, position.y, position.z);
    ends.needsUpdate = true;
    link.computeLineDistances();
    const speed = orbitalSpeed(this.info.mass, giant.orbit_au, distanceAu);
    this.giantSpeeds[k] = speed;
    const at = (mean: number) => {
      const { trueAnomaly, distance } = keplerPosition(giant.eccentricity, mean);
      return this.inPlaneAt(trueAnomaly + giant.periapsis_angle, giant.orbit_au * distance);
    };
    const mean = this.giantAnomaly(k);
    const direction = at(mean + 1e-3).sub(at(mean - 1e-3)).normalize();
    const length = speed * this.arrowScale;
    const tip = position.clone().addScaledVector(direction, length);
    const barb = Math.min(0.06, length * 0.35);
    const side = new Vector3(-direction.z, 0, direction.x).multiplyScalar(barb * 0.6);
    const back = tip.clone().addScaledVector(direction, -barb);
    const points = arrow.geometry.getAttribute('position') as BufferAttribute;
    [position, tip, back.clone().add(side), tip, back.clone().sub(side)].forEach((p, i) => points.setXYZ(i, p.x, p.y, p.z));
    points.needsUpdate = true;
    this.arrowMids[k] = position.clone().addScaledVector(direction, length / 2);
  }

  /**
   * Places the giants at the animation's time: each at Kepler's pace, its
   * year in proportion to the world's (a Jupiter around a world of Earth's
   * year turns twelve times slower).
   */
  private placeGiants(): void {
    this.giants.forEach((giant, k) => {
      const { trueAnomaly, distance } = keplerPosition(giant.eccentricity, this.giantAnomaly(k));
      this.giantDistances[k] = giant.orbit_au * distance;
      const position = this.inPlaneAt(trueAnomaly + giant.periapsis_angle, giant.orbit_au * distance);
      this.giantMarkers[k]?.position.copy(position);
      this.placeVector(k, giant, position, giant.orbit_au * distance);
    });
  }

  /**
   * Mean anomaly of giant `k` now, radians. In a planetary system, every
   * planet starts from its drawn place and runs on the system's clock.
   */
  private giantAnomaly(k: number): number {
    const giant = this.giants[k]!;
    if (this.selected !== null) {
      return 2 * Math.PI * (giant.phase + (this.orbitTime * this.daysPerSecond) / giant.year_days);
    }
    const year = this.info?.year_days ?? 365.25;
    return giant.phase + this.time * ((2 * Math.PI) / YEAR_S) * (year / giant.year_days);
  }

  /**
   * How fast a planetary system's planets go round, a factor of its pace
   * (the innermost planet's year in SYSTEM_INNER_YEAR_S at ×1); the star
   * keeps boiling at its own pace. A lone world's view ignores it.
   */
  setSpeed(factor: number): void {
    this.speed = factor;
  }

  /** Shows or hides the orbit line (and its periapsis and apoapsis). */
  setOrbitVisible(on: boolean): void {
    this.orbitOn = on;
    this.applyVisibility();
    this.requestRender();
  }

  /** Shows or hides the prominences and eruptions. */
  setProminences(on: boolean): void {
    this.arcs.visible = on;
    this.requestRender();
  }

  /** Shows or hides the habitable zone. */
  setZoneVisible(on: boolean): void {
    this.zoneOn = on;
    this.applyVisibility();
    this.requestRender();
  }

  private applyVisibility(): void {
    this.orbit.visible = this.orbitOn;
    this.zone.visible = this.zoneOn;
    // A planetary system draws a dotted line to every planet instead.
    this.link.visible = this.distancesOn && this.selected === null;
    const system = this.selected !== null && this.giantsOn;
    this.linkGroup.visible = system && this.linksOn;
    this.arrowGroup.visible = system && this.velocitiesOn;
    for (const marker of this.apsides) marker.visible = this.distancesOn && this.orbitOn;
  }

  /**
   * Turns and boils the star and moves the planet along its orbit (a year in
   * YEAR_S seconds, thirty frames a second at most, on the display's clock),
   * or stops them.
   */
  setAnimated(on: boolean): void {
    if (on && this.loop === null && !this.disposed) {
      this.lastTick = performance.now();
      this.loop = requestAnimationFrame(this.tick);
    } else if (!on && this.loop !== null) {
      cancelAnimationFrame(this.loop);
      this.loop = null;
    }
  }

  private readonly tick = (now: number): void => {
    this.loop = requestAnimationFrame(this.tick);
    const elapsed = now - this.lastTick;
    if (elapsed < FRAME_MS - 4) return;
    this.lastTick = now;
    const step = Math.min(elapsed / 1000, MAX_STEP_S);
    this.time += step;
    this.orbitTime += step * this.speed;
    this.star.material.uniforms['uTime']!.value = this.time;
    this.animateProminences();
    this.placePlanet();
    const turn = this.meanAnomaly() / (2 * Math.PI);
    const days = this.selected !== null ? this.orbitTime * this.daysPerSecond : (this.time / YEAR_S) * (this.info?.year_days ?? 365.25);
    this.onClock(turn - Math.floor(turn), days);
    this.render();
  };

  /** Mean anomaly of the planet now, radians. */
  private meanAnomaly(): number {
    if (this.selected !== null) return this.giantAnomaly(this.selected);
    return START_ANOMALY + this.time * ((2 * Math.PI) / YEAR_S);
  }

  /** Back to the starting view: the whole system, seen from above its plane. */
  resetView(): void {
    // Far enough for the system (out to about 7.6 star radii) to fit the
    // narrower side of the view.
    const halfFov = ((this.camera.fov / 2) * Math.PI) / 180;
    const distance = ((this.reach + 1.4) / Math.tan(halfFov)) / Math.min(1, this.camera.aspect);
    // Room to zoom out past the fitted view, however far the system reaches.
    this.controls.maxDistance = Math.max(80, distance * 2);
    this.userMoved = false;
    this.camera.position.set(0, distance * 0.55, distance * 0.83);
    this.controls.target.set(0, 0, 0);
    this.controls.update();
    this.requestRender();
  }

  /**
   * The planet under a point of the screen (client px), by its index in
   * setStar's `giants` (in a planetary system, every planet, the selected
   * one included): the nearest marker within a few pixels, null when none
   * is there or the planets are hidden.
   */
  pickPlanet(clientX: number, clientY: number): number | null {
    if (!this.giantsOn) return null;
    const rect = this.canvas.getBoundingClientRect();
    let best: number | null = null;
    let bestDistance = 12;
    this.giantMarkers.forEach((marker, i) => {
      if (!marker.visible) return;
      const p = marker.getWorldPosition(new Vector3()).project(this.camera);
      if (p.z > 1) return;
      const x = rect.left + ((p.x + 1) / 2) * rect.width;
      const y = rect.top + ((1 - p.y) / 2) * rect.height;
      const d = Math.hypot(x - clientX, y - clientY);
      if (d < bestDistance) {
        bestDistance = d;
        best = i;
      }
    });
    // A marker larger than the tolerance: the ray through it.
    if (best === null) {
      const ray = new Raycaster();
      ray.setFromCamera(
        new Vector2(((clientX - rect.left) / rect.width) * 2 - 1, -((clientY - rect.top) / rect.height) * 2 + 1),
        this.camera,
      );
      const hit = ray.intersectObjects(this.giantMarkers, false)[0];
      if (hit) best = this.giantMarkers.indexOf(hit.object as (typeof this.giantMarkers)[number]);
    }
    return best;
  }

  dispose(): void {
    this.disposed = true;
    for (const group of [this.giantGroup, this.linkGroup, this.arrowGroup]) group.traverse((object) => {
      const mesh = object as Mesh<BufferGeometry, MeshBasicMaterial>;
      mesh.geometry?.dispose();
      mesh.material?.dispose();
    });
    for (const child of this.arcs.children) {
      const mesh = child as Mesh<BufferGeometry, ShaderMaterial>;
      mesh.geometry.dispose();
      mesh.material.dispose();
    }
    this.setAnimated(false);
    this.disposeMoons();
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

  /** A prominence: a bundle of glowing strands along its arc (see prominenceGeometry). */
  private arcMesh(arc: Prominence, index: number): Mesh<BufferGeometry, ShaderMaterial> {
    const mesh = new Mesh(
      prominenceGeometry(arc, STAR_RADIUS, this.seed * 13 + index),
      new ShaderMaterial({
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
      }),
    );
    mesh.renderOrder = 2;
    return mesh;
  }

  private buildProminences(): void {
    for (const child of [...this.arcs.children]) {
      const mesh = child as Mesh<BufferGeometry, ShaderMaterial>;
      mesh.geometry.dispose();
      mesh.material.dispose();
      this.arcs.remove(mesh);
    }
    this.eruption = null;
    prominences(this.klass, this.seed).forEach((arc, i) => this.arcs.add(this.arcMesh(arc, i)));
  }

  /**
   * Turns the prominences with the surface (the surface shader turns by
   * 0.02 rad/s), flickers them and, now and then on active stars, blows one
   * off: it rises and fades out over a few seconds.
   */
  private animateProminences(): void {
    this.arcs.rotation.y = -this.time * 0.02;
    for (const child of this.arcs.children) {
      ((child as Mesh<BufferGeometry, ShaderMaterial>).material.uniforms['uTime']!).value = this.time;
    }
    const cycle = Math.floor(this.time / ERUPTION_S);
    const phase = (this.time % ERUPTION_S) / ERUPTION_S;
    const active = this.klass === 'M' || this.klass === 'K' || this.klass === 'G';
    if (!active) return;
    if (!this.eruption || this.eruption.userData['cycle'] !== cycle) {
      if (this.eruption) {
        this.arcs.remove(this.eruption);
        this.eruption.geometry.dispose();
        this.eruption.material.dispose();
      }
      const arc = prominences('M', this.seed + 31 * (cycle + 1))[0]!;
      this.eruption = this.arcMesh({ ...arc, height: arc.height + 0.15 }, cycle);
      this.eruption.userData['cycle'] = cycle;
      this.arcs.add(this.eruption);
    }
    // Rises over the first half of the cycle, gone by its end.
    const u = this.eruption.material.uniforms;
    const rise = Math.min(phase * 1.6, 1) ** 1.5;
    u['uLift']!.value = 1 + 9 * rise;
    // The strands come apart as the arc blows off, as torn field lines.
    u['uFray']!.value = 0.35 * rise;
    u['uOpacity']!.value = 1.1 * (1 - phase) ** 2;
  }

  /** A point of the orbital plane at a true anomaly and distance (AU), in view units. */
  private inPlane(trueAnomaly: number, au: number): Vector3 {
    return this.inPlaneAt(trueAnomaly + this.omega, au);
  }

  /** A point of the orbital plane at an angle from the reference direction and a distance (AU), in view units. */
  private inPlaneAt(angle: number, au: number): Vector3 {
    const r = this.at(au);
    return new Vector3(Math.cos(angle) * r, 0, -Math.sin(angle) * r);
  }

  /** Distance of the planet from the star now, AU. */
  private planetAu = 1;

  private placePlanet(): void {
    if (!this.info) return;
    // A turn in YEAR_S seconds when animated (not the real year), at
    // Kepler's pace: faster near the star.
    const e = this.info.eccentricity ?? 0;
    const { trueAnomaly, distance } = keplerPosition(e, this.meanAnomaly());
    this.planetAu = this.info.orbit_au * distance;
    this.placeGiants();
    // In a planetary system, the selected planet is its own giant marker.
    const own = this.selected !== null ? this.giantMarkers[this.selected] : undefined;
    this.planet.position.copy(own ? own.position : this.inPlane(trueAnomaly, this.planetAu));
    this.placeMoons();
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
    if (this.frame !== null) {
      cancelAnimationFrame(this.frame);
      this.frame = null;
    }
    if (this.disposed) return;
    const moving = this.controls.update();
    // The glow faces the camera.
    this.glow.quaternion.copy(this.camera.quaternion);
    this.renderer.render(this.scene, this.camera);
    const giantsShown = this.giantsOn && this.giants.length > 0;
    if ((this.distancesOn || giantsShown) && this.info) {
      // The zone's edges are labelled across the apsides line, away from
      // the periapsis and apoapsis labels.
      const edge = (au: number) => this.screen(this.inPlane(Math.PI / 2, au));
      const planet = this.distancesOn ? this.screen(this.planet.position) : null;
      const on = this.distancesOn;
      this.onLabels({
        planet: planet && { ...planet, au: this.planetAu },
        // A planetary system labels the selected planet as the others.
        self: on || this.selected !== null ? null : this.screen(this.planet.position),
        periapsis: on && this.orbitOn ? this.screen(this.apsides[0]!.position) : null,
        apoapsis: on && this.orbitOn ? this.screen(this.apsides[1]!.position) : null,
        inner: on && this.zoneOn ? edge(this.info.hz_inner_au) : null,
        outer: on && this.zoneOn ? edge(this.info.hz_outer_au) : null,
        giants: giantsShown
          ? this.giants.flatMap((giant, k) => {
              const at = this.screen(this.giantMarkers[k]!.position);
              return at ? [
                    {
                      ...at,
                      name: giant.name,
                      au: giant.orbit_au,
                      distance: this.giantDistances[k] ?? giant.orbit_au,
                      index: k,
                      speed: this.giantSpeeds[k] ?? 0,
                      arrow: this.arrowGroup.visible && this.arrowMids[k] ? this.screen(this.arrowMids[k]) : null,
                    },
                  ] : [];
            })
          : [],
      });
    } else {
      this.onLabels(null);
    }
    if (moving) {
      this.requestRender();
    }
  };
}
