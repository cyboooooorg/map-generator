// Resolution of the drawing buffer. With MSAA, every frame costs in
// proportion to the square of the pixel ratio: a 3× phone screen draws 9
// times the pixels of a 1× one, for little visible gain over 2×. While the
// view moves (camera, damping, morph), frames are drawn at a lower ratio,
// and the settled view at full ratio again.

/** Highest pixel ratio drawn at, when the view is still. */
export const MAX_PIXEL_RATIO = 2;
/** Highest pixel ratio while the view moves. */
export const MOVING_PIXEL_RATIO = 1.25;
/**
 * The view counts as moving until it has been still this long, so a short
 * pause in a drag does not switch the resolution back and forth.
 */
export const SETTLE_MS = 150;

/** The pixel ratio of the settled view, for a screen of `deviceRatio` (`window.devicePixelRatio`). */
export function fullPixelRatio(deviceRatio: number): number {
  return Math.min(deviceRatio, MAX_PIXEL_RATIO);
}

/** The pixel ratio to draw at, for a screen of `deviceRatio`, when the view last moved `msSinceMotion` ago. */
export function pixelRatio(deviceRatio: number, msSinceMotion: number): number {
  const full = fullPixelRatio(deviceRatio);
  return msSinceMotion < SETTLE_MS ? Math.min(full, MOVING_PIXEL_RATIO) : full;
}
