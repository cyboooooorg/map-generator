// Coordinate conventions shared by the 3D view, picking and the GLSL shader.
//
// - Texture coordinates (u, v): u = 0 at the left edge (longitude 0), v = 1 at
//   the top (north pole), as three.js loads the equirectangular world.png.
// - Globe: unit sphere, y up, longitude 0 facing +z:
//     x = cos(lat) sin(lon), y = sin(lat), z = cos(lat) cos(lon)
// - Flat map: plane z = 0, 2π wide and π high, centred on the origin.

import { formatNumber, withUnit } from './format';
import { t } from './i18n/index.svelte';

export interface LonLat {
  /** Longitude in radians, in [0, 2π). */
  lon: number;
  /** Latitude in radians, in [-π/2, π/2]. */
  lat: number;
}

export type Vec3 = readonly [number, number, number];

const TAU = 2 * Math.PI;

function wrapLon(lon: number): number {
  const wrapped = lon % TAU;
  return wrapped < 0 ? wrapped + TAU : wrapped;
}

export function uvToLonLat(u: number, v: number): LonLat {
  return { lon: wrapLon(u * TAU), lat: (v - 0.5) * Math.PI };
}

export function lonLatToSphere({ lon, lat }: LonLat): Vec3 {
  return [Math.cos(lat) * Math.sin(lon), Math.sin(lat), Math.cos(lat) * Math.cos(lon)];
}

export function sphereToLonLat([x, y, z]: Vec3): LonLat {
  const length = Math.hypot(x, y, z);
  return {
    lon: wrapLon(Math.atan2(x, z)),
    lat: Math.asin(Math.max(-1, Math.min(1, y / length))),
  };
}

/** Point of the flat map plane (z = 0) to longitude/latitude, or null outside the map. */
export function planeToLonLat(x: number, y: number): LonLat | null {
  const u = x / TAU + 0.5;
  const v = y / Math.PI + 0.5;
  if (u < 0 || u > 1 || v < 0 || v > 1) {
    return null;
  }
  return uvToLonLat(u, v);
}

/** Pixel of a `width × height` equirectangular map (row 0 = north) containing a point. */
export function lonLatToPixel({ lon, lat }: LonLat, width: number, height: number): [number, number] {
  const x = Math.min(width - 1, Math.floor((wrapLon(lon) / TAU) * width));
  const y = Math.min(height - 1, Math.max(0, Math.floor((0.5 - lat / Math.PI) * height)));
  return [x, y];
}

/** Distance along the ray to the first hit of a sphere centred on the origin, or null. */
export function raySphere(origin: Vec3, direction: Vec3, radius: number): number | null {
  const [ox, oy, oz] = origin;
  const [dx, dy, dz] = direction;
  const a = dx * dx + dy * dy + dz * dz;
  const b = 2 * (ox * dx + oy * dy + oz * dz);
  const c = ox * ox + oy * oy + oz * oz - radius * radius;
  const discriminant = b * b - 4 * a * c;
  if (discriminant < 0) {
    return null;
  }
  const t = (-b - Math.sqrt(discriminant)) / (2 * a);
  return t >= 0 ? t : null;
}

/** Distance along the ray to the plane z = 0, or null if parallel or behind. */
export function rayPlaneZ0(origin: Vec3, direction: Vec3): number | null {
  if (Math.abs(direction[2]) < 1e-12) {
    return null;
  }
  const t = -origin[2] / direction[2];
  return t >= 0 ? t : null;
}

/** "12.3° N" / "45.0° E" style formatting; longitudes shown in [-180, 180]. */
export function formatLonLat({ lon, lat }: LonLat): string {
  const latDeg = (lat * 180) / Math.PI;
  let lonDeg = (lon * 180) / Math.PI;
  if (lonDeg > 180) {
    lonDeg -= 360;
  }
  const ns = latDeg >= 0 ? t.geo.north : t.geo.south;
  const ew = lonDeg >= 0 ? t.geo.east : t.geo.west;
  return t.geo.point({
    lat: withUnit(`${formatNumber(Math.abs(latDeg), 1)}°`, ns),
    lon: withUnit(`${formatNumber(Math.abs(lonDeg), 1)}°`, ew),
  });
}

/** Angle between two points seen from the planet centre, in radians. */
export function centralAngle(a: LonLat, b: LonLat): number {
  // Haversine: accurate for short and long distances alike.
  const dLat = b.lat - a.lat;
  const dLon = b.lon - a.lon;
  const h = Math.sin(dLat / 2) ** 2 + Math.cos(a.lat) * Math.cos(b.lat) * Math.sin(dLon / 2) ** 2;
  return 2 * Math.asin(Math.min(1, Math.sqrt(h)));
}

/** Great-circle distance between two points of a planet, in km. */
export function distanceKm(a: LonLat, b: LonLat, circumferenceKm: number): number {
  return (centralAngle(a, b) * circumferenceKm) / TAU;
}

/** Length of a path of great-circle legs, in km. */
export function pathKm(points: readonly LonLat[], circumferenceKm: number): number {
  let total = 0;
  for (let i = 1; i < points.length; i += 1) {
    total += distanceKm(points[i - 1]!, points[i]!, circumferenceKm);
  }
  return total;
}

/** `count + 1` points evenly spaced along the great circle from `a` to `b`. */
export function greatCircleArc(a: LonLat, b: LonLat, count: number): LonLat[] {
  const pa = lonLatToSphere(a);
  const pb = lonLatToSphere(b);
  const angle = centralAngle(a, b);
  const points: LonLat[] = [];
  for (let i = 0; i <= count; i += 1) {
    const t = i / count;
    if (angle < 1e-9) {
      points.push(a);
      continue;
    }
    // Spherical linear interpolation.
    const wa = Math.sin((1 - t) * angle) / Math.sin(angle);
    const wb = Math.sin(t * angle) / Math.sin(angle);
    points.push(sphereToLonLat([wa * pa[0] + wb * pb[0], wa * pa[1] + wb * pb[1], wa * pa[2] + wb * pb[2]]));
  }
  return points;
}

/** Texture coordinates (u, v) of a point; v = 1 at the north pole. */
export function lonLatToUv({ lon, lat }: LonLat): [number, number] {
  return [wrapLon(lon) / TAU, lat / Math.PI + 0.5];
}

/** Longitude and latitude in degrees, longitude in [-180, 180], for copying. */
export function lonLatDegrees({ lon, lat }: LonLat): [number, number] {
  let lonDeg = (wrapLon(lon) * 180) / Math.PI;
  if (lonDeg > 180) {
    lonDeg -= 360;
  }
  return [(lat * 180) / Math.PI, lonDeg];
}
