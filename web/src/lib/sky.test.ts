import { describe, expect, it } from 'vitest';
import type { WorldMetadata } from './api/types/WorldMetadata';
import { hazeDepth, hazeVeil, liftSky, skyFromWorld, sunDeclinationDeg } from './sky';

const EARTH = { rayleigh: [0.17, 0.41, 1.0], haze: 0.1, haze_colour: [1, 1, 1], greenhouse: 0, clouds: 'water', cloud_cover: 0 };
const world = (pressure: number, atmosphere: object = EARTH) =>
  ({ pressure_bar: pressure, look: { atmosphere: { ...EARTH, ...atmosphere } } }) as unknown as WorldMetadata;

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

// The skies of Earth, Venus and Mars as the generator's `sky_of` gives
// them, and Titan's methane haze (`METHANE_HAZE`).
const VENUS = { rayleigh: [0.4, 0.55, 0.25], haze: 0.3, haze_colour: [0.85, 0.95, 0.45] };
const MARS = { rayleigh: [0.17, 0.41, 1.0], haze: 0.1, haze_colour: [1, 1, 1] };
const TITAN = { haze: 0.9, haze_colour: [0.95, 0.55, 0.15] };

describe('thick hazes', () => {
  it('keeps the haze depth of every haze up to 0.5', () => {
    for (const haze of [0, 0.05, 0.1, 0.3, 0.45, 0.5]) {
      expect(hazeDepth(haze)).toBeCloseTo(2 * haze, 12);
    }
  });

  it('keeps the skies of Earth, Venus and Mars', () => {
    expect(skyFromWorld(world(1))).toEqual({
      top: 1 + 0.03 + 0.02 * Math.log10(2),
      betaR: [0.17, 0.41, 1.0].map((k) => (k * 0.25) / (0.25 * (0.03 + 0.02 * Math.log10(2)))),
      betaM: 0.2 / (0.12 * (0.03 + 0.02 * Math.log10(2))),
      mieAlbedo: [1, 1, 1],
      heightR: 0.25 * (0.03 + 0.02 * Math.log10(2)),
      heightM: 0.12 * (0.03 + 0.02 * Math.log10(2)),
      g: 0.76 - 0.5 * 0.25,
    });
    const venus = skyFromWorld(world(92, VENUS))!;
    expect(venus.betaM * venus.heightM).toBeCloseTo(0.6, 12);
    expect(venus.g).toBeCloseTo(0.76 - 0.5 * 0.75, 12);
    const mars = skyFromWorld(world(0.006, MARS))!;
    expect(mars.betaM * mars.heightM).toBeCloseTo(0.2, 12);
    for (const sky of [skyFromWorld(world(1))!, venus, mars]) {
      for (const altitude of [0.0003, 0.05, 0.3, 2.2, 10]) {
        expect(hazeVeil(sky, altitude)).toBe(1);
      }
    }
  });

  it("hides Titan's ground under an orange haze from afar", () => {
    const titan = skyFromWorld(world(1.5, TITAN))!;
    const depth = titan.betaM * titan.heightM;
    // Seen straight down, less than 1 % of the ground's light comes through.
    expect(Math.exp(-depth)).toBeLessThan(0.01);
    expect(titan.mieAlbedo[0]).toBeGreaterThan(titan.mieAlbedo[1]);
    expect(titan.mieAlbedo[1]).toBeGreaterThan(2 * titan.mieAlbedo[2]);
    // Scattered many times: brighter than one scattering, red most.
    expect(titan.mieAlbedo[0] / 0.95).toBeGreaterThan(titan.mieAlbedo[2] / 0.15);
    expect(titan.mieAlbedo[2]).toBeGreaterThan(0.15);
    // Many scatterings: about evenly lit from the sunny side.
    expect(titan.g).toBeLessThan(0.1);
    expect(hazeVeil(titan, 2.2)).toBe(1);
  });

  it("thins Titan's haze over the ground close in", () => {
    // Titan's methane haze (0.9) and a thinner hiding haze (0.7, depth 3.8).
    for (const haze of [0.9, 0.7, 1]) {
      const titan = skyFromWorld(world(1.5, { ...TITAN, haze }))!;
      const seen = (altitude: number) => Math.exp(-titan.betaM * titan.heightM * hazeVeil(titan, altitude));
      // From afar, the disc is the haze's, opaque and orange.
      for (const altitude of [1.5, 2.2, 10]) {
        expect(hazeVeil(titan, altitude)).toBe(1);
        expect(seen(altitude)).toBeLessThan(0.03);
      }
      // The ground starts to show by 0.6 radii up…
      expect(seen(0.6)).toBeGreaterThan(0.08);
      expect(seen(0.6)).toBeLessThan(0.3);
      // …and reads clearly by 0.2, then clearer down to 0.05.
      expect(seen(0.2)).toBeGreaterThanOrEqual(0.5);
      expect(seen(0.05)).toBeGreaterThan(0.75);
      expect(seen(0.01)).toBeCloseTo(seen(0.05), 12);
      // Clearer at every step down, without a jump.
      let last = 0;
      for (let altitude = 2; altitude >= 0.05; altitude *= 0.97) {
        const now = seen(altitude);
        expect(now).toBeGreaterThanOrEqual(last);
        expect(now - last).toBeLessThan(0.05);
        last = now;
      }
    }
    // The veil follows the depths, not the shell: lifted, the same.
    const titan = skyFromWorld(world(1.5, TITAN))!;
    expect(hazeVeil(liftSky(titan, 0.06), 0.1)).toBeCloseTo(hazeVeil(titan, 0.1), 12);
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
