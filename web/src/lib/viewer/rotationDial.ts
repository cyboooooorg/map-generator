// The rotation dial the planet view draws round the body shown (see
// PlanetView and rotationDial.ts): a ring in its equatorial plane
// graduated every 5° (long ticks every 30°), a pointer running out to it
// from the surface at the longitude the dial reads, and, standing on the
// ring, a vertical half-circle at the star's bearing, graduated every 10°
// from −90° to +90°, the star's marker hung on it at its height; and a
// fainter quarter-circle for each body of the sky chosen (the host, the
// moons), from the ring up or down to its height, its marker at its end.
// Positions are in radii of the body shown, in its frame, as the guides'
// (see bodyGuides.ts).
import {
  BufferGeometry,
  Color,
  ConeGeometry,
  CylinderGeometry,
  Float32BufferAttribute,
  Group,
  Line,
  LineBasicMaterial,
  LineLoop,
  LineSegments,
  Mesh,
  MeshBasicMaterial,
  SphereGeometry,
  Vector3,
} from 'three';
import type { DialBody, DialState } from '../rotationDial';

/** Where the dial sits by default, in radii of the body: clear of its surface and of the spin guides' ring. */
export const DIAL_RADIUS = 1.25;
const RING_COLOUR = new Color('#9fd3ff');
const POINTER_COLOUR = new Color('#ffcc66');
/** The star's marker and half-circle: a soft gold, not the star's glare. */
const SUN_COLOUR = new Color('#c9b98a');
const HOST_COLOUR = new Color('#c89a6a');
const MOON_COLOUR = new Color('#93a8bf');
/** The pointer's tube radius and its head's length, in radii of the body: thinner than the spin arrow. */
const POINTER_TUBE = 0.008;
const HEAD_LENGTH = POINTER_TUBE * 8;
const CIRCLE_POINTS = 144;
const TICKS = 72;
/** The fainter outer circle, as a multiple of the ring's radius, and the ticks, in radii of the ring. */
const OUTER = 1.1;
const SHORT_TICK = 0.05;
const LONG_TICK = 0.1;
/** Where the numbers stand, in radii of the ring: outside the outer circle and the long ticks. */
export const LABEL_OUT = 1.2;
const MARKER = 0.03;
/** The half-circle's numbers, degrees of height. */
export const ARC_NUMBERS = [-90, -60, -30, 0, 30, 60, 90];
const Y = new Vector3(0, 1, 0);

/** A body's quarter-circle and marker, kept while it is chosen. */
interface BodyArc {
  arc: Line;
  marker: Mesh;
  point: Vector3 | null;
}

export class RotationDial {
  /** Added to the scene once; in the body's frame. */
  readonly group = new Group();
  /** The ring, built once at radius 1 with its 0° along +x, then turned and scaled by `set`. */
  private readonly ring = new Group();
  private readonly pointer = new Group();
  private readonly shaft: Mesh;
  private readonly head: Mesh;
  /** The star's half-circle, built at radius 1 in the plane of +x (its bearing) and +y, then turned and scaled. */
  private readonly sunArc = new Group();
  private readonly sun: Mesh;
  private readonly bodyArcs = new Map<string, BodyArc>();
  private readonly quarter: BufferGeometry;
  private readonly marker: SphereGeometry;
  private readonly materials: { material: LineBasicMaterial | MeshBasicMaterial; weight: number }[] = [];
  private readonly geometries: BufferGeometry[] = [];
  private readonly turn = new Vector3();
  private ringRadius = DIAL_RADIUS;
  private zero = new Vector3(1, 0, 0);
  private pointerDirection = new Vector3(0, 0, 1);
  private sunBearing: Vector3 | null = null;
  private sunPoint: Vector3 | null = null;
  private hostPoint: Vector3 | null = null;
  private bodyOrder: string[] = [];

