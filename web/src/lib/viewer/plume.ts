import type { Vec3 } from '../skyBodies';
import { dustGain } from './dustPhase';

/**
 * Height of a geyser plume over the pole, in radii of the moon drawn
 * (look constant: Cassini images of Enceladus show the plume rising about
 * one radius above the south pole; Porco et al. 2006, Science 311, 1393).
 */
export const PLUME_HEIGHT_RADII = 1.5;

/** Opacity of the plume at 90° of scattering (look constant); back-lit it grows with `dustGain`. */
export const PLUME_ALPHA = 0.12;

/**
 * Where a plume stands: `up` along the spin axis of the moon (`axes[1]`, its
 * north pole, negated for a south pole) and its `height` in the units of
 * the disc, from `drawnHalf`, the half-size the moon is actually drawn at
 * (a small moon's few-pixel floor included).
 */
export function plumeFrame(axes: readonly [Vec3, Vec3, Vec3] | readonly [readonly number[], readonly number[], readonly number[]], south: boolean, drawnHalf: number): { up: Vec3; height: number } {
  const [x = 0, y = 0, z = 0] = axes[1];
  const s = south ? -1 : 1;
  return { up: [s * x, s * y, s * z], height: PLUME_HEIGHT_RADII * drawnHalf };
}

/**
 * Opacity of the plume from the cosine of the phase angle of its moon
 * (1 full, -1 new): the ice grains scatter forward, so a new moon, with
 * the sun behind it, shows the brightest plume.
 */
export function plumeAlpha(phaseCos: number): number {
  return Math.min(1, PLUME_ALPHA * dustGain(-phaseCos));
}
