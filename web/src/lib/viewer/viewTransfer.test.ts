import { describe, expect, it } from 'vitest';
import { lonLatToSphere } from '../geo';
import {
  altitudeForMapHeight,
  clampMapCentre,
  GLOBE_DISTANCE,
  lerpLonLat,
  MAP_DISTANCE,
  mapHeightForAltitude,
  mapPoint,
  morphCamera,
} from './viewTransfer';

const close = (actual: readonly number[], expected: readonly number[]) =>
  actual.forEach((value, i) => expect(value).toBeCloseTo(expected[i]!, 9));

describe('altitude and map height', () => {
  it('match the whole globe with the whole map', () => {
    expect(mapHeightForAltitude(GLOBE_DISTANCE - 1)).toBeCloseTo(MAP_DISTANCE, 12);
    expect(altitudeForMapHeight(MAP_DISTANCE)).toBeCloseTo(GLOBE_DISTANCE - 1, 12);
  });

  it('keep the scale near the ground', () => {
    expect(mapHeightForAltitude(0.001) / 0.001).toBeCloseTo(1, 2);
  });

  it('are inverse and increasing', () => {
    let previous = 0;
    for (const altitude of [0.0003, 0.01, 0.3, 1, 2.2, 7]) {
      const height = mapHeightForAltitude(altitude);
      expect(height).toBeGreaterThan(previous);
      expect(altitudeForMapHeight(height)).toBeCloseTo(altitude, 9);
      previous = height;
    }
  });
});

describe('clampMapCentre', () => {
  it('keeps a close view where it is', () => {
    expect(clampMapCentre(-3, 1.4, 0.1, 0.1)).toEqual([-3, 1.4]);
  });

  it('keeps the map filling the view', () => {
    close(clampMapCentre(-3.1, 1.5, 1, 0.5), [-Math.PI + 1, Math.PI / 2 - 0.5]);
  });

  it('centres an axis where the whole map fits', () => {
    expect(clampMapCentre(-3, 1, 4, 2)).toEqual([0, 0]);
  });
});

describe('lerpLonLat', () => {
  it('goes the short way round', () => {
    const half = lerpLonLat({ lon: 6, lat: 0 }, { lon: 0.2, lat: 0.4 }, 0.5);
    expect(half.lon).toBeCloseTo(6 + (0.2 + 2 * Math.PI - 6) / 2, 12);
    expect(half.lat).toBeCloseTo(0.2, 12);
  });
});

describe('morphCamera', () => {
  const centre = { lon: 2.5, lat: -0.6 };

  it('looks down on the map point from the map height', () => {
    const { target, position } = morphCamera(centre, 0.4, 0.2, 0);
    close(target, mapPoint(centre));
    close(position, [centre.lon - Math.PI, centre.lat, 0.4]);
  });

  it('looks at the globe point from the altitude, along its normal', () => {
    const { target, position } = morphCamera(centre, 0.4, 0.2, 1);
    const sphere = lonLatToSphere(centre);
    close(target, sphere);
    close(position, sphere.map((c) => c * 1.2));
  });

  it('stays at a distance between both in the middle', () => {
    const { target, position } = morphCamera({ lon: Math.PI, lat: 0 }, 0.4, 0.1, 0.5);
    const distance = Math.hypot(...position.map((c, i) => c - target[i]!));
    expect(distance).toBeCloseTo(0.2, 9);
  });
});
