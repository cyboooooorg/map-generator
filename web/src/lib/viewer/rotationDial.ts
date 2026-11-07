// The rotation dial the planet view draws round the body shown (see
// PlanetView and rotationDial.ts): a ring in its equatorial plane
// graduated every 5° (long ticks every 30°), a pointer running out to it
// from the surface at the longitude the dial reads, and small markers on
// the ring towards the star and the host planet. Positions are in radii
// of the body shown, in its frame, as the guides' (see bodyGuides.ts).
import {
  BufferGeometry,
  Color,
  ConeGeometry,
  CylinderGeometry,
  Float32BufferAttribute,
  Group,
  LineBasicMaterial,
  LineLoop,
  LineSegments,
  Mesh,
  MeshBasicMaterial,
  SphereGeometry,
  Vector3,
} from 'three';
import type { DialState } from '../rotationDial';

/** Where the dial sits by default, in radii of the body: clear of its surface and of the spin guides' ring. */
export const DIAL_RADIUS = 1.25;
const RING_COLOUR = new Color('#9fd3ff');
const POINTER_COLOUR = new Color('#ffcc66');
const SUN_COLOUR = new Color('#ffe28a');
const HOST_COLOUR = new Color('#c89a6a');
/** The pointer's tube radius, as the spin arrow's (see bodyGuides.ts), and its head's length. */
const SPIN_TUBE = 0.022;
const HEAD_LENGTH = SPIN_TUBE * 7;
const CIRCLE_POINTS = 144;
const TICKS = 72;
/** The fainter outer circle, as a multiple of the ring's radius, and the ticks, in radii of the ring. */
const OUTER = 1.1;
const SHORT_TICK = 0.05;
const LONG_TICK = 0.1;
const MARKER = 0.04;
const Y = new Vector3(0, 1, 0);

export class RotationDial {
  /** Added to the scene once; in the body's frame. */
  readonly group = new Group();
  /** The ring, built once at radius 1 with its 0° along +x, then turned and scaled by `set`. */
  private readonly ring = new Group();
  private readonly pointer = new Group();
  private readonly shaft: Mesh;
  private readonly head: Mesh;
  private readonly sun: Mesh;
  private readonly host: Mesh;
  private readonly materials: { material: LineBasicMaterial | MeshBasicMaterial; weight: number }[] = [];
  private readonly geometries: BufferGeometry[] = [];
  private readonly turn = new Vector3();
  private ringRadius = DIAL_RADIUS;
  private zero = new Vector3(1, 0, 0);
  private pointerDirection = new Vector3(0, 0, 1);
  private sunDirection: Vector3 | null = null;
  private hostDirection: Vector3 | null = null;

  constructor() {
    this.group.visible = false;
    this.group.renderOrder = 2;

    const line = (colour: Color, weight: number) => this.material(new LineBasicMaterial({ color: colour }), weight);
    const mesh = (colour: Color) => this.material(new MeshBasicMaterial({ color: colour }), 1);

    const circle = (radius: number) => {
      const points: number[] = [];
      for (let i = 0; i < CIRCLE_POINTS; i++) {
        const a = (2 * Math.PI * i) / CIRCLE_POINTS;
        points.push(radius * Math.cos(a), 0, -radius * Math.sin(a));
      }
      return this.geometry(new BufferGeometry().setAttribute('position', new Float32BufferAttribute(points, 3)));
    };
    // Tick k points along the 0° direction (+x) turned k·5° right-handed about +y: towards −z.
    const ticks: number[] = [];
    for (let k = 0; k < TICKS; k++) {
      const a = (2 * Math.PI * k) / TICKS;
      const [c, s] = [Math.cos(a), -Math.sin(a)];
      const length = k % 6 === 0 ? LONG_TICK : SHORT_TICK;
      ticks.push(c, 0, s, c * (1 + length), 0, s * (1 + length));
    }
    const tickGeometry = this.geometry(new BufferGeometry().setAttribute('position', new Float32BufferAttribute(ticks, 3)));
    this.ring.add(
      new LineLoop(circle(1), line(RING_COLOUR, 1)),
      new LineLoop(circle(OUTER), line(RING_COLOUR, 0.4)),
      new LineSegments(tickGeometry, line(RING_COLOUR, 1)),
    );

    // The pointer is built along +y from its foot at the surface, then turned (as bodyGuides' arrows).
    const pointerMaterial = mesh(POINTER_COLOUR);
    this.shaft = new Mesh(this.geometry(new CylinderGeometry(SPIN_TUBE, SPIN_TUBE, 1, 12)), pointerMaterial);
    this.head = new Mesh(this.geometry(new ConeGeometry(SPIN_TUBE * 3, HEAD_LENGTH, 16)), pointerMaterial);
    this.pointer.add(this.shaft, this.head);

    const marker = this.geometry(new SphereGeometry(1, 16, 12));
    this.sun = new Mesh(marker, mesh(SUN_COLOUR));
    this.host = new Mesh(marker, mesh(HOST_COLOUR));
    this.sun.visible = this.host.visible = false;

    this.group.add(this.ring, this.pointer, this.sun, this.host);
    this.group.traverse((o) => (o.renderOrder = 2));
  }

