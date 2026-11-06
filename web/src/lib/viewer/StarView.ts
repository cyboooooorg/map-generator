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
  LineSegments,
  Mesh,
  MeshBasicMaterial,
  MOUSE,
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
  type Texture,
  DataTexture,
  TextureLoader,
  Vector3,
  WebGLRenderer,
} from 'three';
import { OrbitControls } from 'three/examples/jsm/controls/OrbitControls.js';
import { Line2 } from 'three/examples/jsm/lines/Line2.js';
import { LineGeometry } from 'three/examples/jsm/lines/LineGeometry.js';
import { LineMaterial } from 'three/examples/jsm/lines/LineMaterial.js';
import type { Giant } from '../api/types/Giant';
import type { GiantInfo } from '../api/types/GiantInfo';
import type { StarInfo } from '../api/types/StarInfo';
import { fullPixelRatio } from './pixelRatio';
import { axesOnScreen, type ScreenAxis } from './axes';
import { starsFragment, starsVertex } from './shaders';
import {
  glowFragment,
  glowVertex,
  prominenceFragment,
  prominenceVertex,
  starSurfaceFragment,
  starSurfaceVertex,
} from './starShaders';
import { previewTexture } from './previewTexture';
import type { Preview } from '../api/types/Preview';
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
  frostLineAu,
  hillRadiusAu,
  hillShownRadius,
  sphereGrid,
  type SystemGiant,
  moonOrbits,
  gravityScale,
  orbitalSpeed,
  starGravity,
  systemCompressedFrom,
  systemScale,
  velocityScale,
} from './starSystem';

/** Size of a preview's texture on a marker, pixels (a marker is a few pixels to a few dozen wide). */
const PREVIEW_WIDTH = 64;
const PREVIEW_HEIGHT = 32;
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
  /** The star's centre on screen. */
  star: { x: number; y: number } | null;
  /** To scale, the tip of the star's pin (its label goes there); null otherwise. */
  starPin: { x: number; y: number } | null;
  /** A point of a planetary system's frost line, with its distance (AU), to label it; null when hidden. */
  frost: { x: number; y: number; au: number } | null;
  /**
   * A point of the dashed circle beyond which distances are compressed,
   * with its distance from the star (AU), to label it; null without one.
   */
  boundary: { x: number; y: number; au: number } | null;
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
    /** The star's pull on it now, m/s², and the middle of its vector on screen (null when hidden or off screen). */
    gravity: number;
    pull: { x: number; y: number } | null;
    /** To scale, the tip of its pin (its label goes there); null otherwise. */
    pin: { x: number; y: number } | null;
  }[];
}

