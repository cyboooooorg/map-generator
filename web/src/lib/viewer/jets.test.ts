import { describe, expect, it } from 'vitest';
import { fracturePoints, jetFragment, jetSources, JETS_PER_FRACTURE } from './jets';

describe('jets', () => {
  it('puts the jets on the fracture, between its ends', () => {
    const f = { lat_deg: -80, lon_deg: 30, azimuth_deg: 90, length_km: 130 };
    const pts = fracturePoints(f, 252.1, 24);
    expect(pts).toHaveLength(24);
    const mid = pts[11]!;
    expect(Math.abs((mid.lat * 180) / Math.PI + 80)).toBeLessThan(0.5);
  });
  it('handles the fracture through the pole and the seam', () => {
    const pole = fracturePoints({ lat_deg: -90, lon_deg: 45, azimuth_deg: 0, length_km: 130 }, 252.1, 24);
    expect(pole.every((p) => p.lat < -1.2)).toBe(true);
    const seam = fracturePoints({ lat_deg: -82, lon_deg: 179, azimuth_deg: 90, length_km: 130 }, 252.1, 24);
    expect(seam.every((p) => Number.isFinite(p.lon))).toBe(true);
  });
  it('gives no jets without geysers', () => {
    expect(jetSources({ south: true, fractures: [] }, 252.1)).toEqual([]);
  });
  it('spans the fracture length and counts jets per fracture', () => {
    const f = { lat_deg: 0, lon_deg: 0, azimuth_deg: 0, length_km: 100 };
    const pts = fracturePoints(f, 100, 10);
    // Along north at the equator: longitude stays 0, latitude spans (almost) 1 rad.
    expect(pts[0]!.lat).toBeCloseTo(-0.45, 6);
    expect(pts[9]!.lat).toBeCloseTo(0.45, 6);
    expect(jetSources({ south: false, fractures: [f, f] }, 100)).toHaveLength(2 * JETS_PER_FRACTURE);
  });
  it('skips a zero-length fracture', () => {
    const none = { lat_deg: -85, lon_deg: 0, azimuth_deg: 0, length_km: 0 };
    const some = { lat_deg: -85, lon_deg: 0, azimuth_deg: 0, length_km: 50 };
    expect(jetSources({ south: true, fractures: [none, some] }, 252.1)).toHaveLength(JETS_PER_FRACTURE);
  });
  it('outputs a straight colour with its alpha, colour-space encoded', () => {
    expect(jetFragment).toContain('vec4(vec3(0.92, 0.96, 1.0), a)');
    expect(jetFragment).toContain('#include <colorspace_fragment>');
    expect(jetFragment).not.toMatch(/\* a, a\)/);
  });
});
