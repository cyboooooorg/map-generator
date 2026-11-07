import { describe, expect, it } from 'vitest';
import {
  distanceKm,
  formatLonLat,
  greatCircleArc,
  lonLatDegrees,
  lonLatToUv,
  pathKm,
  lonLatToPixel,
  lonLatToSphere,
  planeToLonLat,
  rayPlaneZ0,
  rayRelief,
  raySphere,
  sphereToLonLat,
  uvToLonLat,
} from './geo';

describe('geo', () => {
  it('maps the texture corners to longitude 0 and the poles', () => {
    expect(uvToLonLat(0, 1)).toEqual({ lon: 0, lat: Math.PI / 2 });
    expect(uvToLonLat(0.5, 0.5).lon).toBeCloseTo(Math.PI);
  });

  it('round-trips between the sphere and longitude/latitude', () => {
    for (const point of [
      { lon: 0.3, lat: 0.2 },
      { lon: 4.0, lat: -1.1 },
      { lon: 6.2, lat: 0 },
    ]) {
      const back = sphereToLonLat(lonLatToSphere(point));
      expect(back.lon).toBeCloseTo(point.lon);
      expect(back.lat).toBeCloseTo(point.lat);
    }
  });

  it('puts longitude 0 in front of the camera, north up', () => {
    const [x, y, z] = lonLatToSphere({ lon: 0, lat: 0 });
    expect([x, y, z].map((c) => Math.round(c))).toEqual([0, 0, 1]);
    expect(lonLatToSphere({ lon: 0, lat: Math.PI / 2 })[1]).toBeCloseTo(1);
  });

  it('finds the pixel of a point, row 0 being north', () => {
    expect(lonLatToPixel({ lon: 0, lat: Math.PI / 2 }, 200, 100)).toEqual([0, 0]);
    expect(lonLatToPixel({ lon: Math.PI, lat: 0 }, 200, 100)).toEqual([100, 50]);
    expect(lonLatToPixel({ lon: 6.28, lat: -Math.PI / 2 }, 200, 100)).toEqual([199, 99]);
  });

  it('intersects rays with the globe and the flat map', () => {
    expect(raySphere([0, 0, 3], [0, 0, -1], 1)).toBeCloseTo(2);
    expect(raySphere([0, 3, 3], [0, 0, -1], 1)).toBeNull();
    expect(rayPlaneZ0([1, 1, 5], [0, 0, -1])).toBeCloseTo(5);
    expect(planeToLonLat(0, 0)?.lat).toBeCloseTo(0);
    expect(planeToLonLat(10, 0)).toBeNull();
  });

  it('formats coordinates with hemispheres', () => {
    expect(formatLonLat({ lon: (350 * Math.PI) / 180, lat: (-12.34 * Math.PI) / 180 })).toBe(
      '12.3° S, 10.0° W',
    );
  });

  it('measures great-circle distances on a planet of any size', () => {
    const rad = Math.PI / 180;
    // A quarter of the equator, and pole to equator, are a quarter of the circumference.
    expect(distanceKm({ lon: 0, lat: 0 }, { lon: 90 * rad, lat: 0 }, 40_000)).toBeCloseTo(10_000, 6);
    expect(distanceKm({ lon: 0, lat: Math.PI / 2 }, { lon: 1, lat: 0 }, 40_000)).toBeCloseTo(10_000, 6);
    // Across the date line, the short way.
    expect(distanceKm({ lon: 359 * rad, lat: 0 }, { lon: 1 * rad, lat: 0 }, 36_000)).toBeCloseTo(200, 6);
    expect(
      pathKm(
        [
          { lon: 0, lat: 0 },
          { lon: 10 * rad, lat: 0 },
          { lon: 10 * rad, lat: 10 * rad },
        ],
        36_000,
      ),
    ).toBeCloseTo(2000, 6);
  });

  it('samples great-circle arcs between their end points', () => {
    const a = { lon: 0, lat: 0 };
    const b = { lon: Math.PI / 2, lat: 0 };
    const arc = greatCircleArc(a, b, 4);
    expect(arc).toHaveLength(5);
    expect(arc[2]!.lon).toBeCloseTo(Math.PI / 4);
    expect(arc[2]!.lat).toBeCloseTo(0);
  });

  it('converts to texture coordinates and degrees', () => {
    expect(lonLatToUv({ lon: Math.PI, lat: 0 })).toEqual([0.5, 0.5]);
    const [lat, lon] = lonLatDegrees({ lon: (270 * Math.PI) / 180, lat: Math.PI / 4 });
    expect(lat).toBeCloseTo(45);
    expect(lon).toBeCloseTo(-90);
  });

  describe('rayRelief', () => {
    // A cone-shaped mountain 0.1 radii high, 0.3 radii wide, on the +x axis.
    const mountain = (x: number, y: number, z: number): number => {
      const d = Math.hypot(x - 1, y, z);
      return 1 + Math.max(0, 0.1 * (1 - d / 0.3));
    };
    const flat = () => 1;

    it('gives the sphere hit on a flat surface', () => {
      const origin: [number, number, number] = [0.2, 0.1, 5];
      const direction: [number, number, number] = [0, 0, -1];
      const t = rayRelief(origin, direction, flat, 1.05);
      expect(t).toBeCloseTo(raySphere(origin, direction, 1)!, 6);
    });

    it('misses when the ray passes outside the relief', () => {
      expect(rayRelief([0, 2, 5], [0, 0, -1], mountain, 1.1)).toBeNull();
    });

    it('hits the flank of a mountain a grazing ray meets before the sphere', () => {
      // Parallel to the ground, 0.04 above the sphere at x = 1: the plain
      // sphere test would miss it or land far behind.
      const origin: [number, number, number] = [1.04, 0, 5];
      const direction: [number, number, number] = [0, 0, -1];
      const t = rayRelief(origin, direction, mountain, 1.1)!;
      expect(t).not.toBeNull();
      const z = 5 - t;
      expect(z).toBeGreaterThan(0.05);
      const point = [origin[0], 0, z];
      const r = Math.hypot(...point);
      const [x, y, zz] = point.map((v) => v / r) as [number, number, number];
      expect(r).toBeCloseTo(mountain(x, y, zz), 4);
      const sphere = raySphere(origin, direction, 1);
      expect(sphere === null || t < sphere).toBe(true);
    });

    it('hits the summit rather than the sphere behind it when looking straight down', () => {
      const t = rayRelief([5, 0, 0], [-1, 0, 0], mountain, 1.1)!;
      expect(5 - t).toBeCloseTo(1.1, 3);
    });
  });
});
