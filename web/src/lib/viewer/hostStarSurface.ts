/**
 * The host star in a planet's sky (PlanetView's sky disc) drawn with the
 * same living surface as the star views (starSurface.ts): the geometry
 * from the disc on screen to the star's inertial frame, and how much of
 * the surface the disc shows at its size.
 *
 * The star's inertial frame has y along its spin axis, the system's +y,
 * and longitude 0 along the system's +x at day 0 (the plan's Decision 9);
 * `systemSky` gives its axes in the frame of the body shown, the planet
 * view's world. The disc's frame is the screen's: x right, y up, z from
 * the star towards the camera. The basis changes with the camera every
 * frame, which is geometry: the surface at a day depends only on the
 * activity and the day.
 */
import type { Vec3 } from '../skyBodies';

const dot = (a: Vec3, b: Vec3): number => a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
const cross = (a: Vec3, b: Vec3): Vec3 => [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
const unit = (a: Vec3): Vec3 => {
  const l = Math.max(Math.hypot(a[0], a[1], a[2]), 1e-12);
  return [a[0] / l, a[1] / l, a[2] / l];
};

/**
 * Maps the disc's point-sprite coordinates (x right, y up, z towards the
 * viewer, unit disc) to the star's inertial frame: columns = star-frame
 * images of screen right, screen up and the line of sight (star →
 * camera). `toCamera` is the unit direction from the star to the camera,
 * `cameraUp` the screen's up in the world (made orthogonal to the line
 * of sight here), `starAxes` the star's x, y (spin) and z axes in the
 * world. Column-major, as `Matrix3.fromArray` and GLSL's `mat3` take it.
 */
export function discBasis(toCamera: Vec3, cameraUp: Vec3, starAxes: [Vec3, Vec3, Vec3]): number[] {
  const z = unit(toCamera);
  let up: Vec3 = [cameraUp[0] - z[0] * dot(cameraUp, z), cameraUp[1] - z[1] * dot(cameraUp, z), cameraUp[2] - z[2] * dot(cameraUp, z)];
  // An up along the line of sight: any perpendicular will do.
  if (Math.hypot(...up) < 1e-9) up = Math.abs(z[1]) < 0.9 ? cross(z, cross([0, 1, 0], z)) : cross(z, cross([1, 0, 0], z));
  const y = unit(up);
  const x = cross(y, z);
  const [sx, sy, sz] = starAxes;
  const inStar = (w: Vec3): Vec3 => [dot(w, sx), dot(w, sy), dot(w, sz)];
  return [...inStar(x), ...inStar(y), ...inStar(z)];
}

/** Disc diameter, CSS px, below which the sky disc shows no active region. */
const DETAIL_FROM_PX = 16;
/** Disc diameter, CSS px, from which it shows them all. */
const DETAIL_FULL_PX = 24;

/**
 * Detail on the sky disc, from its diameter in CSS pixels: the active
 * regions fade in from 16 to 24 px and are off below, so that the usual
 * disc (the Sun from Earth is about 14 px at 1080p) skips the region loop
 * and costs only its limb law. Granulation is never drawn there: compiled
 * out (`STAR_NO_GRANULATION`).
 */
export function hostStarDetail(diskPx: number): number {
  return Math.min(1, Math.max(0, (diskPx - DETAIL_FROM_PX) / (DETAIL_FULL_PX - DETAIL_FROM_PX)));
}

/** The sprite's side while the disc is small, drawing-buffer px, before it must grow with the disc. */
const SPRITE_CAP_PX = 512;

/**
 * The side of the square the host star is drawn on, drawing-buffer px,
 * for a disc `diskPx` across: six discs and a margin, at most 512 px,
 * and never less than twice the disc and the margin, so that the glow,
 * which fades out from a quarter of the side to its half, always starts
 * outside the disc. (A disc wider than the capped square once filled it
 * whole: a white square behind a close-in planet.)
 */
export function hostStarSpriteSize(diskPx: number, pixelRatio: number): number {
  const margin = 24 * pixelRatio;
  return Math.max(Math.min(diskPx * 6 + margin, SPRITE_CAP_PX), diskPx * 2 + margin);
}
