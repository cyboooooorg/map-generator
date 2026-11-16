import { describe, expect, it } from 'vitest';
import { fracturePoints, jetFragment, jetSources, JETS_PER_FRACTURE, meanderKm } from './jets';

/** Angle, radians, between two points given in latitude and longitude. */
function angle(a: { lat: number; lon: number }, b: { lat: number; lon: number }): number {
  const u = (p: { lat: number; lon: number }) => [
    Math.cos(p.lat) * Math.cos(p.lon),
    Math.cos(p.lat) * Math.sin(p.lon),
    Math.sin(p.lat),
  ];
  const [x, y] = [u(a), u(b)];
  return Math.acos(Math.min(1, x[0]! * y[0]! + x[1]! * y[1]! + x[2]! * y[2]!));
}

describe('jets', () => {
  it('puts the jets on the fracture, between its ends', () => {
    const f = { lat_deg: -80, lon_deg: 30, azimuth_deg: 90, length_km: 130 };
    const pts = fracturePoints(f, 0, 252.1, 24);
    expect(pts).toHaveLength(24);
    // Near the centre, off its latitude by the meander there (striking
    // east, the line moves along the meridian).
    const mid = pts[11]!;
    const meanderDeg = (meanderKm(0, -0.5 + 11.5 / 24, 130) / 252.1) * (180 / Math.PI);
    expect(Math.abs(Math.abs((mid.lat * 180) / Math.PI + 80) - Math.abs(meanderDeg))).toBeLessThan(0.1);
  });
  it('handles the fracture through the pole and the seam', () => {
    const pole = fracturePoints({ lat_deg: -90, lon_deg: 45, azimuth_deg: 0, length_km: 130 }, 0, 252.1, 24);
    expect(pole.every((p) => p.lat < -1.2)).toBe(true);
    const seam = fracturePoints({ lat_deg: -82, lon_deg: 179, azimuth_deg: 90, length_km: 130 }, 1, 252.1, 24);
    expect(seam.every((p) => Number.isFinite(p.lon))).toBe(true);
  });
  it('gives no jets without geysers', () => {
    expect(jetSources({ south: true, fractures: [] }, 252.1)).toEqual([]);
  });
  it('spans the fracture length and counts jets per fracture', () => {
    const f = { lat_deg: 0, lon_deg: 0, azimuth_deg: 0, length_km: 100 };
    const pts = fracturePoints(f, 0, 100, 10);
    // Along north at the equator: latitude spans (almost) 1 rad, each point
    // moved sideways (west, centre × north) by the meander.
    expect(pts[0]!.lat).toBeCloseTo(-0.45, 2);
    expect(pts[9]!.lat).toBeCloseTo(0.45, 2);
    pts.forEach((p, i) => {
      const along = -0.5 + (i + 0.5) / 10;
      const sideways = -Math.asin(Math.cos(p.lat) * Math.sin(p.lon)) * 100;
      expect(sideways).toBeCloseTo(meanderKm(0, along, 100), 6);
    });
    expect(jetSources({ south: false, fractures: [f, f] }, 100)).toHaveLength(2 * JETS_PER_FRACTURE);
  });
  it('skips a zero-length fracture, the others keeping their index', () => {
    const none = { lat_deg: -85, lon_deg: 0, azimuth_deg: 0, length_km: 0 };
    const some = { lat_deg: -85, lon_deg: 0, azimuth_deg: 0, length_km: 50 };
    const sources = jetSources({ south: true, fractures: [none, some] }, 252.1);
    expect(sources).toEqual(fracturePoints(some, 1, 252.1, JETS_PER_FRACTURE));
  });
  it('matches its Rust twin meander_km', () => {
    // Pinned in crates/mapgen/src/planet/geysers.rs too.
    const cases: [number, number, number, number][] = [
      [0, 0.0, 130.0, 5.73541341829182],
      [1, 0.25, 130.0, -0.6613252465646292],
      [2, -0.45, 130.0, 0.6434323178558201],
      [3, 0.5, 130.0, -11.46808168745059],
      [5, -0.1, 40.0, -2.635000805293958],
      [11, 0.49, 3000.0, -10.857832404480899],
    ];
    for (const [index, along, length, want] of cases) {
      expect(Math.abs(meanderKm(index, along, length) - want)).toBeLessThan(1e-9);
    }
  });
  it('bends neighbouring fractures differently', () => {
    const f = { lat_deg: -80, lon_deg: 30, azimuth_deg: 90, length_km: 130 };
    const [a, b] = [fracturePoints(f, 1, 252.1, 24), fracturePoints(f, 2, 252.1, 24)];
    const most = Math.max(...a.map((p, i) => angle(p, b[i]!) * 252.1));
    expect(most).toBeGreaterThan(2);
  });
  it('outputs a straight colour with its alpha, colour-space encoded', () => {
    expect(jetFragment).toContain('vec4(vec3(0.92, 0.96, 1.0), a)');
    expect(jetFragment).toContain('#include <colorspace_fragment>');
    expect(jetFragment).not.toMatch(/\* a, a\)/);
  });
});
