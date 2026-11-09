// A giant's rings and its moons' orbits as the star view draws them round
// its marker: the rings, and the orbits of the moons among them, in the
// giant's equatorial plane (see alignRingPlane).
import {
  BufferGeometry,
  DoubleSide,
  Group,
  Line,
  LineBasicMaterial,
  Mesh,
  MeshBasicMaterial,
  RingGeometry,
  SphereGeometry,
  Vector3,
  type Color,
  type ColorRepresentation,
  type Object3D,
} from 'three';
import type { RingSystem } from '../api/types/RingSystem';
import type { Vec3 } from '../skyBodies';
import { RING_MOON_SIZE, ringBands, SYMBOLIC_RINGS, type MoonOrbit } from './starSystem';

const UP = new Vector3(0, 1, 0);
const axisNow = new Vector3();

/**
 * Turns a ring plane (rings or ring moons' orbits drawn round its +y, see
 * ringMeshes and moonSet) so that its normal is `axis`, the giant's north
 * in the view's frame (see giantNorth): the rings lie in its equator.
 */
export function alignRingPlane(plane: Object3D, axis: Vec3): void {
  plane.quaternion.setFromUnitVectors(UP, axisNow.set(...axis).normalize());
}

/**
 * A giant's rings round its marker (`size` its radius): to scale with
 * their gaps when the system drew them (`rings`), each gap wide enough
 * for a ring moon's marker (see moonOrbits); else a plain band. Drawn
 * round the group's +y; alignRingPlane tilts them with the giant's axis.
 */
export function ringMeshes(size: number, rings: RingSystem | null, colour: Color, opacity: number): Group {
  const group = new Group();
  const bands = rings ? ringBands(rings, RING_MOON_SIZE * 1.6) : [SYMBOLIC_RINGS];
  for (const [inner, outer] of bands) {
    const band = new Mesh(
      new RingGeometry(size * inner, size * outer, 64),
      new MeshBasicMaterial({ color: colour, transparent: true, opacity, side: DoubleSide, depthWrite: false }),
    );
    // A RingGeometry lies in its xy plane: laid in the group's xz plane.
    band.rotation.x = -Math.PI / 2;
    band.renderOrder = 3;
    group.add(band);
  }
  return group;
}

/** How a set of moons' orbits and markers are drawn. */
export interface MoonLook {
  /** Colour and opacity of the orbit lines. */
  colour: ColorRepresentation;
  opacity: number;
  /** Segments of an orbit line. */
  segments: number;
  /** Width and height segments of a moon's marker. */
  detail: [number, number];
}

/**
 * A planet's moons round its marker: their orbit lines and markers, on the
 * orbits `moonOrbits` gives. A ring moon's orbit is drawn in `ringPlane`,
 * in the rings' plane (aligned as they are, see alignRingPlane); the
 * others at their inclination to the view's plane. Markers are placed by
 * placeMoonMarkers.
 */
export interface MoonSet {
  group: Group;
  ringPlane: Group;
  markers: Mesh<SphereGeometry, MeshBasicMaterial>[];
}

/** Builds the orbit lines and markers of `orbits` (see MoonSet), into `group` when given. */
export function moonSet(orbits: readonly MoonOrbit[], look: MoonLook, group = new Group()): MoonSet {
  const ringPlane = new Group();
  group.add(ringPlane);
  const markers = orbits.map((orbit) => {
    const parent = orbit.ring ? ringPlane : group;
    const line = new Line(
      new BufferGeometry().setFromPoints(
        Array.from({ length: look.segments + 1 }, (_, i) => {
          const a = (i / look.segments) * 2 * Math.PI;
          return new Vector3(Math.cos(a) * orbit.radius, 0, -Math.sin(a) * orbit.radius);
        }),
      ),
      new LineBasicMaterial({ color: look.colour, transparent: true, opacity: look.opacity }),
    );
    line.rotation.x = orbit.inclination;
    // Transparent: drawn in a fixed order with the glow and prominences.
    line.renderOrder = 3;
    parent.add(line);
    const marker = new Mesh(new SphereGeometry(orbit.size, ...look.detail), new MeshBasicMaterial({ color: '#d9d4c8' }));
    parent.add(marker);
    return marker;
  });
  return { group, ringPlane, markers };
}

/**
 * Places the moons' markers at `time` (animated seconds) on their orbits,
 * each in its own plane (a ring moon's the ring plane); returns their
 * places there, from the planet.
 */
export function placeMoonMarkers(orbits: readonly MoonOrbit[], markers: readonly Mesh[], time: number): Vector3[] {
  return orbits.map((orbit, k) => {
    const angle = orbit.phase + time * orbit.speed;
    const local = new Vector3(Math.cos(angle) * orbit.radius, 0, -Math.sin(angle) * orbit.radius);
    local.applyAxisAngle(new Vector3(1, 0, 0), orbit.inclination);
    markers[k]?.position.copy(local);
    return local;
  });
}

/** Disposes of the geometries and materials of everything under `root`, and empties it. */
export function disposeTree(root: Object3D): void {
  root.traverse((object) => {
    if (object instanceof Mesh || object instanceof Line) {
      object.geometry.dispose();
      (object.material as MeshBasicMaterial).dispose();
    }
  });
  root.clear();
}
