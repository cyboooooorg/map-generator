import { describe, expect, it } from 'vitest';
import type { WorldMetadata } from './api/types/WorldMetadata';
import { liftSky, skyFromWorld, sunDeclinationDeg } from './sky';

const EARTH = { rayleigh: [0.17, 0.41, 1.0], haze: 0.1, haze_colour: [1, 1, 1], greenhouse: 0, clouds: 'water', cloud_cover: 0 };
const world = (pressure: number, atmosphere: object = EARTH) =>
  ({ pressure_bar: pressure, preset: { atmosphere: { ...EARTH, ...atmosphere } } }) as unknown as WorldMetadata;

describe('skyFromWorld', () => {
  it('gives airless worlds no sky', () => {
    expect(skyFromWorld(world(0, { haze: 0 }))).toBeNull();
  });

  it("gives an Earth-like atmosphere a blue sky", () => {
    const sky = skyFromWorld(world(1))!;
    expect(sky.betaR[2]).toBeGreaterThan(sky.betaR[0] * 5);
    expect(sky.top).toBeGreaterThan(1);
  });

  it('thickens and darkens the sky with pressure', () => {
    const thin = skyFromWorld(world(0.006))!;
    const dense = skyFromWorld(world(92))!;
    expect(dense.top).toBeGreaterThan(thin.top);
    expect(dense.betaR[2] * dense.heightR).toBeGreaterThan(thin.betaR[2] * thin.heightR * 10);
  });
});

describe('liftSky', () => {
  it('raises the top over the relief and keeps the zenith optical depths', () => {
    const sky = skyFromWorld(world(1))!;
    const lifted = liftSky(sky, 0.06);
    expect(lifted.top).toBeCloseTo(sky.top + 0.06);
    expect(lifted.betaR[2] * lifted.heightR).toBeCloseTo(sky.betaR[2] * sky.heightR);
    expect(lifted.betaM * lifted.heightM).toBeCloseTo(sky.betaM * sky.heightM);
    expect(liftSky(sky, 0)).toEqual(sky);
  });
});

describe('sunDeclinationDeg', () => {
  it('reaches the tilt at the solstices and follows sin δ = sin ε sin λ between', () => {
    expect(sunDeclinationDeg(23.44, 0)).toBeCloseTo(0, 9);
    expect(sunDeclinationDeg(23.44, 0.25)).toBeCloseTo(23.44, 9);
    expect(sunDeclinationDeg(23.44, 0.75)).toBeCloseTo(-23.44, 9);
    // Halfway to the solstice, a 90° tilt puts the sun over 45°, not over
    // 63.6° (90° × sin 45°) as a sine of the tilt would.
    expect(sunDeclinationDeg(90, 0.125)).toBeCloseTo(45, 9);
    expect(sunDeclinationDeg(60, 0.125)).toBeCloseTo(37.76, 2);
  });
});
