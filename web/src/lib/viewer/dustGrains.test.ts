import { describe, expect, it } from 'vitest';
import { DUST_SCALE_HEIGHT } from './dustPhase';
import {
  createGrains,
  GRAIN_GAIN_MAX,
  GRAIN_SHINE_MIN,
  grainCount,
  grainLight,
  grainShine,
  HALO_ALPHA,
  HALO_LOOK,
  HALO_INNER,
  HALO_OUTER,
  HALO_POLE_HALF_ANGLE,
  haloGrains,
  MOON_CORE_SIGMA,
  moonCloudGrains,
  ringGrains,
  ringProfile,
} from './dustGrains';

// Saturn's E ring, roughly: from Mimas to past Rhea, densest at Enceladus.
const E_RING = { inner: 3, outer: 8, peak: 3.95 };

describe('dust grains', () => {
  it('follows the dust ring shader profile', () => {
    expect(ringProfile(E_RING, E_RING.peak)).toBeCloseTo(1);
    expect(ringProfile(E_RING, E_RING.inner)).toBeCloseTo(0);
    expect(ringProfile(E_RING, E_RING.outer)).toBeCloseTo(0);
    expect(ringProfile(E_RING, 2.9)).toBe(0);
    expect(ringProfile(E_RING, 5)).toBeGreaterThan(ringProfile(E_RING, 7));
  });

  it('spreads ring grains inside the ring, near its plane, densest at the peak', () => {
    const g = ringGrains(E_RING, 4000, 7);
    expect(g).toHaveLength(12000);
    const bins = new Array<number>(10).fill(0);
    for (let i = 0; i < 4000; i += 1) {
      const [x, y, z] = [g[3 * i]!, g[3 * i + 1]!, g[3 * i + 2]!];
      const r = Math.hypot(x, z);
      expect(r).toBeGreaterThanOrEqual(E_RING.inner);
      expect(r).toBeLessThanOrEqual(E_RING.outer);
      expect(Math.abs(y)).toBeLessThan(6 * DUST_SCALE_HEIGHT * r);
      bins[Math.min(9, Math.floor(((r - E_RING.inner) / (E_RING.outer - E_RING.inner)) * 10))]! += 1;
    }
    // Per unit area: the bin holding the peak beats both edges.
    const perArea = bins.map((n, k) => n / (E_RING.inner + (k + 0.5) * 0.5));
    const peakBin = Math.floor(((E_RING.peak - E_RING.inner) / 5) * 10);
    expect(perArea[peakBin]!).toBeGreaterThan(3 * perArea[9]!);
    expect(perArea[peakBin]!).toBeGreaterThan(perArea[0]!);
  });

  it('is deterministic for a seed and changes with it', () => {
    expect(ringGrains(E_RING, 100, 3)).toEqual(ringGrains(E_RING, 100, 3));
    expect(ringGrains(E_RING, 100, 3)).not.toEqual(ringGrains(E_RING, 100, 4));
    expect(moonCloudGrains(100, 3)).toEqual(moonCloudGrains(100, 3));
    expect(haloGrains(100, 3, [0, -1, 0])).toEqual(haloGrains(100, 3, [0, -1, 0]));
    expect(grainShine(100, 3)).toEqual(grainShine(100, 3));
  });

  it('packs the moon cloud round the moon and trails it along the orbit', () => {
    const n = 4000;
    const g = moonCloudGrains(n, 11);
    expect(g).toHaveLength(3 * n);
    let near = 0;
    let far = 0;
    let wideAcross = 0;
    for (let i = 0; i < n; i += 1) {
      const [along, radial, up] = [g[3 * i]!, g[3 * i + 1]!, g[3 * i + 2]!];
      if (Math.abs(along) < 3 * MOON_CORE_SIGMA) near += 1;
      if (Math.abs(along) > 0.5) far += 1;
      if (Math.abs(radial) > 0.2 || Math.abs(up) > 0.2) wideAcross += 1;
    }
    // Dense at the moon, yet a trail reaches well along the orbit.
    expect(near / n).toBeGreaterThan(0.4);
    expect(far / n).toBeGreaterThan(0.1);
    expect(wideAcross).toBe(0);
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

  it('keeps grains bright front-lit and only a few times brighter back-lit', () => {
    // Light in all: opacity times the dot's area.
    const total = (cos: number, shine: number, alpha?: number) => {
      const { opacity, sizeScale } = grainLight(cos, shine, alpha);
      return opacity * sizeScale ** 2;
    };
    // Front-lit (cosScatter −1) as at 90°, even the faintest grain well seen.
    expect(total(-1, 1)).toBeCloseTo(total(0, 1));
    expect(grainLight(-1, GRAIN_SHINE_MIN).opacity).toBeGreaterThan(0.5);
    // Back-lit: opaque, widened, and 3 to 5 times the front-lit light, never more than the cap.
    const back = grainLight(1, 1);
    expect(back.opacity).toBe(1);
    expect(back.sizeScale).toBeGreaterThan(1.5);
    const ratio = total(1, 1) / total(-1, 1);
    expect(ratio).toBeGreaterThanOrEqual(3);
    expect(ratio).toBeLessThanOrEqual(GRAIN_GAIN_MAX + 1e-9);
    for (let c = -1; c <= 1; c += 0.05) expect(total(c, 0.7)).toBeLessThanOrEqual(GRAIN_GAIN_MAX * 0.95 * 0.7 + 1e-9);
    // The halo's softer dots, fainter at 90°.
    expect(grainLight(0, 1, HALO_ALPHA).opacity).toBeLessThan(grainLight(0, 1).opacity);
  });

  it('draws additive points with the clamped phase gain, the halo with its own look', () => {
    const u = { uSunDir: { value: null }, uLightMode: { value: 1 } };
    const points = createGrains(ringGrains(E_RING, 10, 1), grainShine(10, 1), [1, 1, 1], false, u);
    expect(points.geometry.getAttribute('position').count).toBe(10);
    expect(points.material.depthWrite).toBe(false);
    expect(points.material.uniforms['uSunDir']).toBe(u.uSunDir);
    expect(points.material.vertexShader).toContain(
      'clamp(dustGain(-dot(uSunDir, normalize(cameraPosition - p))), uFloor, uGainMax)',
    );
    expect(points.material.uniforms['uGainMax']!.value).toBe(GRAIN_GAIN_MAX);
    const halo = createGrains(haloGrains(10, 1, null), grainShine(10, 1), [1, 1, 1], false, u, HALO_LOOK);
    expect(halo.material.uniforms['uAlpha']!.value).toBe(HALO_LOOK.alpha);
    expect(halo.material.uniforms['uMinPx']!.value).toBeGreaterThan(points.material.uniforms['uMinPx']!.value);
    expect(points.material.fragmentShader).toContain('#include <colorspace_fragment>');
  });
});
