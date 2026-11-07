// Keeping the place across the globe ↔ map switch: the point at the centre
// of the screen stays there, at an equivalent zoom, and the camera follows
// it while the surface morphs (see the vertex shader in shaders.ts).
import { type LonLat, lonLatToSphere, type Vec3 } from '../geo';

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
function lonDelta(a: number, b: number): number {
  const d = (((b - a) % (2 * Math.PI)) + 2 * Math.PI) % (2 * Math.PI);
  return d > Math.PI ? d - 2 * Math.PI : d;
}

/** The point at a fraction `t` of the way from `a` to `b` (the short way round in longitude). */
export function lerpLonLat(a: LonLat, b: LonLat, t: number): LonLat {
  return { lon: a.lon + lonDelta(a.lon, b.lon) * t, lat: a.lat + (b.lat - a.lat) * t };
}

/**
 * The camera looking at `centre` on the surface morphed by `morph` (0 =
 * map, 1 = globe), from `mapHeight` above the map or `altitude` above the
 * globe, in between at a log-interpolated distance. The target is the
 * morphed point itself; the view direction turns from the map's (+z) to
 * the globe's outward normal by longitude, then latitude, so north stays
 * up throughout.
 */
export function morphCamera(
  centre: LonLat,
  mapHeight: number,
  altitude: number,
  morph: number,
): { target: Vec3; position: Vec3 } {
  const sphere = lonLatToSphere(centre);
  const plane = mapPoint(centre);
  const target: Vec3 = [
    plane[0] + (sphere[0] - plane[0]) * morph,
    plane[1] + (sphere[1] - plane[1]) * morph,
    plane[2] + (sphere[2] - plane[2]) * morph,
  ];
  const direction = lonLatToSphere({ lon: lonDelta(0, centre.lon) * morph, lat: centre.lat * morph });
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
