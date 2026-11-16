import { describe, expect, it } from 'vitest';
import { DUST_RING_GAIN_MAX } from './dustPhase';
import {
  createGrains,
  GRAIN_SHINE_MIN,
  grainCount,
  grainLight,
  grainShine,
  HALO_ALPHA,
  HALO_INNER,
  HALO_MAX_PX,
  HALO_OUTER,
  HALO_POLE_HALF_ANGLE,
  haloGrains,
} from './dustGrains';

describe('dust halo grains', () => {
  it('is deterministic for a seed and changes with it', () => {
    expect(haloGrains(100, 3, [0, -1, 0])).toEqual(haloGrains(100, 3, [0, -1, 0]));
    expect(haloGrains(100, 3, null)).not.toEqual(haloGrains(100, 4, null));
    expect(grainShine(100, 3)).toEqual(grainShine(100, 3));
  });

  it('keeps the halo in its shell, denser near the surface and over the venting pole', () => {
    const n = 4000;
    const pole = [0, -1, 0] as const;
    const g = haloGrains(n, 5, pole);
    let low = 0;
    let high = 0;
    let polar = 0;
    for (let i = 0; i < n; i += 1) {
      const [x, y, z] = [g[3 * i]!, g[3 * i + 1]!, g[3 * i + 2]!];
      const r = Math.hypot(x, y, z);
      expect(r).toBeGreaterThanOrEqual(HALO_INNER - 1e-6);
      expect(r).toBeLessThanOrEqual(HALO_OUTER + 1e-6);
      if (r < 1.5) low += 1;
      if (r > 2.5) high += 1;
      if (-y / r > Math.cos(HALO_POLE_HALF_ANGLE)) polar += 1;
    }
    expect(low).toBeGreaterThan(3 * high);
    // The cap is 9 % of the sphere; with the polar share, about half the grains.
    expect(polar / n).toBeGreaterThan(0.35);
    const even = haloGrains(n, 5, null);
    let evenPolar = 0;
    for (let i = 0; i < n; i += 1) {
      const r = Math.hypot(even[3 * i]!, even[3 * i + 1]!, even[3 * i + 2]!);
      if (-even[3 * i + 1]! / r > Math.cos(HALO_POLE_HALF_ANGLE)) evenPolar += 1;
    }
    expect(evenPolar / n).toBeLessThan(0.15);
  });

  it('gives each grain a shine in range and scales the counts with quality', () => {
    const s = grainShine(1000, 2);
    expect(Math.min(...s)).toBeGreaterThanOrEqual(GRAIN_SHINE_MIN);
    expect(Math.max(...s)).toBeLessThanOrEqual(1);
    expect(grainCount(2500, 1)).toBe(2500);
    expect(grainCount(2500, 0.4)).toBe(1000);
    expect(grainCount(10, 0)).toBe(1);
  });

  it('keeps grains seen front-lit and only a little brighter back-lit', () => {
    // Light in all: opacity times the dot's area.
    const total = (cos: number, shine: number) => {
      const { opacity, sizeScale } = grainLight(cos, shine);
      return opacity * sizeScale ** 2;
    };
    // Front-lit (cosScatter −1) as at 90°.
    expect(total(-1, 1)).toBeCloseTo(total(0, 1));
    expect(grainLight(-1, GRAIN_SHINE_MIN).opacity).toBeGreaterThan(0.3);
    // Back-lit: the dust rings' cap times the front-lit light, widened past full opacity.
    expect(total(1, 1) / total(-1, 1)).toBeCloseTo(DUST_RING_GAIN_MAX);
    expect(grainLight(1, 1, 1).opacity).toBe(1);
    expect(grainLight(1, 1, 1).sizeScale).toBeCloseTo(Math.sqrt(DUST_RING_GAIN_MAX));
    for (let c = -1; c <= 1; c += 0.05) {
      expect(total(c, 0.7)).toBeLessThanOrEqual(DUST_RING_GAIN_MAX * HALO_ALPHA * 0.7 + 1e-9);
    }
  });

  it("draws fine additive points with the dust rings' phase gain", () => {
    const u = { uSunDir: { value: null }, uLightMode: { value: 1 } };
    const points = createGrains(haloGrains(10, 1, null), grainShine(10, 1), [1, 1, 1], u);
    expect(points.geometry.getAttribute('position').count).toBe(10);
    expect(points.material.depthWrite).toBe(false);
    expect(points.material.uniforms['uSunDir']).toBe(u.uSunDir);
    expect(points.material.vertexShader).toContain('dustRingGain(-dot(uSunDir, normalize(cameraPosition - p)))');
    expect(points.material.uniforms['uAlpha']!.value).toBe(HALO_ALPHA);
    expect(points.material.uniforms['uMaxPx']!.value).toBe(HALO_MAX_PX);
    expect(HALO_MAX_PX).toBeLessThanOrEqual(2);
    expect(points.material.fragmentShader).toContain('#include <colorspace_fragment>');
  });
});
