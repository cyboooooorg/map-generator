// What the planet view draws about the body shown's own motion, on the
// globe (see PlanetView): its spin, as a solid curved arrow round its axis
// (or, for a body locked to what it orbits, a closed ring: no spin of its
// own against it), and in a planetary system, its host planet (a dotted
// line to it) and its motion: the pull of what it orbits and its orbital
// velocity as arrows, drawn as the spin arrow is (a tube and a cone).
// Positions are in radii of the body, in the frame of the body shown.
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
/** The arrows of the pull and the velocity: their length beyond the surface, in radii. */
const GUIDE_LENGTH = 1.4;
/** Points of the host line, spaced geometrically from the surface to the host (see hostLineRadii). */
const HOST_LINE_POINTS = 96;
/** Fixed hues of meaning (see the redesign spec): velocity cyan, pull orange, the host line pale. */
export const VELOCITY_COLOUR = new Color('#4fd1ff');
export const PULL_COLOUR = new Color('#ff9a3d');
const HOST_LINE_COLOUR = new Color('#c8d2e6');

/** The host planet as the guides draw it: its centre. */
export interface HostGuide {
  position: Vec3;
}

/**
 * The body's motion as the guides draw it: the direction of the pull of
 * what it orbits (its planet, or the star), and its direction of motion
 * round it (or null).
 */
export interface MotionGuide {
  pull: Vec3;
  velocity: Vec3 | null;
}

/**
 * The points the labels hang from, in the scene's frame (see
 * PlanetView.skyScreenPositions). The host's is a fixed point on the line
 * to it (see hostLabelRadius), merged with the pull's label.
 */
export interface GuidePoints {
  spin: Vector3 | null;
  host: Vector3 | null;
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

/** How far beyond the surface the host's label hangs, in radii of the body: one diameter. */
export const HOST_LABEL_BEYOND = 2;

/**
 * How far from the body's centre, in its radii, the host's distance label
 * hangs along the line to the host: one diameter of the body beyond its
 * `surface` there, or the tip of the pull arrow (`pullTip`, from the
 * centre) when that lies farther out, never beyond the host (`distance`).
 * It is fixed on the axis, wherever the camera goes.
 */
export function hostLabelRadius(surface: number, pullTip: number, distance: number): number {
  return Math.min(Math.max(surface + HOST_LABEL_BEYOND, pullTip), Math.max(distance, surface));
}

export class BodyGuides {
  /** Turns with the stars, as the spin axis does (see PlanetView.setStarsTurn). */
  readonly spin = new Group();
  /** In the body's frame, as the sky bodies are. */
  readonly host = new Group();
  /** The pull and the velocity, each starting from the surface where it points (see setSurface); in the body's frame. */
  readonly vectors = new Group();
  private pull: { arrow: Group; direction: Vector3 } | null = null;
  private velocity: { arrow: Group; direction: Vector3 } | null = null;
  /** How far the surface reaches in a direction of the body's frame, in its radii (1: a sphere). */
  private surfaceAt: (direction: Vector3) => number = () => 1;
  private spinTip = new Vector3();
  /** The host's direction (unit) and distance, in radii, from the body's centre; null without a host. */
  private hostAxis: { direction: Vector3; distance: number } | null = null;
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
    this.vectors.visible = false;
  }

  /**
   * How far out the body's surface reaches: `poles` round its poles, and
   * `at` in any direction of its frame, in its radii (an irregular moon's
   * shape with its relief, a few tenths over 1 on a moonlet a few km
   * across). The spin arrow, scaled by `poles`, and the arrows of the
   * pull and of the velocity start from there rather than inside the body.
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

  /** The line to the host planet, or none (null). */
  setHost(guide: HostGuide | null): void {
    clear(this.host);
    this.hostAxis = null;
    if (!guide) return;
    const to = new Vector3(...guide.position);
    const distance = to.length();
    if (distance < 1e-6) return;
    const toward = to.clone().divideScalar(distance);
    // A dotted line from the surface to the host's centre, whole: the
    // host lies far beyond the camera's far plane, which only bounds the
    // globe's depth, so the line's depth is held at that plane instead of
    // being cut there (see clampToFarPlane).
    this.hostAxis = { direction: toward, distance };
    const hostLine = hostLineRadii(distance).map((r) => toward.clone().multiplyScalar(r));
    const material = new LineDashedMaterial({
      color: HOST_LINE_COLOUR,
      dashSize: 0.25,
      gapSize: 0.25,
      transparent: true,
      opacity: 0.75,
    });
    clampToFarPlane(material);
    const line = new Line(new BufferGeometry().setFromPoints(hostLine), material);
    line.computeLineDistances();
    line.frustumCulled = false;
    this.host.add(line);
  }

