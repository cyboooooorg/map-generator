import { describe, expect, it } from 'vitest';
import {
  formatLonLat,
  lonLatToPixel,
  lonLatToSphere,
  planeToLonLat,
  rayPlaneZ0,
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
});