  private material<M extends LineBasicMaterial | MeshBasicMaterial>(material: M, weight: number): M {
    material.transparent = true;
    material.depthWrite = false;
    this.materials.push({ material, weight });
    return material;
  }

  private geometry<G extends BufferGeometry>(geometry: G): G {
    this.geometries.push(geometry);
    return geometry;
  }

  /** Places the ring (0° at state.zero), the pointer and the markers; `radius` in body radii. */
  set(state: DialState, radius: number): void {
    this.ringRadius = radius;
    this.zero.set(...state.zero);
    // Turning +x about +y by φ gives (cos φ, 0, −sin φ).
    this.ring.rotation.y = Math.atan2(-state.zero[2], state.zero[0]);
    this.ring.scale.setScalar(radius);

    const lon = state.pointerLongitude;
    this.pointerDirection.set(Math.sin(lon), 0, Math.cos(lon));
    this.pointer.position.copy(this.pointerDirection);
    this.pointer.quaternion.setFromUnitVectors(Y, this.pointerDirection);
    const shaft = Math.max(radius - 1 - HEAD_LENGTH, 1e-3);
    this.shaft.scale.y = shaft;
    this.shaft.position.y = shaft / 2;
    this.head.position.y = shaft + HEAD_LENGTH / 2;

    this.sunDirection = this.place(this.sun, state.sun, radius);
    this.hostDirection = this.place(this.host, state.host, radius);
  }

  /** Puts `marker` on the ring towards `direction` (hidden for null); returns the point. */
  private place(marker: Mesh, direction: DialState['sun'], radius: number): Vector3 | null {
    marker.visible = direction !== null;
    if (!direction) return null;
    marker.scale.setScalar(MARKER * radius);
    marker.position.set(...direction).multiplyScalar(radius);
    return marker.position;
  }

  /** Shows or hides it, faded with the globe's morph (opacity = fade³, as bodyGuides). */
  update(on: boolean, fade: number): void {
    this.group.visible = on && fade > 0.01;
    const opacity = fade ** 3;
    for (const { material, weight } of this.materials) material.opacity = opacity * weight;
  }

  /** Points to label, in the body's frame: the 12 numbered ticks (0°, 30°, … 330°, in order), the pointer's tip, the sun and host markers (null when absent). */
  anchors(): { ticks: Vector3[]; pointer: Vector3; sun: Vector3 | null; host: Vector3 | null } {
    const ticks = Array.from({ length: 12 }, (_, k) =>
      this.turn.copy(this.zero).applyAxisAngle(Y, (k * Math.PI) / 6).multiplyScalar(this.ringRadius).clone(),
    );
    return {
      ticks,
      pointer: this.pointerDirection.clone().multiplyScalar(this.ringRadius),
      sun: this.sunDirection?.clone() ?? null,
      host: this.hostDirection?.clone() ?? null,
    };
  }

  dispose(): void {
    this.geometries.forEach((g) => g.dispose());
    this.materials.forEach(({ material }) => material.dispose());
    this.group.clear();
  }
}
