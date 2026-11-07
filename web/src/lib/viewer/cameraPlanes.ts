/** How far behind the globe's centre the far plane reaches at least, radii. */
const MIN_DEPTH_BEHIND = 3;

/** The globe camera's clipping planes. */
export interface CameraPlanes {
  near: number;
  far: number;
}

/**
 * The near and far planes of the globe camera.
 *
 * `altitude` is the camera's distance from the centre minus 1, and
 * `ringReach` the radius of the outermost ring drawn, bright or dust, in
 * radii of the body (0: none). The far plane lies beyond the far edge of
 * the rings seen from their plane, `distance + ringReach`, else the part of
 * them behind the planet is clipped and the stars show through. Stars and
 * sky bodies are drawn at infinity, held at the far plane whatever it is.
 */
export function cameraPlanes(altitude: number, ringReach: number): CameraPlanes {
  const behind = Math.max(MIN_DEPTH_BEHIND, ringReach * 1.02 + 0.05);
  return { near: Math.max(altitude * 0.1, 1e-6), far: altitude + 1 + behind };
}
