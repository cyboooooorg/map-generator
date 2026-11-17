import type { Vec3 } from '../skyBodies';
import { dustGain } from './dustPhase';

/**
 * Height of a geyser plume over the pole, in radii of the moon drawn
 * (look constant: Cassini images of Enceladus show the plume rising about
 * one radius above the south pole; Porco et al. 2006, Science 311, 1393).
 */
export const PLUME_HEIGHT_RADII = 1.5;

/**
 * Shortest plume, in CSS pixels, like the floor of the disc (look constant:
 * the shortest plume an eye picks out next to a 5 px disc).
 */
export const PLUME_MIN_PX = 10;

/** Half-width of the top of the plume at least, in CSS pixels (look constant: keeps the fan from being a sliver). */
export const PLUME_MIN_SPREAD_PX = 2.5;

/** Opacity of the plume at 90° of scattering (look constant); back-lit it grows with `dustGain`. */
export const PLUME_ALPHA = 0.35;

/**
 * Where a plume stands: `up` along the spin axis of the moon (`axes[1]`, its
 * north pole, negated for a south pole) and its `height` in the units of
 * `drawnHalf`, the half-size the moon is actually drawn at (a small moon's
 * few-pixel floor included), never under `PLUME_MIN_PX`: pass `drawnHalf`
 * in CSS pixels to get the floor, or 1 to get radii.
 */
export function plumeFrame(axes: readonly [Vec3, Vec3, Vec3] | readonly [readonly number[], readonly number[], readonly number[]], south: boolean, drawnHalf: number, minHeight = PLUME_MIN_PX): { up: Vec3; height: number } {
  const [x = 0, y = 0, z = 0] = axes[1];
  const s = south ? -1 : 1;
  return { up: [s * x, s * y, s * z], height: Math.max(PLUME_HEIGHT_RADII * drawnHalf, minHeight) };
}

/**
 * Opacity of the plume from the cosine of the phase angle of its moon
 * (1 full, -1 new): the ice grains scatter forward, so a new moon, with
 * the sun behind it, shows the brightest plume.
 */
export function plumeAlpha(phaseCos: number): number {
  return Math.min(1, PLUME_ALPHA * dustGain(-phaseCos));
}

/**
 * The direction across a plume whose axis on the disc is `axis`, as the
 * plume shader uses it: with x across and y along, (across, axis) has the
 * same handedness as the disc's (right, up), so the quad is front-facing.
 */
export function plumeAcross(axis: readonly [number, number]): [number, number] {
  return [axis[1], -axis[0]];
}
