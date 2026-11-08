import { describe, expect, it } from 'vitest';
import type { SkyBody, Vec3 } from './skyBodies';
import {
  NIGHT_CEILING,
  NIGHT_FLOOR,
  lambertPhase,
  nightAt,
  nightBrightness,
  nightSources,
  reflectedRatio,
} from './nightLight';

const SUN: Vec3 = [0, 0, 1];

/** A body of the sky straight behind the observer's night (opposite the sun): full. */
const body = (diameterDeg: number, extra: Partial<SkyBody> = {}): SkyBody => ({
  direction: [0, 0, -1],
  angularDiameterDeg: diameterDeg,
  colour: [0.3, 0.3, 0.3],
  maps: null,
  axes: null,
  ...extra,
});

/** Angular diameter of a body of radius r (km) at distance d (km), degrees. */
const diameter = (r: number, d: number) => (2 * Math.asin(r / d) * 180) / Math.PI;

describe('lambertPhase', () => {
  it('is 1 at full, 0 at new and 1/pi at quarter', () => {
    expect(lambertPhase(0)).toBeCloseTo(1, 12);
    expect(lambertPhase(Math.PI)).toBeCloseTo(0, 12);
    expect(lambertPhase(Math.PI / 2)).toBeCloseTo(1 / Math.PI, 12);
  });
});

describe('reflected light', () => {
  it('gives the full Moon about 2e-6 of the Sun, 0.3 lux of 1.3e5', () => {
    const moon = body(diameter(1737, 384_400), { colour: [0.12, 0.12, 0.12] });
    expect(reflectedRatio(moon, SUN)).toBeGreaterThan(1.5e-6);
    expect(reflectedRatio(moon, SUN)).toBeLessThan(3e-6);
  });

  it('gives a new body nothing and a quarter body a third of the full one', () => {
    const full = reflectedRatio(body(5), SUN);
    expect(reflectedRatio(body(5, { phaseCos: -1 }), SUN)).toBeCloseTo(0, 12);
    expect(reflectedRatio(body(5, { phaseCos: 0 }), SUN) / full).toBeCloseTo(1 / Math.PI, 6);
  });

  it('adds the rings, seen open, and none when seen edge on', () => {
    const rings = { inner: 1.2, outer: 2.3, gaps: [], opacity: 0.8, colour: [1, 1, 1] as Vec3, bright: true };
    const bare = reflectedRatio(body(10), SUN);
    expect(reflectedRatio(body(10, { rings, axes: [[1, 0, 0], [0, 0, -1], [0, 1, 0]] }), SUN)).toBeGreaterThan(2 * bare);
    expect(reflectedRatio(body(10, { rings, axes: [[1, 0, 0], [0, 1, 0], [0, 0, 1]] }), SUN)).toBeCloseTo(bare, 12);
  });
});

describe('night brightness', () => {
  it('shows the display floor when nothing lights it', () => {
    expect(nightAt([0, 0, 1], [])).toBe(NIGHT_FLOOR);
    expect(nightBrightness(0)).toBe(NIGHT_FLOOR);
  });

  it('grows with the log of the light and stops at the ceiling', () => {
    const a = nightBrightness(1e-6);
    const b = nightBrightness(1e-5);
    const c = nightBrightness(1e-4);
    expect(b - a).toBeCloseTo(c - b, 12);
    expect(a).toBeGreaterThan(NIGHT_FLOOR);
    expect(nightBrightness(10)).toBe(NIGHT_CEILING);
  });

  it('lights a moon beside a full giant more than a planet with a small moon', () => {
    // A giant of 70 000 km at 400 000 km, lit full; versus a moon of 1 000 km at 400 000 km.
    const giant = nightSources([body(diameter(70_000, 400_000), { colour: [0.5, 0.4, 0.3] })], SUN);
    const smallMoon = nightSources([body(diameter(1_000, 400_000))], SUN);
    const normal: Vec3 = [0, 0, -1];
    expect(nightAt(normal, giant)).toBeGreaterThan(nightAt(normal, smallMoon) + 0.05);
    expect(nightAt(normal, smallMoon)).toBeLessThan(0.12);
  });

  it('lights the side facing the source, not the other', () => {
    const sources = nightSources([body(diameter(70_000, 400_000))], SUN);
    expect(nightAt([0, 0, -1], sources)).toBeGreaterThan(nightAt([0, 0, 1], sources));
    expect(nightAt([0, 0, 1], sources)).toBeCloseTo(NIGHT_FLOOR, 3);
  });

  it('fades smoothly to the horizon, with no step', () => {
    const sources = nightSources([body(diameter(70_000, 400_000))], SUN);
    let worst = 0;
    let last = nightAt([0, 0, -1], sources);
    for (let deg = 1; deg <= 120; deg++) {
      const a = (deg * Math.PI) / 180;
      const now = nightAt([Math.sin(a), 0, -Math.cos(a)], sources);
      worst = Math.max(worst, Math.abs(now - last));
      last = now;
    }
    // At most a tenth of the night's range per degree.
    expect(worst).toBeLessThan(0.1 * (NIGHT_CEILING - NIGHT_FLOOR));
  });

  it('keeps the brightest four sources', () => {
    const many = Array.from({ length: 7 }, (_, i) => body(1 + i));
    const sources = nightSources(many, SUN);
    expect(sources).toHaveLength(4);
    expect(sources[0]!.ratio).toBeGreaterThan(sources[3]!.ratio);
  });
});
