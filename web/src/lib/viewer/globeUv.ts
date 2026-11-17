// Where a point of the globe lies on the map, from its direction: the CPU
// twin of `locate` in the surface fragment shader (shaders.ts), which uses
// it round the poles. There the mesh's pole row is one point carrying
// every u from 0 to 1, so the uv interpolated across the triangles that
// fan round it is sheared into a starburst.

/** A point on the map: u east from the seam, v north (0 to 1). */
export interface GlobeUv {
  /** u in [0, 1), wrapping at the seam (longitude 0, +z). */
  u: number;
  v: number;
}

/**
 * The map position of unit direction (x, y, z), the inverse of the
 * surface vertex shader's sphere = (cos lat sin lon, sin lat, cos lat cos
 * lon) with lon = 2π u and lat = π (v − 0.5).
 */
export function globeUv(x: number, y: number, z: number): GlobeUv {
  const turn = Math.atan2(x, z) / (2 * Math.PI);
  // turn − floor(turn) rounds to 1 for a tiny negative turn.
  const u = turn - Math.floor(turn);
  return {
    u: u >= 1 ? 0 : u,
    v: Math.asin(Math.min(Math.max(y, -1), 1)) / Math.PI + 0.5,
  };
}

/**
 * The change of (u, v) for a small change (dx, dy, dz) of unit direction
 * (x, y, z): u = atan(x, z) / 2π and v = asin(y) / π + 0.5 differentiated.
 * Continuous across the seam, where u itself jumps by a whole turn; the
 * shader takes it from the screen derivatives of the direction. y enters
 * only through x² + z² = 1 − y².
 */
export function globeUvStep(
  x: number,
  _y: number,
  z: number,
  dx: number,
  dy: number,
  dz: number,
): [number, number] {
  const across = Math.max(x * x + z * z, 1e-12);
  const ring = Math.max(Math.sqrt(across), 1e-6);
  return [(z * dx - x * dz) / (2 * Math.PI * across), dy / (Math.PI * ring)];
}

/**
 * u moved by whole turns to the side of the seam `near` is on (the mesh's
 * own u), so that a read at the seam stays next to its neighbours.
 */
export function unwrapNear(u: number, near: number): number {
  return u + Math.floor(near - u + 0.5);
}
