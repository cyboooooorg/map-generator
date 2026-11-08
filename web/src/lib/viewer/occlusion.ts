// Whether the body shown hides a point from the camera, for the labels laid
// over the view (see PlanetView.skyScreenPositions): a sphere test let the
// labels of a small, irregular moon's dial and guides show through the
// parts of it that bulge past its mean radius.
import { Vector3 } from 'three';

/** Samples along the part of the ray inside the shape's bounding sphere. */
const STEPS = 64;

/**
 * Whether the surface `radiusAt(direction)` (radii of the body, its centre
 * at the origin, never beyond `bound`) lies between `camera` and `point`:
 * the ray is sampled where it crosses the bounding sphere, short of the
 * point itself.
 */
export function hiddenByShape(
  camera: Vector3,
  point: Vector3,
  radiusAt: (direction: Vector3) => number,
  bound: number,
): boolean {
  const ray = point.clone().sub(camera);
  const length = ray.length();
  if (length < 1e-9) return false;
  ray.divideScalar(length);
  const b = camera.dot(ray);
  const disc = b * b - (camera.lengthSq() - bound * bound);
  if (disc <= 0) return false;
  const root = Math.sqrt(disc);
  const from = Math.max(-b - root, 0);
  const to = Math.min(-b + root, length - 1e-6);
  if (from >= to) return false;
  const q = new Vector3();
  for (let k = 0; k <= STEPS; k++) {
    q.copy(camera).addScaledVector(ray, from + ((to - from) * k) / STEPS);
    const r = q.length();
    if (r < 1e-9 || r < radiusAt(q)) return true;
  }
  return false;
}

/** A sphere of the sky that can cover what lies behind it: a body's centre and radius, in the scene's frame. */
export interface SkyDisc {
  position: readonly [number, number, number];
  radius: number;
}

/**
 * Whether a sky body's disc covers the direction `direction` (from `camera`)
 * to something `distance` away (Infinity for the stars and the host star):
 * the ray crosses a body nearer than that, centre to centre, so the
 * labels of far objects do not show over the host planet or a moon in
 * front of them. A body never covers itself (equal distances).
 */
export function hiddenBySkyDisc(
  camera: readonly [number, number, number],
  direction: readonly [number, number, number],
  distance: number,
  discs: readonly SkyDisc[],
): boolean {
  const length = Math.hypot(...direction);
  if (length < 1e-12) return false;
  const d = [direction[0] / length, direction[1] / length, direction[2] / length] as const;
  for (const { position, radius } of discs) {
    const to = [position[0] - camera[0], position[1] - camera[1], position[2] - camera[2]] as const;
    const centre = Math.hypot(...to);
    // Not the body itself, nor one farther than the object.
    if (centre >= distance * (1 - 1e-6) - 1e-9) continue;
    const along = to[0] * d[0] + to[1] * d[1] + to[2] * d[2];
    if (along <= 0) continue;
    // Squared distance from the ray to the centre against the radius.
    if (centre * centre - along * along < radius * radius) return true;
  }
  return false;
}
