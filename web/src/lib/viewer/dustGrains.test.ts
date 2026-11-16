import { Matrix3, NormalBlending, Vector3 } from 'three';
import { describe, expect, it } from 'vitest';
import { DUST_RING_GAIN_MAX } from './dustPhase';
import {
  CLOUD_ALPHA,
  CLOUD_LEVEL,
  CLOUD_LOOK,
  CLOUD_MAX_PX,
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
  hostAxesMatrix,
  MOON_CLOUD_GRAINS,
  MOON_CORE_SIGMA,
  MOON_TRAIL_RAD,
  moonCloudGrains,
  toHostFrame,
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

describe('moon cloud grains', () => {
  it('is deterministic for a seed and changes with it', () => {
    expect(moonCloudGrains(100, 3)).toEqual(moonCloudGrains(100, 3));
    expect(moonCloudGrains(100, 3)).not.toEqual(moonCloudGrains(100, 4));
  });

  it('packs round the moon and thins smoothly along the orbit, ahead and behind', () => {
    const n = 20000;
    const g = moonCloudGrains(n, 11);
    expect(g).toHaveLength(3 * n);
    const bins = [0, 0, 0, 0];
    let ahead = 0;
    let near = 0;
    let wide = 0;
    for (let i = 0; i < n; i += 1) {
      const [along, radial, up] = [g[3 * i]!, g[3 * i + 1]!, g[3 * i + 2]!];
      const a = Math.abs(along);
      if (a < 3 * MOON_CORE_SIGMA) near += 1;
      if (along > 3 * MOON_CORE_SIGMA) ahead += 1;
      if (a < 4 * MOON_TRAIL_RAD) bins[Math.floor(a / MOON_TRAIL_RAD)]! += 1;
      if (Math.abs(radial) > 0.2 || Math.abs(up) > 0.2) wide += 1;
    }
    // Dense at the moon, as many ahead as behind, then falling about e
    // times per scale (past the first, which holds the core too).
    expect(near / n).toBeGreaterThan(0.4);
    expect(Math.abs(ahead / (n - near) - 0.5)).toBeLessThan(0.03);
    expect(bins[0]!).toBeGreaterThan(3 * bins[1]!);
    for (let k = 2; k < 4; k += 1) {
      expect(bins[k]! / bins[k - 1]!).toBeGreaterThan(0.25);
      expect(bins[k]! / bins[k - 1]!).toBeLessThan(0.5);
    }
    // Close to the orbit.
    expect(wide / n).toBeLessThan(0.01);
  });

  it('draws a few hundred fine dots that blend and never saturate', () => {
    expect(MOON_CLOUD_GRAINS).toBeLessThanOrEqual(500);
    expect(CLOUD_MAX_PX).toBeLessThanOrEqual(1.5);
    // Back-lit the brightest grain stays under full opacity (no widening),
    // and however many overlap a pixel tends to CLOUD_LEVEL of the colour.
    expect(grainLight(1, 1, CLOUD_ALPHA).opacity).toBeLessThan(1);
    expect(CLOUD_LEVEL).toBeLessThan(0.9);
    const u = { uSunDir: { value: null }, uLightMode: { value: 1 } };
    const points = createGrains(moonCloudGrains(10, 1), grainShine(10, 1), [1, 1, 1], u, CLOUD_LOOK, true);
    expect(points.material.blending).toBe(NormalBlending);
    expect(points.material.uniforms['uOrbit']!.value).toBe(1);
    expect(points.material.uniforms['uLevel']!.value).toBe(CLOUD_LEVEL);
    expect(points.material.fragmentShader).toContain('uColour * uLevel');
  });
});

describe("the moon cloud in the giant's frame", () => {
  // A giant at (10, 0, 0), radius 4, tipped: its spin axis along the scene's x.
  const axes: [[number, number, number], [number, number, number], [number, number, number]] = [
    [0, -1, 0],
    [1, 0, 0],
    [0, 0, 1],
  ];
  it("is the identity in the giant's own view", () => {
    const identity: typeof axes = [
      [1, 0, 0],
      [0, 1, 0],
      [0, 0, 1],
    ];
    expect(toHostFrame([3, 0.5, -2], [0, 0, 0], 1, identity)).toEqual([3, 0.5, -2]);
  });
  it("takes a moon into the giant's radii and axes, and the shader takes it back", () => {
    const moon: [number, number, number] = [10, 0, 16];
    const local = toHostFrame(moon, [10, 0, 0], 4, axes);
    expect(local[0]).toBeCloseTo(0, 9);
    expect(local[1]).toBeCloseTo(0, 9);
    expect(local[2]).toBeCloseTo(4, 9);
    // uHostCentre + uHostScale * uHostAxes * p, as the grain shader does.
    const back = new Vector3(...local)
      .applyMatrix3(new Matrix3().set(...hostAxesMatrix(axes)))
      .multiplyScalar(4)
      .add(new Vector3(10, 0, 0));
    expect(back.toArray().map((v) => Math.round(v * 1e9) / 1e9)).toEqual(moon);
    const off = toHostFrame([14, 2, 0], [10, 0, 0], 4, axes);
    const again = new Vector3(...off)
      .applyMatrix3(new Matrix3().set(...hostAxesMatrix(axes)))
      .multiplyScalar(4)
      .add(new Vector3(10, 0, 0));
    expect(again.toArray().map((v) => Math.round(v * 1e9) / 1e9)).toEqual([14, 2, 0]);
  });
});