/** Shortest time between two animated frames, ms (thirty frames a second at most). */
const FRAME_MS = 1000 / 30;
/** Longest step of the animation clock, s: a stall (a world loading, a hidden tab) pauses the motion instead of making it jump. */
const MAX_STEP_S = 0.1;
/** Seconds of animation for one orbit of the planet. */
export const YEAR_S = 60;
/** Mean anomaly of the planet when the animation clock reads zero, radians. */
const START_ANOMALY = 0.6;
/** How far out the farthest point of a system drawn to scale lies, view units. */
const REAL_SPAN = 9;
/** Kilometres in an AU. */
const AU_KM = 149_597_870.7;
/** Height of the pins up from the bodies drawn to scale, view units. */
const PIN_HEIGHT = 0.9;
/** The Sun's radius, AU. */
const SUN_RADIUS_AU = 0.004_65;
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
  /** Called after each frame with the world's axes as the camera sees them (the axes gizmo). */
  onAxes: (axes: ScreenAxis[]) => void = () => {};
  /** The camera's flight under way (see fly), or none. */
  private flight: { from: { position: Vector3; target: Vector3 }; to: { position: Vector3; target: Vector3 }; start: number; duration: number; done: () => void } | null = null;
  /** Called with a planet's index (in setStar's `giants`) when it is clicked; null ignores clicks. */
  onPickPlanet: ((index: number) => void) | null = null;
  /** Where a press started, to tell a click from a drag. */
  private pressAt: { x: number; y: number } | null = null;
  private disposed = false;
  /** The planet's moons around it (see setMoons), centred on the barycentre. */
  private readonly moonGroup = new Group();
  /** The giant planets of the system, their orbits and rings (see setStar). */
  private readonly giantGroup = new Group();
  private giants: SystemGiant[] = [];
  /** A planetary system's moons, round every planet: by planet, its group, orbits and markers. */
  private systemMoons: { group: Group; orbits: MoonOrbit[]; markers: Mesh<SphereGeometry, MeshBasicMaterial>[] }[] = [];
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
  private arrows: Line2[] = [];
  /** The star's pull on each planet: a vector from it towards the star, along its dotted line. */
  private readonly pullGroup = new Group();
  private pulls: Line2[] = [];
  /** The fat lines' materials, which need the view's size in pixels. */
  private fatMaterials: LineMaterial[] = [];
  /** The neutral colours of the overlays, from the theme (see setOverlayColours). */
  private overlay = { orbit: '#c8d2e6', selected: '#e6ecf8', line: '#ffe3a8', muted: '#8a93a8' };
  /** A planetary system's selected orbit, drawn thick over the thin one. */
  private readonly selectedOrbit: Line2;
  /** A planetary system's frost line (see frostLineAu), and its Hill spheres round the planets. */
  private readonly frost: Line<BufferGeometry, LineDashedMaterial>;
  private frostAu = 0;
  private frostOn = true;
  private readonly hillGroup = new Group();
  private hills: LineSegments<BufferGeometry, LineBasicMaterial | LineDashedMaterial>[] = [];
  /** To scale, the pins up from the bodies (see buildPins). */
  private readonly pinGroup = new Group();
  private pins: Line<BufferGeometry, LineBasicMaterial>[] = [];
  private starPinLine: Line<BufferGeometry, LineBasicMaterial> | null = null;
  private hillsOn = false;
  /** Whether a planetary system is drawn to scale: distances linear from the star, the star its true size. */
  private realScale = false;
  /** The star's drawn radius over STAR_RADIUS (1 but to scale). */
  private starScale = 1;
  /** The last setStar's arguments, to draw the system again on another scale. */
  private lastStar: [StarInfo, number, SystemGiant[], number | null] | null = null;
  private gravities: number[] = [];
  private pullMids: Vector3[] = [];
  /** View units per m/s² of the pull vectors (see gravityScale). */
  private pullScale = 0;
  private pullsOn = true;
  /** The giants' orbit lines, which Orbit hides in a planetary system. */
  private giantOrbits: Line<BufferGeometry, LineBasicMaterial>[] = [];
  /** Each planet's orbit drawn thick and bright, shown in place of its thin line while the planet is highlighted (see setHighlighted). */
  private orbitHighlights: Line2[] = [];
  /** The dashed circle where distances start being compressed, if any. */
  private boundaryAt: Vector3 | null = null;
  /** A planetary system's dashed line across the habitable zone, between its edges. */
  private readonly zoneLine: Line<BufferGeometry, LineDashedMaterial>;
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
  /** A ring round each giant's marker, facing the camera, shown while it is highlighted. */
  private halos: Mesh<RingGeometry, MeshBasicMaterial>[] = [];
  /** The giants highlighted, by index (see setHighlighted). */
  private highlighted: ReadonlySet<number> = new Set();
  /** The map wrapped round each giant's marker, by index, or null for its colour (see setPlanetMaps). */
  private mapUrls: readonly (string | null)[] = [];
  /** Each giant's preview as a texture, by index, or null (see setPlanetPreviews). */
  private previewTextures: (DataTexture | null)[] = [];
  /** The maps loaded (or loading), by url. */
  private readonly maps = new Map<string, Texture>();
  private readonly loader = new TextureLoader();
  /** The giant under the pointer, by index (see onHoverPlanet). */
  private hovered: number | null = null;
  /**
   * Called with the index of the giant the pointer comes over (in setStar's
   * `giants`), and with null when it leaves it; null ignores the pointer.
   * Touch has no hover: a tap is a click.
   */
  onHoverPlanet: ((index: number | null) => void) | null = null;
  /** Called with a giant's index when its marker is double-clicked; null ignores double clicks. */
  onOpenPlanet: ((index: number) => void) | null = null;
  /**
   * A planetary system's selected planet, by its index in `giants`, or
   * null for a lone world's view. With one, every planet is drawn as a
   * giant, on a scale and a clock that do not depend on the selection.
   */
  private selected: number | null = null;
  /** The planets of the system shown, to restart the clock only when they change. */
  private systemKey = '';
  /** Earth days per second of animation in a planetary system, at speed ×1 (one). */
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
    // The right button frees the camera: it pans on the screen's two axes.
    this.controls.enablePan = true;
    this.controls.screenSpacePanning = true;
    this.controls.mouseButtons = { LEFT: MOUSE.ROTATE, MIDDLE: MOUSE.DOLLY, RIGHT: MOUSE.PAN };
    canvas.addEventListener('contextmenu', (event) => event.preventDefault());
    // A press, the wheel or a key ends a flight at once (see fly).
    canvas.addEventListener('pointerdown', () => this.endFlight(), { capture: true });
    canvas.addEventListener('wheel', () => this.endFlight(), { capture: true, passive: true });
    window.addEventListener('keydown', this.endFlightOnKey);
    this.controls.minDistance = STAR_RADIUS * 1.4;
    this.controls.maxDistance = 80;
    this.controls.addEventListener('change', () => this.requestRender());
    // Only the user's drags and wheels start the controls, not resetView.
    this.controls.addEventListener('start', () => (this.userMoved = true));
    // A click on a planet's marker picks it (a drag turns the view).
    canvas.addEventListener('pointerdown', (event) => (this.pressAt = { x: event.clientX, y: event.clientY }));
    canvas.addEventListener('pointermove', (event) => {
      if (event.pointerType === 'touch' || !this.onHoverPlanet) return;
      // Not while a drag turns the view.
      this.hover(event.buttons === 0 ? this.pickPlanet(event.clientX, event.clientY) : null);
    });
    canvas.addEventListener('pointerleave', () => this.hover(null));
    canvas.addEventListener('dblclick', (event) => {
      if (!this.onOpenPlanet) return;
      const index = this.pickPlanet(event.clientX, event.clientY);
      if (index !== null) this.onOpenPlanet(index);
    });
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
    this.orbit = new Line(circle, new LineBasicMaterial({ color: this.overlay.orbit, transparent: true, opacity: 0.7 }));
    this.scene.add(this.orbit);
    this.planet = new Mesh(new SphereGeometry(PLANET_MARKER, 32, 16), new MeshBasicMaterial({ color: '#7fb2ff' }));
    this.scene.add(this.planet);
    this.scene.add(this.moonGroup);
    this.scene.add(this.giantGroup);
    this.scene.add(this.linkGroup);
    this.scene.add(this.arrowGroup);
    this.scene.add(this.pullGroup);
    this.scene.add(this.hillGroup);
    this.scene.add(this.pinGroup);
    this.selectedOrbit = new Line2(new LineGeometry(), this.fatMaterial(this.overlay.selected, 2.2, 0.9));
    this.selectedOrbit.renderOrder = 3;
    this.selectedOrbit.frustumCulled = false;
    this.selectedOrbit.visible = false;
    this.scene.add(this.selectedOrbit);
    this.frost = new Line(
      new BufferGeometry(),
      new LineDashedMaterial({ color: '#9cc8ff', dashSize: 0.1, gapSize: 0.08, transparent: true, opacity: 0.7 }),
    );
    this.frost.renderOrder = 3;
    this.frost.visible = false;
    this.scene.add(this.frost);
    this.zoneLine = new Line(
      new BufferGeometry().setFromPoints([new Vector3(), new Vector3()]),
      new LineDashedMaterial({ color: '#8fe0aa', dashSize: 0.05, gapSize: 0.05, transparent: true, opacity: 0.9 }),
    );
    this.zoneLine.renderOrder = 3;
    this.zoneLine.frustumCulled = false;
    this.zoneLine.visible = false;
    this.scene.add(this.zoneLine);
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
  setStar(info: StarInfo, omega = 0, giants: SystemGiant[] = [], selected: number | null = null): void {
    this.lastStar = [info, omega, giants, selected];
    // A planetary system, with a planet selected or none (-1).
    const system = selected !== null && selected < giants.length;
    const picked = system && selected >= 0;
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
    // A planetary system's clock counts Earth days: one a second at ×1.
    this.daysPerSecond = system ? 1 : 0;
    this.planet.visible = !system;
    // The selected planet's moons circle its own marker, as much larger as it is.
    this.moonGroup.scale.setScalar(picked ? giantMarkerSize(giants[selected]!.radius_earth) / PLANET_MARKER : 1);
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
    // To scale: linear from the star's centre, the farthest point REAL_SPAN out.
    const outer = Math.max(info.hz_outer_au, ...giants.map((g) => g.orbit_au * (1 + g.eccentricity)));
    const real = system && this.realScale;
    if (real) this.from = Infinity;
    const at = real ? (au: number) => (REAL_SPAN * au) / outer : systemScale(info, giants, 6, this.from);
    this.at = at;
    this.starScale = real ? Math.max((REAL_SPAN * SUN_RADIUS_AU * info.radius) / outer / STAR_RADIUS, 1e-4) : 1;
    this.star.scale.setScalar(this.starScale);
    this.arcs.scale.setScalar(this.starScale);
    // The glow is the light, not the size: it shrinks less, to stay visible.
    // To scale, the glow shrinks with the star.
    this.glow.scale.setScalar(real ? this.starScale : 1);
    this.controls.minDistance = STAR_RADIUS * (real ? 0.05 : 1.4);
    this.frostAu = frostLineAu(info.luminosity);
    this.frost.geometry.dispose();
    this.frost.geometry = new BufferGeometry().setFromPoints(
      Array.from({ length: 193 }, (_, i) => this.inPlaneAt((i / 192) * 2 * Math.PI, this.frostAu)),
    );
    this.frost.computeLineDistances();
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
    {
      const ends = this.zoneLine.geometry.getAttribute('position') as BufferAttribute;
      const inner = this.zoneEdge(info.hz_inner_au);
      const outer = this.zoneEdge(info.hz_outer_au);
      ends.setXYZ(0, inner.x, inner.y, inner.z);
      ends.setXYZ(1, outer.x, outer.y, outer.z);
      ends.needsUpdate = true;
      this.zoneLine.computeLineDistances();
    }
    // The ellipse, the star at a focus.
    const e = info.eccentricity ?? 0;
    // In a planetary system, the selected planet's own orbit, tilted as it is.
    const own = picked ? giants[selected] : undefined;
    const point = (nu: number, au: number) => (own ? this.orbitPoint(own, nu, au) : this.inPlane(nu, au));
    this.orbit.geometry.dispose();
    this.orbit.geometry = new BufferGeometry().setFromPoints(
      Array.from({ length: 361 }, (_, i) => point((i / 360) * 2 * Math.PI, info.orbit_au * orbitDistance(e, (i / 360) * 2 * Math.PI))),
    );
    this.apsides[0]!.position.copy(point(0, info.orbit_au * (1 - e)));
    this.apsides[1]!.position.copy(point(Math.PI, info.orbit_au * (1 + e)));
    this.buildSystemMoons();
    // To scale, the planets are their true size too (most of them under a
    // pixel: the pins and their labels show where they are), their moons hidden.
    this.giantMarkers.forEach((marker, k) => {
      const giant = giants[k]!;
      const trueSize = (REAL_SPAN * (giant.radius_earth * 6371)) / AU_KM / outer;
      marker.scale.setScalar(real ? Math.max(trueSize / giantMarkerSize(giant.radius_earth), 1e-4) : 1);
    });
    for (const moons of this.systemMoons) moons.group.visible = !real;
    this.buildPins(real);
    this.selectedOrbit.geometry.setPositions(
      own
        ? Array.from({ length: 361 }, (_, i) => point((i / 360) * 2 * Math.PI, info.orbit_au * orbitDistance(e, (i / 360) * 2 * Math.PI))).flatMap((p) => [p.x, p.y, p.z])
        : [0, 0, 0, 0, 0, 0],
    );
    this.buildHills();
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
    // A planetary system draws the moons of every planet (see buildSystemMoons).
    if (this.selected !== null) return;
    this.moonOrbits = orbits;
    for (const orbit of orbits) {
      const ring = new Line(
        new BufferGeometry().setFromPoints(
          Array.from({ length: 97 }, (_, i) => {
            const a = (i / 96) * 2 * Math.PI;
            return new Vector3(Math.cos(a) * orbit.radius, 0, -Math.sin(a) * orbit.radius);
          }),
        ),
        new LineBasicMaterial({ color: this.overlay.orbit, transparent: true, opacity: 0.35 }),
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

  /**
   * A planetary system's moons, round every planet's marker, scaled to it,
   * on the orbits `moonOrbits` gives, each at its real pace on the
   * system's clock.
   */
  private buildSystemMoons(): void {
    for (const moons of this.systemMoons) {
      for (const child of [...moons.group.children]) {
        const object = child as Mesh<BufferGeometry, MeshBasicMaterial>;
        object.geometry.dispose();
        object.material.dispose();
      }
      this.giantGroup.remove(moons.group);
    }
    this.systemMoons = [];
    if (this.selected === null) return;
    for (const giant of this.giants) {
      const group = new Group();
      group.scale.setScalar(giantMarkerSize(giant.radius_earth) / PLANET_MARKER);
      // A day a second at ×1: a year lasts its days in seconds.
      const orbits = moonOrbits(giant.moons ?? [], giant.radius_earth * 6371, giant.year_days, giant.year_days, false);
      const markers = orbits.map((orbit) => {
        const ring = new Line(
          new BufferGeometry().setFromPoints(
            Array.from({ length: 65 }, (_, i) => {
              const a = (i / 64) * 2 * Math.PI;
              return new Vector3(Math.cos(a) * orbit.radius, 0, -Math.sin(a) * orbit.radius);
            }),
          ),
          new LineBasicMaterial({ color: this.overlay.orbit, transparent: true, opacity: 0.3 }),
        );
        ring.rotation.x = orbit.inclination;
        ring.renderOrder = 3;
        group.add(ring);
        const marker = new Mesh(new SphereGeometry(orbit.size, 12, 6), new MeshBasicMaterial({ color: '#d9d4c8' }));
        group.add(marker);
        return marker;
      });
      this.giantGroup.add(group);
      this.systemMoons.push({ group, orbits, markers });
    }
  }

  /** Places a planetary system's moons round their planets at the system's time. */
  private placeSystemMoons(): void {
    this.systemMoons.forEach((moons, k) => {
      const marker = this.giantMarkers[k];
      if (!marker) return;
      moons.group.position.copy(marker.position);
      moons.orbits.forEach((orbit, m) => {
        const angle = orbit.phase + this.orbitTime * orbit.speed;
        const local = new Vector3(Math.cos(angle) * orbit.radius, 0, -Math.sin(angle) * orbit.radius);
        local.applyAxisAngle(new Vector3(1, 0, 0), orbit.inclination);
        moons.markers[m]?.position.copy(local);
      });
    });
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
    this.giantOrbits = [];
    this.fatMaterials = this.fatMaterials.filter((m) => !this.orbitHighlights.some((h) => h.material === m));
    this.orbitHighlights = [];
    this.boundaryAt = null;
    this.halos = [];
    const from = this.from;
    if (this.giants.some((g) => g.orbit_au * (1 + g.eccentricity) > from)) {
      const boundary = new Line(
        new BufferGeometry().setFromPoints(
          Array.from({ length: 193 }, (_, i) => this.inPlaneAt((i / 192) * 2 * Math.PI, from)),
        ),
        new LineDashedMaterial({ color: this.overlay.muted, dashSize: 0.12, gapSize: 0.12, transparent: true, opacity: 0.5 }),
      );
      boundary.computeLineDistances();
      // Labelled on the far side of the star, where the view sees it best.
      this.boundaryAt = this.inPlaneAt(Math.PI / 2, from);
      boundary.renderOrder = 3;
      this.giantGroup.add(boundary);
    }
    for (const giant of this.giants) {
      const colour = new Color().setRGB(...giant.colour, SRGBColorSpace);
      const orbit = new Line(
        new BufferGeometry().setFromPoints(
          Array.from({ length: 361 }, (_, i) => {
            const nu = (i / 360) * 2 * Math.PI;
            return this.orbitPoint(giant, nu, giant.orbit_au * orbitDistance(giant.eccentricity, nu));
          }),
        ),
        new LineBasicMaterial({ color: colour.clone().lerp(new Color(this.overlay.orbit), 0.5), transparent: true, opacity: 0.45 }),
      );
      orbit.renderOrder = 3;
      this.giantOrbits.push(orbit);
      this.giantGroup.add(orbit);
      const bright = new Line2(
        new LineGeometry().setPositions(Array.from(orbit.geometry.getAttribute('position').array)),
        this.fatMaterial(`#${colour.clone().lerp(new Color('#ffffff'), 0.35).getHexString()}`, 2.2, 0.95),
      );
      bright.renderOrder = 3;
      bright.frustumCulled = false;
      bright.visible = false;
      this.orbitHighlights.push(bright);
      this.giantGroup.add(bright);
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
      // Its highlight: a ring a little wider than the marker (and its rings).
      const halo = new Mesh(
        new RingGeometry(size * (giant.rings !== 'none' ? 2.45 : 1.45), size * (giant.rings !== 'none' ? 2.75 : 1.8), 48),
        new MeshBasicMaterial({ color: '#ffd27a', transparent: true, opacity: 0.9, side: DoubleSide, depthWrite: false }),
      );
      halo.renderOrder = 4;
      halo.visible = false;
      marker.add(halo);
      this.halos.push(halo);
      this.giantMarkers.push(marker);
      this.giantGroup.add(marker);
    }
    this.applyHighlights();
    this.applyMaps();
  }

  /** Highlights the giants of these indices (in setStar's `giants`) with a ring and their orbit drawn bright, and only them. */
  setHighlighted(indices: readonly number[]): void {
    this.highlighted = new Set(indices);
    this.applyHighlights();
    this.requestRender();
  }

  private applyHighlights(): void {
    this.halos.forEach((halo, k) => (halo.visible = this.highlighted.has(k)));
    this.applyVisibility();
  }

  /**
   * Wraps a map round giant markers: `urls[k]`, an equirectangular image,
   * round giant `k`'s, its colour where it is null or still loading.
   * Maps no longer listed are let go.
   */
  setPlanetMaps(urls: readonly (string | null)[]): void {
    this.mapUrls = urls;
    const wanted = new Set(urls.filter((url): url is string => url !== null));
    for (const [url, map] of this.maps) {
      if (!wanted.has(url)) {
        map.dispose();
        this.maps.delete(url);
      }
    }
    for (const url of wanted) {
      if (this.maps.has(url)) continue;
      const map = this.loader.load(url, () => {
        // Loaded: shown, unless let go meanwhile.
        if (this.maps.get(url) !== map || this.disposed) return;
        this.applyMaps();
        this.requestRender();
      });
      map.colorSpace = SRGBColorSpace;
      this.maps.set(url, map);
    }
    this.applyMaps();
    this.requestRender();
  }

  /**
   * Wraps a preview round giant markers: `previews[k]`, giant `k`'s preview
   * (its belts, land and caps), shown until its map is (see setPlanetMaps);
   * null leaves its plain colour.
   */
  setPlanetPreviews(previews: readonly (Preview | null)[]): void {
    for (const texture of this.previewTextures) texture?.dispose();
    this.previewTextures = previews.map((preview) => {
      if (!preview) return null;
      const texture = new DataTexture(previewTexture(preview, PREVIEW_WIDTH, PREVIEW_HEIGHT), PREVIEW_WIDTH, PREVIEW_HEIGHT);
      texture.colorSpace = SRGBColorSpace;
      texture.needsUpdate = true;
      return texture;
    });
    this.applyMaps();
    this.requestRender();
  }

  private applyMaps(): void {
    this.giantMarkers.forEach((marker, k) => {
      const url = this.mapUrls[k] ?? null;
      const map = url ? this.maps.get(url) : undefined;
      // A texture is ready once its image has loaded; until then, or
      // without a map, the planet's preview.
      const ready = (map && map.image ? map : null) ?? this.previewTextures[k] ?? null;
      if (marker.material.map === ready) return;
      marker.material.map = ready;
      // The map shows its own colours; without one, the planet's colour.
      if (ready) marker.material.color.set('#ffffff');
      else {
        const giant = this.giants[k];
        if (giant) marker.material.color.setRGB(...giant.colour, SRGBColorSpace);
      }
      marker.material.needsUpdate = true;
    });
  }

  /** The pointer is over giant `index` now (null: over none). */
  private hover(index: number | null): void {
    if (index === this.hovered) return;
    this.hovered = index;
    this.canvas.style.cursor = index === null ? '' : 'pointer';
    this.onHoverPlanet?.(index);
  }

  /**
   * A planetary system's dotted star–planet lines and velocity vectors,
   * one each per planet; none for a lone world.
   */
  private buildVectors(): void {
    for (const group of [this.linkGroup, this.arrowGroup, this.pullGroup]) {
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
    this.pulls = [];
    this.gravities = [];
    this.pullMids = [];
    this.fatMaterials = this.fatMaterials.filter((m) => m === this.selectedOrbit.material || this.orbitHighlights.some((h) => h.material === m));
    if (this.selected === null || !this.info) return;
    this.arrowScale = velocityScale(this.info.mass, this.giants);
    this.pullScale = gravityScale(this.info.mass, this.giants);
    for (const giant of this.giants) {
      const colour = new Color().setRGB(...giant.colour, SRGBColorSpace).lerp(new Color(this.overlay.line), 0.6);
      const link = new Line(
        new BufferGeometry().setFromPoints([new Vector3(), new Vector3()]),
        new LineDashedMaterial({ color: colour, dashSize: 0.06, gapSize: 0.08, transparent: true, opacity: 0.6 }),
      );
      link.renderOrder = 3;
      link.frustumCulled = false;
      this.planetLinks.push(link);
      this.linkGroup.add(link);
      // The shaft, then the two barbs of the head: tip, barb, tip, barb.
      const arrow = new Line2(new LineGeometry(), this.fatMaterial('#9fe0ff', 2.5, 0.95));
      arrow.renderOrder = 3;
      arrow.frustumCulled = false;
      this.arrows.push(arrow);
      this.arrowGroup.add(arrow);
      const pull = new Line2(new LineGeometry(), this.fatMaterial('#ffa07a', 2.5, 0.95));
      pull.renderOrder = 3;
      pull.frustumCulled = false;
      this.pulls.push(pull);
      this.pullGroup.add(pull);
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
   * Recolours the neutral overlays (orbit lines, the selected orbit, the
   * dotted lines, moon rings, the compression circle) with a theme's
   * colours; the colours that carry meaning (zone, frost line, arrows)
   * stay as they are.
   */
  setOverlayColours(colours: { orbit: string; selected: string; line: string; muted: string }): void {
    this.overlay = { ...colours };
    this.orbit.material.color.set(colours.orbit);
    this.selectedOrbit.material.color.set(colours.selected);
    if (this.lastStar) this.setStar(...this.lastStar);
    else this.requestRender();
  }

  /**
   * To scale: a vertical line up from the star and from each planet, at
   * whose tip its label goes (the bodies themselves are mostly too small to
   * see); none otherwise.
   */
  private buildPins(on: boolean): void {
    for (const pin of [...this.pins, ...(this.starPinLine ? [this.starPinLine] : [])]) {
      pin.geometry.dispose();
      pin.material.dispose();
      this.pinGroup.remove(pin);
    }
    this.pins = [];
    this.starPinLine = null;
    if (!on) return;
    const line = (from: Vector3) => {
      const pin = new Line(
        new BufferGeometry().setFromPoints([from, from.clone().setY(from.y + PIN_HEIGHT)]),
        new LineBasicMaterial({ color: this.overlay.muted, transparent: true, opacity: 0.8 }),
      );
      pin.frustumCulled = false;
      pin.renderOrder = 3;
      this.pinGroup.add(pin);
      return pin;
    };
    this.starPinLine = line(new Vector3());
    this.pins = this.giants.map(() => line(new Vector3()));
  }

  /** Shows or hides a planetary system's vectors of the star's pull on its planets. */
  setFrostLine(on: boolean): void {
    this.frostOn = on;
    this.applyVisibility();
    this.requestRender();
  }

  /** Shows or hides a circle round each planet of a planetary system: its Hill sphere (see hillRadiusAu), as far as its moons can be held. */
  setHillSpheres(on: boolean): void {
    this.hillsOn = on;
    this.applyVisibility();
    this.requestRender();
  }

  /**
   * Draws a planetary system to scale, or not: distances linear from the
   * star's centre and the star its true size (the planets' markers stay
   * larger than to scale), else as usual. The view is fitted again.
   */
  setRealScale(on: boolean): void {
    if (on === this.realScale) return;
    this.realScale = on;
    if (this.lastStar) {
      this.userMoved = false;
      this.setStar(...this.lastStar);
      this.resetView();
    }
  }

  /** Earth days since a planetary system's clock started (see setSystemDay). */
  get systemDay(): number {
    return this.orbitTime * this.daysPerSecond;
  }

  /** Sets a planetary system's clock to `days` Earth days since its start, every planet where it then is. */
  setSystemDay(days: number): void {
    if (this.selected === null || this.daysPerSecond <= 0) return;
    this.orbitTime = Math.max(0, days) / this.daysPerSecond;
    this.placePlanet();
    const turn = this.meanAnomaly() / (2 * Math.PI);
    this.onClock(turn - Math.floor(turn), this.systemDay);
    this.requestRender();
  }

  /** The day (see systemDay) planet `index` next passes its periapsis, after now. */
  nextPeriapsisDay(index: number): number | null {
    const giant = this.giants[index];
    if (!giant || this.selected === null) return null;
    const turns = giant.phase + this.systemDay / giant.year_days;
    return (Math.floor(turns + 1e-9) + 1 - giant.phase) * giant.year_days;
  }

  /** A fat line's material, `width` CSS px wide. */
  private fatMaterial(colour: string, width: number, opacity: number): LineMaterial {
    const material = new LineMaterial({ color: new Color(colour).getHex(), linewidth: width, transparent: true, opacity, depthWrite: false });
    material.resolution.set(Math.max(this.canvas.clientWidth, 1), Math.max(this.canvas.clientHeight, 1));
    this.fatMaterials.push(material);
    return material;
  }

  /**
   * A grid sphere round each planet of a planetary system, as wide as its
   * Hill sphere on the view's scale, or dashed just outside its marker and
   * its moons' rings where those are drawn larger than that (see
   * hillShownRadius).
   */
  private buildHills(): void {
    for (const hill of this.hills) {
      hill.geometry.dispose();
      hill.material.dispose();
      this.hillGroup.remove(hill);
    }
    this.hills = [];
    if (this.selected === null || !this.info) return;
    this.giants.forEach((giant, k) => {
      const a = giant.orbit_au * (1 - giant.eccentricity);
      const hill = hillRadiusAu(giant.orbit_au, giant.eccentricity, giant.mass_earth, this.info!.mass);
      // Its width on the view's scale where the planet is (the scale may be
      // compressed there).
      const true_ = (this.at(a + hill) - this.at(Math.max(a - hill, 0))) / 2;
      // Its moons' rings, round the marker and scaled to it (hidden to scale).
      const moons = this.systemMoons[k];
      const reach =
        moons && !this.realScale ? Math.max(0, ...moons.orbits.map((o) => o.radius)) * moons.group.scale.x : 0;
      const { radius, toScale } = hillShownRadius(true_, giantMarkerSize(giant.radius_earth), reach);
      const sphere = new LineSegments(
        new BufferGeometry().setFromPoints(sphereGrid(radius).map(([x, y, z]) => new Vector3(x, y, z))),
        toScale
          ? new LineBasicMaterial({ color: '#c9b6ff', transparent: true, opacity: 0.4, depthWrite: false })
          : new LineDashedMaterial({ color: '#c9b6ff', dashSize: radius * 0.08, gapSize: radius * 0.06, transparent: true, opacity: 0.4, depthWrite: false }),
      );
      sphere.computeLineDistances();
      sphere.renderOrder = 3;
      sphere.frustumCulled = false;
      this.hills.push(sphere);
      this.hillGroup.add(sphere);
    });
  }

  setPulls(on: boolean): void {
    this.pullsOn = on;
    this.applyVisibility();
    this.requestRender();
  }

  /** An arrow's five points: its shaft from `from` along `direction`, `length` long, then its head's two barbs. */
  private static arrowPoints(from: Vector3, direction: Vector3, length: number): Vector3[] {
    const tip = from.clone().addScaledVector(direction, length);
    const barb = Math.min(0.06, length * 0.35);
    const side = new Vector3(-direction.z, 0, direction.x).multiplyScalar(barb * 0.6);
    const back = tip.clone().addScaledVector(direction, -barb);
    return [from, tip, back.clone().add(side), tip, back.clone().sub(side)];
  }

  /** Where the habitable zone's edge `au` is crossed by its dashed line: a fixed direction, whichever planet is selected. */
  private zoneEdge(au: number): Vector3 {
    return this.selected !== null ? this.inPlaneAt(Math.PI / 2, au) : this.inPlane(Math.PI / 2, au);
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
      return this.orbitPoint(giant, trueAnomaly, giant.orbit_au * distance);
    };
    const mean = this.giantAnomaly(k);
    const direction = at(mean + 1e-3).sub(at(mean - 1e-3)).normalize();
    const length = speed * this.arrowScale;
    arrow.geometry.setPositions(StarView.arrowPoints(position, direction, length).flatMap((p) => [p.x, p.y, p.z]));
    this.arrowMids[k] = position.clone().addScaledVector(direction, length / 2);
    // The star's pull, towards the star; never longer than the way to it,
    // and long enough to be seen.
    const pull = this.pulls[k];
    if (!pull) return;
    const gravity = starGravity(this.info.mass, distanceAu);
    this.gravities[k] = gravity;
    const inward = position.clone().negate();
    const room = Math.max(inward.length() - STAR_RADIUS * this.starScale * 1.2, 0.02);
    inward.normalize();
    const pullLength = Math.min(Math.max(gravity * this.pullScale, 0.04), room);
    pull.geometry.setPositions(StarView.arrowPoints(position, inward, pullLength).flatMap((p) => [p.x, p.y, p.z]));
    this.pullMids[k] = position.clone().addScaledVector(inward, pullLength / 2);
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
      const position = this.orbitPoint(giant, trueAnomaly, giant.orbit_au * distance);
      this.giantMarkers[k]?.position.copy(position);
      this.placeVector(k, giant, position, giant.orbit_au * distance);
      const pin = this.pins[k];
      if (pin) {
        const ends = pin.geometry.getAttribute('position') as BufferAttribute;
        ends.setXYZ(0, position.x, position.y, position.z);
        ends.setXYZ(1, position.x, position.y + PIN_HEIGHT, position.z);
        ends.needsUpdate = true;
      }
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
   * How fast a planetary system's planets go round, in Earth days per
   * second (see defaultSystemSpeed); the star keeps boiling at its own
   * pace. A lone world's view ignores it.
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
    // In a planetary system, Orbit hides every planet's orbit line.
    // A highlighted planet's orbit is drawn bright in place of its thin
    // line; the selected one's is the thick selected orbit.
    const orbitsOn = this.selected === null || this.orbitOn;
    this.giantOrbits.forEach((line, k) => {
      const bright = this.highlighted.has(k) && k !== this.selected;
      line.visible = orbitsOn && !bright;
      const highlight = this.orbitHighlights[k];
      if (highlight) highlight.visible = orbitsOn && bright;
    });
    this.zoneLine.visible = this.selected !== null && this.zoneOn && this.distancesOn;
    // A planetary system draws a dotted line to every planet instead.
    this.link.visible = this.distancesOn && this.selected === null;
    const system = this.selected !== null && this.giantsOn;
    this.linkGroup.visible = system && this.linksOn;
    this.arrowGroup.visible = system && this.velocitiesOn;
    this.pullGroup.visible = system && this.pullsOn;
    this.hillGroup.visible = system && this.hillsOn;
    this.pinGroup.visible = system && this.giantsOn;
    this.frost.visible = this.selected !== null && this.frostOn;
    // A planetary system's selected orbit is drawn thick.
    this.selectedOrbit.visible = this.picked() && this.orbitOn;
    if (this.selected !== null) this.orbit.visible = false;
    // A planetary system with none selected has no apsides to mark.
    const anyPlanet = this.selected === null || this.selected >= 0;
    for (const marker of this.apsides) marker.visible = this.distancesOn && this.orbitOn && anyPlanet;
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
    if (this.selected !== null) return this.selected >= 0 ? this.giantAnomaly(this.selected) : 0;
    return START_ANOMALY + this.time * ((2 * Math.PI) / YEAR_S);
  }

  /** Back to the starting view: the whole system, seen from above its plane. */
  resetView(): void {
    this.endFlight();
    const pose = this.fittedPose();
    this.userMoved = false;
    this.camera.position.copy(pose.position);
    this.controls.target.copy(pose.target);
    this.controls.update();
    this.requestRender();
  }

  /** The starting view: the whole system, seen from above its plane, the target on the star. */
  private fittedPose(): { position: Vector3; target: Vector3 } {
    // Far enough for the system (out to about 7.6 star radii) to fit the
    // narrower side of the view.
    const halfFov = ((this.camera.fov / 2) * Math.PI) / 180;
    const distance = ((this.reach + 1.4) / Math.tan(halfFov)) / Math.min(1, this.camera.aspect);
    // Room to zoom out past the fitted view, however far the system reaches.
    this.controls.maxDistance = Math.max(80, distance * 2);
    return { position: new Vector3(0, distance * 0.55, distance * 0.83), target: new Vector3() };
  }

  /** Where the camera stops close to planet `index`: in front of it, on the side the camera looks from. */
  private nearPose(index: number): { position: Vector3; target: Vector3 } | null {
    const marker = this.giantMarkers[index];
    if (!marker) return null;
    const target = marker.position.clone();
    const away = this.camera.position.clone().sub(target);
    if (away.lengthSq() < 1e-9) away.set(0, 1, 1);
    const size = giantMarkerSize(this.giants[index]?.radius_earth ?? 1);
    return { position: target.clone().addScaledVector(away.normalize(), size * 6), target };
  }

  /**
   * Flies the camera to planet `index` (about a second, eased), then calls
   * `done`: the travelling into a body's view. A press, the wheel or a key
   * ends it at once. With no such planet, `done` runs at once.
   */
  flyToPlanet(index: number, done: () => void): void {
    const to = this.nearPose(index);
    if (!to) {
      done();
      return;
    }
    this.fly(to, 1000, done);
  }

  /** Starts close to planet `index` and flies out to the whole system: the travelling back from a body. */
  flyOutFrom(index: number): void {
    const from = this.nearPose(index);
    if (!from) return;
    this.endFlight();
    this.camera.position.copy(from.position);
    this.controls.target.copy(from.target);
    this.fly(this.fittedPose(), 1100, () => (this.userMoved = false));
  }

  private fly(to: { position: Vector3; target: Vector3 }, duration: number, done: () => void): void {
    this.endFlight();
    this.controls.enabled = false;
    this.flight = {
      from: { position: this.camera.position.clone(), target: this.controls.target.clone() },
      to,
      start: performance.now(),
      duration,
      done,
    };
    this.requestRender();
  }

  /** Whether a planet of the planetary system shown is selected. */
  private picked(): boolean {
    return this.selected !== null && this.selected >= 0;
  }

  /** Ends the flight under way, the camera where it was going. */
  private endFlight(): void {
    const flight = this.flight;
    if (!flight) return;
    this.flight = null;
    this.camera.position.copy(flight.to.position);
    this.controls.target.copy(flight.to.target);
    this.controls.enabled = true;
    this.controls.update();
    flight.done();
    this.requestRender();
  }

  private readonly endFlightOnKey = (): void => this.endFlight();

  /** Moves the camera along the flight, eased in and out; true while it goes on. */
  private stepFlight(now: number): boolean {
    const flight = this.flight;
    if (!flight) return false;
    const u = Math.min(1, (now - flight.start) / flight.duration);
    const eased = u < 0.5 ? 4 * u * u * u : 1 - (-2 * u + 2) ** 3 / 2;
    this.camera.position.lerpVectors(flight.from.position, flight.to.position, eased);
    this.controls.target.lerpVectors(flight.from.target, flight.to.target, eased);
    this.camera.lookAt(this.controls.target);
    if (u >= 1) {
      this.endFlight();
      return false;
    }
    return true;
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
    window.removeEventListener('keydown', this.endFlightOnKey);
    this.flight = null;
    for (const object of [this.selectedOrbit, this.frost]) {
      object.geometry.dispose();
      object.material.dispose();
    }
    for (const group of [this.giantGroup, this.linkGroup, this.arrowGroup, this.pullGroup, this.hillGroup, this.pinGroup]) group.traverse((object) => {
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
    for (const map of this.maps.values()) map.dispose();
    for (const texture of this.previewTextures) texture?.dispose();
    this.maps.clear();
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

  /**
   * A point of giant `giant`'s orbit at a true anomaly and distance (AU),
   * in view units: in the reference plane, or tilted by its inclination
   * about its line of nodes (a planetary system's planets, see
   * SystemGiant), the distance compressed as the view's.
   */
  private orbitPoint(giant: SystemGiant, trueAnomaly: number, au: number): Vector3 {
    const tilt = giant.inclination ?? 0;
    if (tilt === 0) return this.inPlaneAt(trueAnomaly + giant.periapsis_angle, au);
    const node = giant.node ?? 0;
    // The angle from the ascending node in the orbit's plane.
    const u = trueAnomaly + giant.periapsis_angle - node;
    const x = Math.cos(node) * Math.cos(u) - Math.sin(node) * Math.sin(u) * Math.cos(tilt);
    const y = Math.sin(node) * Math.cos(u) + Math.cos(node) * Math.sin(u) * Math.cos(tilt);
    const z = Math.sin(u) * Math.sin(tilt);
    const r = this.at(au);
    // The reference plane is the view's (x, -z) plane, its north the view's up.
    return new Vector3(x * r, z * r, -y * r);
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
    this.placeSystemMoons();
    this.hills.forEach((hill, k) => {
      const marker = this.giantMarkers[k];
      if (marker) hill.position.copy(marker.position);
    });
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
    for (const material of this.fatMaterials) material.resolution.set(clientWidth, clientHeight);
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
    const flying = this.stepFlight(performance.now());
    const moving = flying || (this.flight === null && this.controls.update());
    // The glow faces the camera.
    this.glow.quaternion.copy(this.camera.quaternion);
    // So do the highlights (their markers do not turn).
    for (const halo of this.halos) if (halo.visible) halo.quaternion.copy(this.camera.quaternion);
    this.renderer.render(this.scene, this.camera);
    this.onAxes(axesOnScreen(this.camera.quaternion));
    const giantsShown = this.giantsOn && this.giants.length > 0;
    if ((this.distancesOn || giantsShown) && this.info) {
      // The zone's edges are labelled across the apsides line, away from
      // the periapsis and apoapsis labels.
      const edge = (au: number) => this.screen(this.zoneEdge(au));
      const anyPlanet = this.selected === null || this.selected >= 0;
      const planet = this.distancesOn && anyPlanet ? this.screen(this.planet.position) : null;
      const on = this.distancesOn;
      this.onLabels({
        planet: planet && { ...planet, au: this.planetAu },
        // A planetary system labels the selected planet as the others.
        self: on || this.selected !== null ? null : this.screen(this.planet.position),
        periapsis: on && this.orbitOn && anyPlanet ? this.screen(this.apsides[0]!.position) : null,
        apoapsis: on && this.orbitOn && anyPlanet ? this.screen(this.apsides[1]!.position) : null,
        inner: on && this.zoneOn ? edge(this.info.hz_inner_au) : null,
        outer: on && this.zoneOn ? edge(this.info.hz_outer_au) : null,
        star: this.screen(new Vector3()),
        starPin: this.pins.length > 0 ? this.screen(new Vector3(0, PIN_HEIGHT, 0)) : null,
        frost: this.frost.visible
          ? (() => {
              const at = this.screen(this.inPlaneAt(-Math.PI / 2, this.frostAu));
              return at && { ...at, au: this.frostAu };
            })()
          : null,
        boundary:
          this.boundaryAt && this.giantsOn && this.selected !== null
            ? (() => {
                const at = this.screen(this.boundaryAt);
                return at && { ...at, au: this.from };
              })()
            : null,
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
                      gravity: this.gravities[k] ?? 0,
                      pull: this.pullGroup.visible && this.pullMids[k] ? this.screen(this.pullMids[k]) : null,
                      pin: this.pins[k] ? this.screen(this.giantMarkers[k]!.position.clone().setY(this.giantMarkers[k]!.position.y + PIN_HEIGHT)) : null,
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
