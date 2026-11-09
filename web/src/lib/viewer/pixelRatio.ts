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

/** The caps a view draws within (see QualityProfile): settled, and while it moves. */
export interface PixelRatioCaps {
  maxPixelRatio: number;
  movingPixelRatio: number;
}

const DEFAULT_CAPS: PixelRatioCaps = { maxPixelRatio: MAX_PIXEL_RATIO, movingPixelRatio: MOVING_PIXEL_RATIO };

/** The pixel ratio of the settled view, for a screen of `deviceRatio` (`window.devicePixelRatio`). */
export function fullPixelRatio(deviceRatio: number, caps: PixelRatioCaps = DEFAULT_CAPS): number {
  return Math.min(deviceRatio, caps.maxPixelRatio);
}

/** The pixel ratio to draw at, for a screen of `deviceRatio`, when the view last moved `msSinceMotion` ago. */
export function pixelRatio(deviceRatio: number, msSinceMotion: number, caps: PixelRatioCaps = DEFAULT_CAPS): number {
  const full = fullPixelRatio(deviceRatio, caps);
  return msSinceMotion < SETTLE_MS ? Math.min(full, caps.movingPixelRatio) : full;
}

/**
 * Highest pixel ratio of an animated close-up (see `animatedPixelRatio`), on
 * every screen: below 1 the browser upscales the smaller drawing buffer to
 * the canvas's unchanged CSS size, a softer frame for about 44 % fewer pixels
 * than ratio 1.
 */
export const CLOSE_UP_PIXEL_RATIO = 0.75;

/**
 * The pixel ratio of a view that is either animated or still, for a screen of
 * `deviceRatio`: the moving ratio while its clock runs (every frame changes),
 * the full ratio once it is paused (one sharp frame). With `close`, a view
 * whose whole screen is costly surface (the star view under 1.1 radii) is
 * drawn at `CLOSE_UP_PIXEL_RATIO` at most while it runs.
 */
export function animatedPixelRatio(
  deviceRatio: number,
  animated: boolean,
  caps: PixelRatioCaps = DEFAULT_CAPS,
  close = false,
): number {
  const full = fullPixelRatio(deviceRatio, caps);
  if (!animated) return full;
  const moving = Math.min(full, caps.movingPixelRatio);
  return close ? Math.min(moving, CLOSE_UP_PIXEL_RATIO) : moving;
}
