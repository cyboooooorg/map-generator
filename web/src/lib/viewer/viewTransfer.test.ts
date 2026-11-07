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
  morphFrame,
  morphPoint,
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

describe('morphPoint', () => {
  const close = (a: readonly number[], b: readonly number[]) => a.forEach((v, i) => expect(v).toBeCloseTo(b[i]!, 6));

  it('is the map at 0 and the globe at 1, whatever the centre', () => {
    for (const centre of [0, 1, Math.PI, 4, -2]) {
      for (const p of [{ lon: 0.3, lat: 0.2 }, { lon: 3, lat: -1.1 }, { lon: 6, lat: 0.7 }]) {
        close(morphPoint(p, 0, centre), mapPoint(p));
        close(morphPoint(p, 1, centre), lonLatToSphere(p));
      }
    }
  });

  it('wraps the map round the globe, its edges going back, not folding towards the viewer', () => {
    // Map centred on longitude π: halfway, the camera looks along the
    // centre's turned normal; the centre is nearest it, the edges behind.
    const morph = 0.5;
    const turn = Math.PI * morph;
    const towards = [Math.sin(turn), 0, Math.cos(turn)];
    const depth = (p: readonly number[]) => p[0]! * towards[0]! + p[2]! * towards[2]!;
    const centre = depth(morphPoint({ lon: Math.PI, lat: 0 }, morph, Math.PI));
    for (const lon of [0.01, 1, 2, 4.3, 6.2]) {
      expect(depth(morphPoint({ lon, lat: 0 }, morph, Math.PI))).toBeLessThan(centre);
    }
  });

  it('moves smoothly', () => {
    const p = { lon: 2, lat: 0.4 };
    let last = morphPoint(p, 0, 1);
    for (let k = 1; k <= 100; k++) {
      const next = morphPoint(p, k / 100, 1);
      expect(Math.hypot(next[0]! - last[0]!, next[1]! - last[1]!, next[2]! - last[2]!)).toBeLessThan(0.1);
      last = next;
    }
  });
});

describe('morphCamera clipping', () => {
  // PlanetView sets near = 0.1 × d and far = d + 4 from the camera's
  // distance d to the followed point: the whole sheet must stay inside,
  // also zoomed out, where the unrolling carries the camera far away.
  it('keeps the whole sheet between the near and far planes through the morph', () => {
    const wrapLon = 0.7;
    for (const [mapHeight, altitude] of [[12, 7], [5.5, 2.2], [1, 0.3]] as const) {
      for (let i = 0; i <= 100; i++) {
        const morph = i / 100;
        const centre = { lon: wrapLon, lat: 0.2 };
        const { position, target, distance } = morphCamera(centre, mapHeight, altitude, morph, wrapLon);
        const view = [target[0] - position[0], target[1] - position[1], target[2] - position[2]].map((v) => v / distance);
        for (let lon = 0; lon < 2 * Math.PI; lon += 0.2) {
          for (let lat = -Math.PI / 2; lat <= Math.PI / 2; lat += 0.2) {
            const front = morphFrame({ lon, lat }, morph, wrapLon);
            const p = front.point;
            const depth = (p[0] - position[0]) * view[0]! + (p[1] - position[1]) * view[1]! + (p[2] - position[2]) * view[2]!;
            // Only the points facing the camera are drawn.
            const facing = front.normal[0] * -view[0]! + front.normal[1] * -view[1]! + front.normal[2] * -view[2]! > 0;
            if (!facing) continue;
            expect(depth).toBeLessThanOrEqual(distance + 4);
            expect(depth).toBeGreaterThanOrEqual(distance * 0.1);
          }
        }
      }
    }
  });
});