  constructor() {
    this.group.visible = false;
    this.group.renderOrder = 2;

    const line = (colour: Color, weight: number) => this.material(new LineBasicMaterial({ color: colour }), weight);
    const mesh = (colour: Color, weight = 1) => this.material(new MeshBasicMaterial({ color: colour }), weight);
    const points = (list: number[]) =>
      this.geometry(new BufferGeometry().setAttribute('position', new Float32BufferAttribute(list, 3)));

    const circle = (radius: number) => {
      const list: number[] = [];
      for (let i = 0; i < CIRCLE_POINTS; i++) {
        const a = (2 * Math.PI * i) / CIRCLE_POINTS;
        list.push(radius * Math.cos(a), 0, -radius * Math.sin(a));
      }
      return points(list);
    };
    // Tick k points along the 0° direction (+x) turned k·5° right-handed about +y: towards −z.
    const ticks: number[] = [];
    for (let k = 0; k < TICKS; k++) {
      const a = (2 * Math.PI * k) / TICKS;
      const [c, s] = [Math.cos(a), -Math.sin(a)];
      const length = k % 6 === 0 ? LONG_TICK : SHORT_TICK;
      ticks.push(c, 0, s, c * (1 + length), 0, s * (1 + length));
    }
    this.ring.add(
      new LineLoop(circle(1), line(RING_COLOUR, 1)),
      new LineLoop(circle(OUTER), line(RING_COLOUR, 0.4)),
      new LineSegments(points(ticks), line(RING_COLOUR, 1)),
    );

    // The half-circle from −90° to +90° of height, ticks every 10°, long every 30°.
    const half: number[] = [];
    for (let i = 0; i <= CIRCLE_POINTS / 2; i++) {
      const a = -Math.PI / 2 + (Math.PI * i) / (CIRCLE_POINTS / 2);
      half.push(Math.cos(a), Math.sin(a), 0);
    }
    const arcTicks: number[] = [];
    for (let degrees = -90; degrees <= 90; degrees += 10) {
      const a = (degrees * Math.PI) / 180;
      const length = degrees % 30 === 0 ? LONG_TICK : SHORT_TICK;
      arcTicks.push(Math.cos(a), Math.sin(a), 0, Math.cos(a) * (1 + length), Math.sin(a) * (1 + length), 0);
    }
    this.sunArc.add(new Line(points(half), line(SUN_COLOUR, 0.7)), new LineSegments(points(arcTicks), line(SUN_COLOUR, 0.7)));

    // A body's quarter-circle, from the ring up to the zenith (flipped for one below the equator).
    const up: number[] = [];
    for (let i = 0; i <= CIRCLE_POINTS / 4; i++) {
      const a = (Math.PI / 2) * (i / (CIRCLE_POINTS / 4));
      up.push(Math.cos(a), Math.sin(a), 0);
    }
    this.quarter = points(up);

    // The pointer is built along +y from its foot at the surface, then turned (as bodyGuides' arrows).
    const pointerMaterial = mesh(POINTER_COLOUR);
    this.shaft = new Mesh(this.geometry(new CylinderGeometry(POINTER_TUBE, POINTER_TUBE, 1, 12)), pointerMaterial);
    this.head = new Mesh(this.geometry(new ConeGeometry(POINTER_TUBE * 2.5, HEAD_LENGTH, 16)), pointerMaterial);
    this.pointer.add(this.shaft, this.head);

    this.marker = this.geometry(new SphereGeometry(1, 16, 12));
    this.sun = new Mesh(this.marker, mesh(SUN_COLOUR, 0.85));
    this.sun.visible = this.sunArc.visible = false;

    this.group.add(this.ring, this.pointer, this.sunArc, this.sun);
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

  /**
   * Places the ring (0° at state.zero), the pointer, the star's
   * half-circle and the quarter-circles of `state.bodies` (those chosen);
   * `radius` in body radii.
   */
  set(state: DialState, radius: number): void {
    this.ringRadius = radius;
    this.zero.set(...state.zero);
    // Turning +x about +y by φ gives (cos φ, 0, −sin φ).
    const turnTo = (v: readonly number[]) => Math.atan2(-v[2]!, v[0]!);
    this.ring.rotation.y = turnTo(state.zero);
    this.ring.scale.setScalar(radius);

    const lon = state.pointerLongitude;
    this.pointerDirection.set(Math.sin(lon), 0, Math.cos(lon));
    this.pointer.position.copy(this.pointerDirection);
    this.pointer.quaternion.setFromUnitVectors(Y, this.pointerDirection);
    const shaft = Math.max(radius - 1 - HEAD_LENGTH, 1e-3);
    this.shaft.scale.y = shaft;
    this.shaft.position.y = shaft / 2;
    this.head.position.y = shaft + HEAD_LENGTH / 2;

    // The star: its half-circle at its bearing, its marker at its height.
    this.sunBearing = state.sun ? new Vector3(...state.sun) : null;
    this.sunArc.visible = this.sun.visible = this.sunBearing !== null;
    this.sunPoint = null;
    if (this.sunBearing) {
      this.sunArc.rotation.y = turnTo(state.sun!);
      this.sunArc.scale.setScalar(radius);
      this.sunPoint = this.onArc(this.sunBearing, state.sunAltitude, radius);
      this.sun.position.copy(this.sunPoint);
      this.sun.scale.setScalar(MARKER * radius);
    }

    this.setBodies(state.bodies, radius);
  }

  /** The point at `altitude` (radians) on a vertical circle of `radius` standing at `bearing`. */
  private onArc(bearing: Vector3, altitude: number, radius: number): Vector3 {
    return bearing
      .clone()
      .multiplyScalar(Math.cos(altitude) * radius)
      .addScaledVector(Y, Math.sin(altitude) * radius);
  }

  /** Keeps a quarter-circle for each body of `bodies`, dropping those no longer chosen. */
  private setBodies(bodies: readonly DialBody[], radius: number): void {
    const wanted = new Set(bodies.map((b) => b.id));
    for (const [id, arc] of this.bodyArcs) {
      if (wanted.has(id)) continue;
      this.group.remove(arc.arc, arc.marker);
      for (const material of [arc.arc.material, arc.marker.material] as (LineBasicMaterial | MeshBasicMaterial)[]) {
        material.dispose();
        const k = this.materials.findIndex((m) => m.material === material);
        if (k >= 0) this.materials.splice(k, 1);
      }
      this.bodyArcs.delete(id);
    }
    this.hostPoint = null;
    this.bodyOrder = bodies.map((b) => b.id);
    for (const body of bodies) {
      let arc = this.bodyArcs.get(body.id);
      if (!arc) {
        const colour = body.host ? HOST_COLOUR : MOON_COLOUR;
        arc = {
          arc: new Line(this.quarter, this.material(new LineBasicMaterial({ color: colour }), body.host ? 0.55 : 0.4)),
          marker: new Mesh(this.marker, this.material(new MeshBasicMaterial({ color: colour }), body.host ? 0.85 : 0.7)),
          point: null,
        };
        arc.arc.renderOrder = arc.marker.renderOrder = 2;
        this.bodyArcs.set(body.id, arc);
        this.group.add(arc.arc, arc.marker);
      }
      arc.arc.visible = arc.marker.visible = body.bearing !== null;
      arc.point = null;
      if (!body.bearing) continue;
      const bearing = new Vector3(...body.bearing);
      arc.arc.rotation.set(0, Math.atan2(-bearing.z, bearing.x), 0);
      arc.arc.scale.set(radius, radius * (body.altitude < 0 ? -1 : 1), radius);
      arc.point = this.onArc(bearing, body.altitude, radius);
      arc.marker.position.copy(arc.point);
      arc.marker.scale.setScalar(MARKER * radius * (body.host ? 1 : 0.8));
      if (body.host) this.hostPoint = arc.point;
    }
  }

  /** Shows or hides it, faded with the globe's morph (opacity = fade³, as bodyGuides). */
  update(on: boolean, fade: number): void {
    this.group.visible = on && fade > 0.01;
    const opacity = fade ** 3;
    for (const { material, weight } of this.materials) material.opacity = opacity * weight;
  }

  /**
   * Points to label, in the body's frame: the ring's 12 numbers (0°, 30°,
   * … 330°, in order) and the half-circle's (ARC_NUMBERS, null without a
   * star bearing), outside their circles; the pointer's tip; the star's and
   * the host's markers, and each chosen body's in the order of
   * `state.bodies` (null when absent).
   */
  anchors(): {
    ticks: Vector3[];
    arcTicks: (Vector3 | null)[];
    pointer: Vector3;
    sun: Vector3 | null;
    host: Vector3 | null;
    bodies: (Vector3 | null)[];
  } {
    const out = this.ringRadius * LABEL_OUT;
    const ticks = Array.from({ length: 12 }, (_, k) =>
      this.turn.copy(this.zero).applyAxisAngle(Y, (k * Math.PI) / 6).multiplyScalar(out).clone(),
    );
    const bearing = this.sunBearing;
    const arcTicks = ARC_NUMBERS.map((degrees) => (bearing ? this.onArc(bearing, (degrees * Math.PI) / 180, out) : null));
    return {
      ticks,
      arcTicks,
      pointer: this.pointerDirection.clone().multiplyScalar(this.ringRadius),
      sun: this.sunPoint?.clone() ?? null,
      host: this.hostPoint?.clone() ?? null,
      bodies: this.bodyOrder.map((id) => this.bodyArcs.get(id)?.point?.clone() ?? null),
    };
  }

  dispose(): void {
    this.geometries.forEach((g) => g.dispose());
    this.materials.forEach(({ material }) => material.dispose());
    this.bodyArcs.clear();
    this.group.clear();
  }
}
