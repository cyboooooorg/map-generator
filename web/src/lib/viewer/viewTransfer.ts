// Keeping the place across the globe ↔ map switch: the point at the centre
// of the screen stays there, at an equivalent zoom, and the camera follows
// it while the surface morphs (see the vertex shader in shaders.ts).
import { type LonLat, type Vec3 } from '../geo';

/** Camera distances of the whole planet: the globe has radius 1, the map is 2π × π. */
export const GLOBE_DISTANCE = 3.2;
export const MAP_DISTANCE = 5.5;

/**
 * Curvature of the altitude ↔ height mapping: chosen so that the whole
 * globe (altitude GLOBE_DISTANCE − 1) maps to the whole map (height
 * MAP_DISTANCE).
 */
const K = (MAP_DISTANCE / (GLOBE_DISTANCE - 1) - 1) / (GLOBE_DISTANCE - 1);

/**
 * Height of the map camera that matches an altitude above the globe. Near
 * the ground both show the same scale (a map unit is a globe unit at the
 * equator); far away, the whole map matches the whole globe.
 */
export function mapHeightForAltitude(altitude: number): number {
  return altitude * (1 + K * altitude);
}

/** Inverse of mapHeightForAltitude. */
export function altitudeForMapHeight(height: number): number {
  return (Math.sqrt(1 + 4 * K * height) - 1) / (2 * K);
}

/** Point of the flat map (plane z = 0) at a longitude and latitude. */
export function mapPoint({ lon, lat }: LonLat): Vec3 {
  return [lon - Math.PI, lat, 0];
}

/**
 * Centre of a map view `halfWidth` × `halfHeight` (map units) as close to
 * (x, y) as possible while the map fills the view, or centred on an axis
 * where the whole map fits.
 */
export function clampMapCentre(x: number, y: number, halfWidth: number, halfHeight: number): [number, number] {
  const clamp = (value: number, extent: number, half: number) => {
    const limit = extent - half;
    return limit <= 0 ? 0 : Math.min(limit, Math.max(-limit, value));
  };
  return [clamp(x, Math.PI, halfWidth), clamp(y, Math.PI / 2, halfHeight)];
}

/** Longitude difference b − a, in (−π, π]. */
export function lonDelta(a: number, b: number): number {
  const d = (((b - a) % (2 * Math.PI)) + 2 * Math.PI) % (2 * Math.PI);
  return d > Math.PI ? d - 2 * Math.PI : d;
}

/** The point at a fraction `t` of the way from `a` to `b` (the short way round in longitude). */
export function lerpLonLat(a: LonLat, b: LonLat, t: number): LonLat {
  return { lon: a.lon + lonDelta(a.lon, b.lon) * t, lat: a.lat + (b.lat - a.lat) * t };
}

/**
 * Where a point of the surface is, and its outward normal, morphed by
 * `morph` (0 = map, 1 = globe) round the meridian `wrapLon`: the map
 * rolled onto a sphere of radius 1 / morph that touches it along that
 * meridian, the sphere bulging towards the camera (+z), so the map wraps
 * round it, its edges going back, rather than folding towards the viewer;
 * the sphere then turned by the meridian's longitude, so that it ends on
 * the globe. Mirrors the vertex shader (`morphWrap` in shaders.ts).
 */
export function morphFrame({ lon, lat }: LonLat, morph: number, wrapLon: number): { point: Vec3; normal: Vec3 } {
  const wrap = (l: number) => ((l % (2 * Math.PI)) + 2 * Math.PI) % (2 * Math.PI);
  const x = wrap(lon) - Math.PI;
  if (morph < 1e-4) {
    return { point: [x, lat, 0], normal: [0, 0, 1] };
  }
  const xc = wrap(wrapLon) - Math.PI;
  const a = (x - xc) * morph;
  const b = lat * morph;
  const r = 1 / morph;
  // R (cos b cos a − 1), without the loss of cos near 1.
  const sinHalf = (t: number) => Math.sin(t / 2) ** 2;
  const px = r * Math.cos(b) * Math.sin(a);
  const py = r * Math.sin(b);
  const pz = -2 * r * (sinHalf(b) * Math.cos(a) + sinHalf(a)) + morph;
  const turn = lonDelta(0, wrapLon) * morph;
  const [c, s] = [Math.cos(turn), Math.sin(turn)];
  const [nx, ny, nz] = [Math.cos(b) * Math.sin(a), Math.sin(b), Math.cos(b) * Math.cos(a)];
  return {
    point: [px * c + pz * s + (1 - morph) * xc, py, -px * s + pz * c],
    normal: [nx * c + nz * s, ny, -nx * s + nz * c],
  };
}

/** The point of morphFrame. */
export function morphPoint(point: LonLat, morph: number, wrapLon: number): Vec3 {
  return morphFrame(point, morph, wrapLon).point;
}

/**
 * The camera looking at `centre` on the surface morphed by `morph` (0 =
 * map, 1 = globe) round the meridian `wrapLon` (the centre's own by
 * default), from `mapHeight` above the map or `altitude` above the globe,
 * in between at a log-interpolated distance. The target is the morphed
 * point itself, seen along the surface's normal there, which turns from
 * the map's (+z) to the globe's by longitude, then latitude, so north
 * stays up throughout.
 */
export function morphCamera(
  centre: LonLat,
  mapHeight: number,
  altitude: number,
  morph: number,
  wrapLon = centre.lon,
): { target: Vec3; position: Vec3 } {
  const { point: target, normal: direction } = morphFrame(centre, morph, wrapLon);
  const distance = Math.exp(Math.log(mapHeight) + (Math.log(altitude) - Math.log(mapHeight)) * morph);
  return {
    target,
    position: [
      target[0] + direction[0] * distance,
      target[1] + direction[1] * distance,
      target[2] + direction[2] * distance,
    ],
  };
}
