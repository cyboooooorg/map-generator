// What the planet view draws about the body shown's own motion, on the
// globe (see PlanetView): its spin, as a solid curved arrow round its axis
// (or, for a body locked to what it orbits, a closed ring: no spin of its
// own against it), and in a planetary system, its host planet: a dotted
// line to it, the host's pull on the body and the body's orbital velocity
// as arrows, drawn as the spin arrow is (a tube and a cone). Positions are
// in radii of the body shown, in its frame.
import {
  BufferGeometry,
  Color,
  ConeGeometry,
  CatmullRomCurve3,
  CylinderGeometry,
  Group,
  Line,
  LineDashedMaterial,
  Mesh,
  MeshBasicMaterial,
  type Object3D,
  TorusGeometry,
  TubeGeometry,
  Vector3,
} from 'three';
import type { Vec3 } from '../skyBodies';

/** Where the arrow round the axis sits, its radius and its arc (radians), its tube's radius. */
const SPIN_HEIGHT = 1.3;
const SPIN_RADIUS = 0.3;
const SPIN_ARC = 1.5 * Math.PI;
const SPIN_TUBE = 0.022;
/** The arrows of the host's pull and the body's velocity: their length beyond the surface, in radii. */
const GUIDE_LENGTH = 1.4;
/** Points of the host line, spaced geometrically from the surface to the host (see hostLineRadii). */
const HOST_LINE_POINTS = 96;
/** Fixed hues of meaning (see the redesign spec): velocity cyan, pull orange, the host line pale. */
export const VELOCITY_COLOUR = new Color('#4fd1ff');
export const PULL_COLOUR = new Color('#ff9a3d');
const HOST_LINE_COLOUR = new Color('#c8d2e6');

/** The host planet as the guides draw it: its centre, and the body's direction of motion round it (or null). */
export interface HostGuide {
  position: Vec3;
  velocity: Vec3 | null;
}

/**
 * The points the labels hang from, in the scene's frame (see
 * PlanetView.skyScreenPositions). The host's distance has no fixed point:
 * it goes on whatever part of the host line is in view, among `host`.
 */
export interface GuidePoints {
  spin: Vector3 | null;
  host: Vector3[] | null;
  pull: Vector3 | null;
  velocity: Vector3 | null;
}

/**
 * Distances from the body's centre, in its radii, of the host line's
 * points: from the surface (1) to the host (`distance`), spaced
 * geometrically so that the stretch near the body, the one in view when
 * the camera is close, is drawn and labelled as finely as the rest.
 */
export function hostLineRadii(distance: number, count = HOST_LINE_POINTS): number[] {
  const far = Math.max(distance, 1);
  return Array.from({ length: count }, (_, i) => far ** (i / (count - 1)));
}

/**
 * The index of the point to label among a line's points, given which are
 * in view: the middle of the longest run in view, or -1 if none is. The
 * label then stays on the line for as long as any of it shows.
 */
export function labelIndex(inView: boolean[]): number {
  let best = -1;
  let bestLength = 0;
  let start = -1;
  inView.forEach((shown, i) => {
    if (shown && start < 0) start = i;
    if (start >= 0 && (!shown || i === inView.length - 1)) {
      const end = shown ? i : i - 1;
      if (end - start + 1 > bestLength) {
        bestLength = end - start + 1;
        best = Math.floor((start + end) / 2);
      }
      start = -1;
    }
  });
  return best;
}

/** Where the host line's distance label prefers to sit, and how far out it may go, in radii of the body from its centre. */
export const HOST_LABEL_AT = 2.5;
export const HOST_LABEL_REACH = 12;

/**
 * Which point of the host line (`radii` from the body's centre, nearest
 * first) carries its distance label, given which are in view: a fixed
 * point near the body (`HOST_LABEL_AT`) while it shows, so that the label
 * holds still as the camera moves; else the middle of the longest run in
 * view within `HOST_LABEL_REACH`, where the line is still spread out on
 * the screen; else anywhere along it (see labelIndex); -1 when none is in
 * view.
 */
export function hostLabelIndex(radii: readonly number[], inView: readonly boolean[]): number {
  const fixed = radii.findIndex((r) => r >= HOST_LABEL_AT);
  if (fixed >= 0 && inView[fixed]) return fixed;
  const near = labelIndex(inView.map((shown, i) => shown && radii[i]! <= HOST_LABEL_REACH));
  return near >= 0 ? near : labelIndex([...inView]);
}

