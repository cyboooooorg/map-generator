// A giant's storms, found in its biome map, so that its view can drift
// each of them whole: a storm is a vortex moving at the pace of its own
// latitude (the Great Red Spot keeps its shape), not sheared like the
// bands it sits in, each latitude of which drifts with its own wind.

/** A storm: an ellipse of the map, in uv (u east from the seam, v north from the south pole, both 0 to 1). */
export interface GiantStorm {
  u: number;
  v: number;
  /** Its half-extents, in uv. */
  halfU: number;
  halfV: number;
}

/** Most storms the surface shader takes. */
export const MAX_STORMS = 16;
/** Smallest storm kept, in pixels of the map: smaller specks drift with their band. */
const MIN_PIXELS = 12;

/**
 * The storms of a biome map (`ids`, row 0 north, `width` × `height`):
 * each patch of `stormId` pixels joined by their sides (across the seam
 * too), as its centre and half-extents, the largest first, `MAX_STORMS`
 * at most.
 */
export function giantStorms(ids: Uint8Array, width: number, height: number, stormId: number): GiantStorm[] {
  const seen = new Uint8Array(width * height);
  const stack = new Int32Array(width * height);
  const found: (GiantStorm & { pixels: number })[] = [];
  for (let start = 0; start < ids.length; start++) {
    if (ids[start] !== stormId || seen[start]) continue;
    // Flood the patch, keeping its columns relative to its first pixel so
    // that one across the seam stays whole.
    const x0 = start % width;
    let top = 0;
    stack[top++] = start;
    seen[start] = 1;
    let pixels = 0;
    let minDx = 0;
    let maxDx = 0;
    let minY = height;
    let maxY = -1;
    let sumDx = 0;
    let sumY = 0;
    while (top > 0) {
      const i = stack[--top]!;
      const x = i % width;
      const y = (i - x) / width;
      let dx = x - x0;
      if (dx > width / 2) dx -= width;
      if (dx < -width / 2) dx += width;
      pixels += 1;
      sumDx += dx;
      sumY += y;
      minDx = Math.min(minDx, dx);
      maxDx = Math.max(maxDx, dx);
      minY = Math.min(minY, y);
      maxY = Math.max(maxY, y);
      const neighbours = [y * width + ((x + 1) % width), y * width + ((x - 1 + width) % width), i - width, i + width];
      for (const [k, n] of neighbours.entries()) {
        if (k === 2 && y === 0) continue;
        if (k === 3 && y === height - 1) continue;
        if (ids[n] === stormId && !seen[n]) {
          seen[n] = 1;
          stack[top++] = n;
        }
      }
    }
    if (pixels < MIN_PIXELS) continue;
    const cx = (((x0 + sumDx / pixels) % width) + width) % width;
    const cy = sumY / pixels;
    found.push({
      u: (cx + 0.5) / width,
      v: 1 - (cy + 0.5) / height,
      halfU: (maxDx - minDx + 1) / 2 / width,
      halfV: (maxY - minY + 1) / 2 / height,
      pixels,
    });
  }
  return found
    .sort((a, b) => b.pixels - a.pixels)
    .slice(0, MAX_STORMS)
    .map(({ u, v, halfU, halfV }) => ({ u, v, halfU, halfV }));
}

/**
 * Index of the storm that is a giant's great spot: the one nearest the spot
 * the generator placed (`great_spot_at`, longitude and latitude in degrees),
 * within 15° of it; -1 when the giant has none or no storm lies there. The
 * largest storm is not told apart this way: a white oval can tie with it.
 */
export function greatSpotIndex(storms: readonly GiantStorm[], at: readonly [number, number] | null): number {
  if (!at) return -1;
  let best = -1;
  let bestDistance = 15;
  storms.forEach((s, k) => {
    let dLon = Math.abs(s.u * 360 - at[0]) % 360;
    if (dLon > 180) dLon = 360 - dLon;
    const dLat = (s.v - 0.5) * 180 - at[1];
    const d = Math.hypot(dLon * Math.cos((at[1] * Math.PI) / 180), dLat);
    if (d < bestDistance) {
      bestDistance = d;
      best = k;
    }
  });
  return best;
}
