// Where a point of the globe lies on the map, from its direction: the CPU
// twin of `locate` in the surface fragment shader (shaders.ts), which uses
// it round the poles. There the mesh's pole row is one point carrying
// every u from 0 to 1, so the uv interpolated across the triangles that
// fan round it is sheared into a starburst.

/** A point on the map: u east from the seam (0 to 1), v north (0 to 1). */
export interface GlobeUv {
  /** u in [0, 1), wrapping at the seam (longitude 0, +z). */
  u: number;
  /**
   * The same longitude in [−0.5, 0.5), wrapping at the opposite meridian:
   * continuous across the seam (Tarini 2012, "Cylindrical and toroidal
   * parameterizations without vertex seams", Journal of Graphics Tools).
   */
  uOpposite: number;
  v: number;
}

const fract = (x: number) => x - Math.floor(x);

/**
 * The map position of direction (x, y, z), the inverse of the surface
 * vertex shader's sphere = (cos lat sin lon, sin lat, cos lat cos lon)
 * with lon = 2π u and lat = π (v − 0.5). The direction need not be unit.
 */
export function globeUv(x: number, y: number, z: number): GlobeUv {
  const length = Math.hypot(x, y, z);
  const turn = Math.atan2(x, z) / (2 * Math.PI);
  return {
    u: fract(turn),
    uOpposite: fract(turn + 0.5) - 0.5,
    v: Math.asin(Math.min(Math.max(y / length, -1), 1)) / Math.PI + 0.5,
  };
}

/**
 * The derivative of u across a step, seam-safe: of the two longitudes'
 * changes (`du` of u, `duOpposite` of uOpposite), the smaller, since only
 * one of them can jump by a whole turn there (the shader compares their
 * fwidth).
 */
export function seamSafeStep(du: number, duOpposite: number): number {
  return Math.abs(du) > Math.abs(duOpposite) ? duOpposite : du;
}

/**
 * u moved by whole turns to the side of the seam `near` is on (the mesh's
 * own u), so that a read at the seam stays next to its neighbours.
 */
export function unwrapNear(u: number, near: number): number {
  return u + Math.floor(near - u + 0.5);
}