export class BodyGuides {
  /** Turns with the stars, as the spin axis does (see PlanetView.setStarsTurn). */
  readonly spin = new Group();
  /** In the body's frame, as the sky bodies are. */
  readonly host = new Group();
  /** The host's pull and the body's velocity, each starting from the surface where it points (see setSurface). */
  private readonly arrows = new Group();
  private pull: { arrow: Group; direction: Vector3 } | null = null;
  private velocity: { arrow: Group; direction: Vector3 } | null = null;
  /** How far the surface reaches in a direction of the body's frame, in its radii (1: a sphere). */
  private surfaceAt: (direction: Vector3) => number = () => 1;
  private spinTip = new Vector3();
  private hostLine: Vector3[] | null = null;
  private pullTip: Vector3 | null = null;
  private velocityTip: Vector3 | null = null;
  private locked: boolean | null = null;

  /**
   * `prepare` adjusts each material the guides make (the host planet's
   * spin, drawn far away in a moon's sky, holds its depth at the far plane
   * and hides behind the planet; see PlanetView.setHostSpin).
   */
  constructor(
    private readonly colour: Color,
    private readonly prepare: (material: MeshBasicMaterial) => void = () => {},
  ) {
    this.spin.visible = false;
    this.spin.renderOrder = 2;
    this.host.visible = false;
    this.host.add(this.arrows);
  }

  /**
   * How far out the body's surface reaches: `poles` round its poles, and
   * `at` in any direction of its frame, in its radii (an irregular moon's
   * shape with its relief, a few tenths over 1 on a moonlet a few km
   * across). The spin arrow, scaled by `poles`, and the arrows of the
   * host's pull and of the body's velocity start from there rather than
   * inside the body.
   */
  setSurface(poles: number, at: (direction: Vector3) => number): void {
    this.surfaceAt = at;
    this.spin.scale.setScalar(poles);
    for (const guide of [this.pull, this.velocity]) {
      if (guide) guide.arrow.position.copy(guide.direction).multiplyScalar(at(guide.direction));
    }
    if (this.pull) this.pullTip = this.tipOf(this.pull.direction);
    if (this.velocity) this.velocityTip = this.tipOf(this.velocity.direction);
  }

  /** The end of an arrow along `direction`, from the surface there. */
  private tipOf(direction: Vector3): Vector3 {
    return direction.clone().multiplyScalar(this.surfaceAt(direction) + GUIDE_LENGTH);
  }

  /**
   * The spin depiction: an arrow bent round the axis near its north end,
   * going the way the body turns, or a closed ring for a locked body; null
   * removes it.
   */
  setSpin(locked: boolean | null): void {
    if (locked === this.locked) return;
    this.locked = locked;
    clear(this.spin);
    if (locked === null) return;
    const material = new MeshBasicMaterial({ color: this.colour, transparent: true });
    this.prepare(material);
    if (locked) {
      const ring = new Mesh(new TorusGeometry(SPIN_RADIUS, SPIN_TUBE, 8, 64), material);
      ring.rotation.x = Math.PI / 2;
      ring.position.y = SPIN_HEIGHT;
      this.spin.add(ring);
      this.spinTip.set(0, SPIN_HEIGHT, SPIN_RADIUS);
      return;
    }
    const at = (angle: number) => new Vector3(SPIN_RADIUS * Math.sin(angle), SPIN_HEIGHT, SPIN_RADIUS * Math.cos(angle));
    const points = Array.from({ length: 33 }, (_, i) => at((SPIN_ARC * i) / 32));
    this.spin.add(new Mesh(new TubeGeometry(new CatmullRomCurve3(points), 48, SPIN_TUBE, 8, false), material));
    // Its head, a rounded cone at the end of the arc, pointing on along it.
    const tip = at(SPIN_ARC);
    const along = new Vector3(Math.cos(SPIN_ARC), 0, -Math.sin(SPIN_ARC));
    const head = new Mesh(new ConeGeometry(SPIN_TUBE * 3, SPIN_TUBE * 7, 16), material);
    head.position.copy(tip).addScaledVector(along, SPIN_TUBE * 2.5);
    head.quaternion.setFromUnitVectors(new Vector3(0, 1, 0), along);
    this.spin.add(head);
    this.spinTip.copy(tip);
  }

