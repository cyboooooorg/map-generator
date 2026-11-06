// What the planet view draws about the body shown's own motion, on the
// globe (see PlanetView): its spin, as a solid curved arrow round its axis
// (or, for a body locked to what it orbits, a closed ring: no spin of its
// own against it), and in a planetary system, its host planet: a dotted
// line to it, the host's pull on the body and the body's orbital velocity
// as arrows. Positions are in radii of the body shown, in its frame.
import {
  ArrowHelper,
  BufferGeometry,
  Color,
  ConeGeometry,
  CatmullRomCurve3,
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
/** Fixed hues of meaning (see the redesign spec): velocity cyan, pull orange, the host line pale. */
export const VELOCITY_COLOUR = new Color('#4fd1ff');
export const PULL_COLOUR = new Color('#ff9a3d');
const HOST_LINE_COLOUR = new Color('#c8d2e6');

/** The host planet as the guides draw it: its centre, and the body's direction of motion round it (or null). */
export interface HostGuide {
  position: Vec3;
  velocity: Vec3 | null;
}

/** The points the labels hang from, in the scene's frame (see PlanetView.skyScreenPositions). */
export interface GuidePoints {
  spin: Vector3 | null;
  host: Vector3 | null;
  pull: Vector3 | null;
  velocity: Vector3 | null;
}

export class BodyGuides {
  /** Turns with the stars, as the spin axis does (see PlanetView.setStarsTurn). */
  readonly spin = new Group();
  /** In the body's frame, as the sky bodies are. */
  readonly host = new Group();
  private spinTip = new Vector3();
  private hostMid: Vector3 | null = null;
  private pullTip: Vector3 | null = null;
  private velocityTip: Vector3 | null = null;
  private locked: boolean | null = null;

  constructor(private readonly colour: Color) {
    this.spin.visible = false;
    this.spin.renderOrder = 2;
    this.host.visible = false;
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
    clear(this.host);
    this.hostMid = this.pullTip = this.velocityTip = null;
    if (!guide) return;
    const to = new Vector3(...guide.position);
    const distance = to.length();
    if (distance < 1e-6) return;
    const toward = to.clone().divideScalar(distance);
    // A dotted line from the surface to the host's centre.
    const line = new Line(
      new BufferGeometry().setFromPoints([toward.clone(), to]),
      new LineDashedMaterial({ color: HOST_LINE_COLOUR, dashSize: 0.25, gapSize: 0.25, transparent: true, opacity: 0.75 }),
    );
    line.computeLineDistances();
    this.host.add(line);
    // Its distance is written along the line, a little beyond the pull
    // arrow (the line's middle may lie far off screen).
    this.hostMid = toward.clone().multiplyScalar(Math.min((1 + distance) / 2, 1 + GUIDE_LENGTH * 2.2));
    // The host's pull, from the surface towards it.
    this.host.add(arrow(toward, toward, GUIDE_LENGTH, PULL_COLOUR));
    this.pullTip = toward.clone().multiplyScalar(1 + GUIDE_LENGTH);
    if (guide.velocity) {
      const v = new Vector3(...guide.velocity).normalize();
      this.host.add(arrow(v, v, GUIDE_LENGTH, VELOCITY_COLOUR));
      this.velocityTip = v.clone().multiplyScalar(1 + GUIDE_LENGTH);
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
      host: scene(this.hostMid, this.host),
      pull: scene(this.pullTip, this.host),
      velocity: scene(this.velocityTip, this.host),
    };
  }

  dispose(): void {
    clear(this.spin);
    clear(this.host);
  }
}

/** An arrow from the surface point `from` (a unit vector) along `direction`, `length` long. */
function arrow(from: Vector3, direction: Vector3, length: number, colour: Color): ArrowHelper {
  const helper = new ArrowHelper(direction, from, length, colour, length * 0.22, length * 0.12);
  for (const part of [helper.line, helper.cone]) {
    (part.material as MeshBasicMaterial).transparent = true;
  }
  helper.renderOrder = 2;
  return helper;
}

function clear(group: Object3D): void {
  for (const child of [...group.children]) {
    child.traverse((object) => {
      const mesh = object as Mesh;
      mesh.geometry?.dispose();
      const material = mesh.material as MeshBasicMaterial | MeshBasicMaterial[] | undefined;
      if (Array.isArray(material)) material.forEach((m) => m.dispose());
      else material?.dispose();
    });
    group.remove(child);
  }
}