  /**
   * The arrows of the pull and of the velocity, or none (null). Arrows
   * already there are turned in place (the motion changes every frame
   * while the clock plays); they are rebuilt only when one comes or goes.
   */
  setMotion(motion: MotionGuide | null): void {
    if (motion && this.pull && (motion.velocity === null) === (this.velocity === null)) {
      const turn = (guide: { arrow: Group; direction: Vector3 }, to: Vec3) => {
        const d = new Vector3(...to);
        if (d.lengthSq() < 1e-12) return;
        guide.direction.copy(d.normalize());
        guide.arrow.quaternion.setFromUnitVectors(new Vector3(0, 1, 0), guide.direction);
        guide.arrow.position.copy(guide.direction).multiplyScalar(this.surfaceAt(guide.direction));
      };
      turn(this.pull, motion.pull);
      this.pullTip = this.tipOf(this.pull.direction);
      if (this.velocity && motion.velocity) {
        turn(this.velocity, motion.velocity);
        this.velocityTip = this.tipOf(this.velocity.direction);
      }
      return;
    }
    clear(this.vectors);
    this.pullTip = this.velocityTip = null;
    this.pull = this.velocity = null;
    if (!motion) return;
    const toward = new Vector3(...motion.pull);
    if (toward.lengthSq() < 1e-12) return;
    toward.normalize();
    // The pull, from the surface towards what the body orbits.
    const pull = arrow(toward.clone().multiplyScalar(this.surfaceAt(toward)), toward, GUIDE_LENGTH, PULL_COLOUR, this.prepare);
    this.vectors.add(pull);
    this.pull = { arrow: pull, direction: toward };
    this.pullTip = this.tipOf(toward);
    if (motion.velocity) {
      const v = new Vector3(...motion.velocity).normalize();
      const velocity = arrow(v.clone().multiplyScalar(this.surfaceAt(v)), v, GUIDE_LENGTH, VELOCITY_COLOUR, this.prepare);
      this.vectors.add(velocity);
      this.velocity = { arrow: velocity, direction: v };
      this.velocityTip = this.tipOf(v);
    }
    // Far away in the sky, beyond the far plane its depth is held short of: never culled.
    this.vectors.traverse((object) => (object.frustumCulled = false));
  }

  /** Shows them, faded with the globe (`fade`, 0 on the flat map). */
  update(spinOn: boolean, hostOn: boolean, vectorsOn: boolean, fade: number): void {
    this.spin.visible = spinOn && this.locked !== null && fade > 0.01;
    this.host.visible = hostOn && fade > 0.01;
    this.vectors.visible = vectorsOn && this.pull !== null && fade > 0.01;
    for (const group of [this.spin, this.host, this.vectors]) {
      group.traverse((object) => {
        const material = (object as Mesh).material as MeshBasicMaterial | undefined;
        if (material) material.opacity = fade ** 3 * (object instanceof Line ? 0.75 : 1);
      });
    }
  }

  /** The host label's point on the line to the host, in the host group's frame (see hostLabelRadius). */
  private hostLabel(): Vector3 | null {
    const axis = this.hostAxis;
    if (!axis) return null;
    const surface = this.surfaceAt(axis.direction);
    // Beyond the pull arrow's tip while it is drawn along the line.
    const pull = this.vectors.visible ? (this.pullTip?.length() ?? 0) : 0;
    return axis.direction.clone().multiplyScalar(hostLabelRadius(surface, pull, axis.distance));
  }

  /** The points their labels hang from, in the scene's frame; null for those not shown. */
  points(): GuidePoints {
    const scene = (v: Vector3 | null, group: Group) => (v && group.visible ? group.localToWorld(v.clone()) : null);
    return {
      spin: scene(this.spinTip, this.spin),
      host: scene(this.hostLabel(), this.host),
      pull: scene(this.pullTip, this.vectors),
      velocity: scene(this.velocityTip, this.vectors),
    };
  }

  dispose(): void {
    clear(this.spin);
    clear(this.vectors);
    clear(this.host);
  }
}

/**
 * An arrow from the surface point `from` along `direction`,
 * `length` long in all: a tube and a cone, as the spin arrow.
 */
function arrow(from: Vector3, direction: Vector3, length: number, colour: Color, prepare: (material: MeshBasicMaterial) => void): Group {
  const material = new MeshBasicMaterial({ color: colour, transparent: true });
  prepare(material);
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