  /** The host planet's guides, or none (null). */
  setHost(guide: HostGuide | null): void {
    clear(this.arrows);
    for (const child of [...this.host.children]) if (child !== this.arrows) clearOne(this.host, child);
    this.hostLine = this.pullTip = this.velocityTip = null;
    this.pull = this.velocity = null;
    if (!guide) return;
    const to = new Vector3(...guide.position);
    const distance = to.length();
    if (distance < 1e-6) return;
    const toward = to.clone().divideScalar(distance);
    // A dotted line from the surface to the host's centre, whole: the
    // host lies far beyond the camera's far plane, which only bounds the
    // globe's depth, so the line's depth is held at that plane instead of
    // being cut there (see clampToFarPlane).
    this.hostLine = hostLineRadii(distance).map((r) => toward.clone().multiplyScalar(r));
    const material = new LineDashedMaterial({
      color: HOST_LINE_COLOUR,
      dashSize: 0.25,
      gapSize: 0.25,
      transparent: true,
      opacity: 0.75,
    });
    clampToFarPlane(material);
    const line = new Line(new BufferGeometry().setFromPoints(this.hostLine), material);
    line.computeLineDistances();
    line.frustumCulled = false;
    this.host.add(line);
    // The host's pull, from the surface towards it.
    const pull = arrow(toward.clone().multiplyScalar(this.surfaceAt(toward)), toward, GUIDE_LENGTH, PULL_COLOUR);
    this.arrows.add(pull);
    this.pull = { arrow: pull, direction: toward };
    this.pullTip = this.tipOf(toward);
    if (guide.velocity) {
      const v = new Vector3(...guide.velocity).normalize();
      const velocity = arrow(v.clone().multiplyScalar(this.surfaceAt(v)), v, GUIDE_LENGTH, VELOCITY_COLOUR);
      this.arrows.add(velocity);
      this.velocity = { arrow: velocity, direction: v };
      this.velocityTip = this.tipOf(v);
    }
  }

  /** Shows them, faded with the globe (`fade`, 0 on the flat map). */
  update(spinOn: boolean, hostOn: boolean, fade: number): void {
    this.spin.visible = spinOn && this.locked !== null && fade > 0.01;
    this.host.visible = hostOn && fade > 0.01;
    for (const group of [this.spin, this.host]) {
      group.traverse((object) => {
        const material = (object as Mesh).material as MeshBasicMaterial | undefined;
        if (material) material.opacity = fade ** 3 * (object instanceof Line ? 0.75 : 1);
      });
    }
  }

  /** The points their labels hang from, in the scene's frame; null for those not shown. */
  points(): GuidePoints {
    const scene = (v: Vector3 | null, group: Group) => (v && group.visible ? group.localToWorld(v.clone()) : null);
    return {
      spin: scene(this.spinTip, this.spin),
      host: this.hostLine && this.host.visible ? this.hostLine.map((p) => this.host.localToWorld(p.clone())) : null,
      pull: scene(this.pullTip, this.arrows),
      velocity: scene(this.velocityTip, this.arrows),
    };
  }

  dispose(): void {
    clear(this.spin);
    clear(this.arrows);
    clear(this.host);
  }
}

/**
 * An arrow from the surface point `from` along `direction`,
 * `length` long in all: a tube and a cone, as the spin arrow.
 */
function arrow(from: Vector3, direction: Vector3, length: number, colour: Color): Group {
  const material = new MeshBasicMaterial({ color: colour, transparent: true });
  const headLength = SPIN_TUBE * 7;
  const shaft = length - headLength;
  // Both parts are built along +y from the arrow's foot, then turned.
  const tube = new Mesh(new CylinderGeometry(SPIN_TUBE, SPIN_TUBE, shaft, 12), material);
  tube.position.y = shaft / 2;
  const head = new Mesh(new ConeGeometry(SPIN_TUBE * 3, headLength, 16), material);
  head.position.y = shaft + headLength / 2;
  tube.renderOrder = head.renderOrder = 2;
  const group = new Group();
  group.add(tube, head);
  group.position.copy(from);
  group.quaternion.setFromUnitVectors(new Vector3(0, 1, 0), direction.clone().normalize());
  return group;
}

/**
 * Holds what `material` draws at the camera's far plane at most, instead
 * of letting the far plane cut it: still hidden by the globe, which lies
 * nearer.
 */
function clampToFarPlane(material: LineDashedMaterial): void {
  material.onBeforeCompile = (shader) => {
    shader.vertexShader = shader.vertexShader.replace(
      '#include <project_vertex>',
      '#include <project_vertex>\n\tgl_Position.z = min(gl_Position.z, gl_Position.w * 0.99999);',
    );
  };
}

function clear(group: Object3D): void {
  for (const child of [...group.children]) clearOne(group, child);
}

/** Frees `child`'s geometries and materials and takes it out of `group`. */
function clearOne(group: Object3D, child: Object3D): void {
  child.traverse((object) => {
    const mesh = object as Mesh;
    mesh.geometry?.dispose();
    const material = mesh.material as MeshBasicMaterial | MeshBasicMaterial[] | undefined;
    if (Array.isArray(material)) material.forEach((m) => m.dispose());
    else material?.dispose();
  });
  group.remove(child);
}
