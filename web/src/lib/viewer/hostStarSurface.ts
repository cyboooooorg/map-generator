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

/**
 * Maps the disc's point-sprite coordinates (x right, y up, z towards the
 * viewer, unit disc) to the star's inertial frame: columns = star-frame
 * images of screen right, screen up and the line of sight (star →
 * camera). `toCamera` is the unit direction from the star to the camera,
 * `cameraUp` the screen's up in the world (made orthogonal to the line
 * of sight here), `starAxes` the star's x, y (spin) and z axes in the
 * world. Column-major, as `Matrix3.fromArray` and GLSL's `mat3` take it,
 * written into `out` when given (the view calls it every frame) and
 * returned.
 */
export function discBasis(toCamera: Vec3, cameraUp: Vec3, starAxes: [Vec3, Vec3, Vec3], out: number[] = new Array<number>(9)): number[] {
  // z: the line of sight.
  const zl = Math.max(Math.hypot(toCamera[0], toCamera[1], toCamera[2]), 1e-12);
  const zx = toCamera[0] / zl;
  const zy = toCamera[1] / zl;
  const zz = toCamera[2] / zl;
  // y: the up vector without its part along z; any perpendicular when
  // it lies along the line of sight.
  let ux = cameraUp[0];
  let uy = cameraUp[1];
  let uz = cameraUp[2];
  let along = ux * zx + uy * zy + uz * zz;
  let yx = ux - zx * along;
  let yy = uy - zy * along;
  let yz = uz - zz * along;
  if (Math.hypot(yx, yy, yz) < 1e-9) {
    [ux, uy, uz] = Math.abs(zy) < 0.9 ? [0, 1, 0] : [1, 0, 0];
    along = ux * zx + uy * zy + uz * zz;
    yx = ux - zx * along;
    yy = uy - zy * along;
    yz = uz - zz * along;
  }
  const yl = Math.hypot(yx, yy, yz);
  yx /= yl;
  yy /= yl;
  yz /= yl;
  // x = y × z.
  const xx = yy * zz - yz * zy;
  const xy = yz * zx - yx * zz;
  const xz = yx * zy - yy * zx;
  for (let k = 0; k < 3; k++) {
    const a = starAxes[k]!;
    out[k] = xx * a[0] + xy * a[1] + xz * a[2];
    out[3 + k] = yx * a[0] + yy * a[1] + yz * a[2];
    out[6 + k] = zx * a[0] + zy * a[1] + zz * a[2];
  }
  return out;
}

/** Disc diameter, CSS px, below which the sky disc shows no active region. */
const DETAIL_FROM_PX = 16;
/** Disc diameter, CSS px, from which it shows them all. */
const DETAIL_FULL_PX = 24;

/**
 * Detail on the sky disc, from its diameter in CSS pixels: the active
 * regions fade in from 16 to 24 px and are off below, so that the usual
 * disc (the Sun from Earth is about 14 px at 1080p) skips the region loop
 * and costs only its limb law. The supergranule network is capped by it
 * too (it only shows on a far larger disc, see surfaceDetail).
 * Granulation is never drawn there: compiled out (`STAR_NO_GRANULATION`).
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
