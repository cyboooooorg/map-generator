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
