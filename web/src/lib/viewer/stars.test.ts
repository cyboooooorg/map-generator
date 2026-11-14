import { describe, expect, it } from 'vitest';
import type { Vec3 } from '../skyBodies';
import { PHOTO_STAR_DENSITY, SEEDED_STAR_MAX, STAR_COUNTS, seededStarCounts, starField } from './stars';
import { bandFrame, FIXED_SKY, skyLook, type SkyLook } from './skyLook';

/** FNV-1a over the float bits of a buffer. */
function hash(a: Float32Array): number {
  let x = 2166136261;
  for (const v of new Uint32Array(a.buffer, a.byteOffset, a.length)) x = Math.imul(x ^ v, 16777619) >>> 0;
  return x;
}

const dot = (a: ArrayLike<number>, b: ArrayLike<number>, at = 0) => a[at]! * b[0]! + a[at + 1]! * b[1]! + a[at + 2]! * b[2]!;

describe('starField', () => {
  const field = starField(FIXED_SKY);
  const count = STAR_COUNTS.sky + STAR_COUNTS.band;

  it('reproduces the fixed sky every world had before the seeded sky', () => {
    // Hashes of the buffers of the previous `starField(7)`, taken before the change.
    expect(field.directions.length / 3).toBe(8500);
    expect(hash(field.directions)).toBe(431991875);
    expect(hash(field.colours)).toBe(432136082);
    expect(hash(field.sizes)).toBe(2685119737);
    expect(starField().directions).toEqual(field.directions);
  });

  it('is the same for the same look and differs for another seed', () => {
    expect(starField({ ...FIXED_SKY }).directions).toEqual(field.directions);
    expect(starField({ ...FIXED_SKY, seed: 8 }).directions).not.toEqual(field.directions);
  });

  it('places unit directions with a size and a colour each', () => {
    expect(field.directions.length).toBe(count * 3);
    expect(field.colours.length).toBe(count * 3);
    expect(field.sizes.length).toBe(count);
    for (let i = 0; i < count; i += 1) {
      const [x, y, z] = field.directions.subarray(i * 3, i * 3 + 3);
      expect(Math.hypot(x!, y!, z!)).toBeCloseTo(1, 5);
      expect(field.sizes[i]).toBeGreaterThanOrEqual(1);
      expect(field.colours[i * 3]).toBeLessThanOrEqual(1);
    }
  });

  it('has many faint stars and a few bright ones', () => {
    const sizes = [...field.sizes];
    const bright = sizes.filter((s) => s > 2.5).length;
    expect(bright).toBeGreaterThan(0);
    expect(bright).toBeLessThan(count * 0.1);
  });

  it('spreads band stars about the plane perpendicular to the look north', () => {
    const north: Vec3 = [0.6, 0, 0.8];
    const look: SkyLook = { ...FIXED_SKY, north, centre: [0, 1, 0], seed: 3 };
    const stars = starField(look);
    let band = 0;
    let sky = 0;
    for (let i = 0; i < STAR_COUNTS.sky; i += 1) sky += Math.abs(dot(stars.directions, north, i * 3));
    for (let i = STAR_COUNTS.sky; i < STAR_COUNTS.sky + STAR_COUNTS.band; i += 1) {
      band += Math.abs(dot(stars.directions, north, i * 3));
    }
    // Random stars average 0.5; the band hugs the plane.
    expect(sky / STAR_COUNTS.sky).toBeGreaterThan(0.4);
    expect(band / STAR_COUNTS.band).toBeLessThan(0.15);
  });

  it('has more band stars when brighter', () => {
    const dim = starField({ ...FIXED_SKY, brightness: 0.5 });
    const bright = starField({ ...FIXED_SKY, brightness: 2 });
    expect(bright.sizes.length).toBeGreaterThan(dim.sizes.length);
  });

  it('gathers stars toward the centre with a stronger bulge', () => {
    const near = (look: SkyLook) => {
      const stars = starField(look);
      let n = 0;
      for (let i = 0; i < stars.sizes.length; i += 1) {
        if (dot(stars.directions, look.centre, i * 3) > Math.cos(0.5)) n += 1;
      }
      return n;
    };
    expect(near({ ...FIXED_SKY, bulge: 1 })).toBeGreaterThan(near({ ...FIXED_SKY, bulge: 0 }) + 500);
  });

  it('dims stars behind dust, to nothing when opaque', () => {
    const dark = starField(FIXED_SKY, () => 1);
    expect(Math.max(...dark.colours)).toBe(0);
    const half = starField(FIXED_SKY, () => 0.5);
    expect(half.colours[0]).toBeCloseTo(field.colours[0]! * 0.5, 6);
  });
});

describe('the sky of an outer system', () => {
  const outer = skyLook({ source: 'Derived', position_kpc: [16, 0, 0], axes: [[1, 0, 0], [0, 1, 0], [0, 0, 1]], sky_seed: 5 } as never);

  it('keeps a band denser than the rest of the sky', () => {
    const f = starField(outer);
    const north = outer.north;
    const half = outer.bandWidth * 1.5;
    let inBand = 0;
    let off = 0;
    for (let i = 0; i < f.sizes.length; i += 1) {
      const lat = Math.asin(dot(f.directions, north, i * 3));
      if (Math.abs(lat) < half) inBand += 1;
      else off += 1;
    }
    // Solid angle of |lat| < half is 4 pi sin(half); the rest of the sphere what remains.
    const bandArea = 4 * Math.PI * Math.sin(half);
    expect(inBand / bandArea).toBeGreaterThan(off / (4 * Math.PI - bandArea));
    expect(f.sizes.length).toBeGreaterThan(STAR_COUNTS.band * 0.4);
  });
});

describe('starField density', () => {
  it('draws 1.6 times the sky and band stars for a long exposure, the bulge unchanged', () => {
    expect(PHOTO_STAR_DENSITY).toBe(1.6);
    const look: SkyLook = { ...FIXED_SKY, seed: 31, bulge: 0 };
    const eye = starField(look).sizes.length;
    const photo = starField(look, undefined, PHOTO_STAR_DENSITY).sizes.length;
    expect(photo / eye).toBeGreaterThanOrEqual(1.55);
    expect(photo / eye).toBeLessThanOrEqual(1.65);
    const bulged: SkyLook = { ...look, bulge: 1 };
    expect(starField(bulged, undefined, PHOTO_STAR_DENSITY).sizes.length - photo).toBe(starField(bulged).sizes.length - eye);
  });
});

describe("a system's star field", () => {
  const sunLook = skyLook({ source: 'Derived', position_kpc: [8.2, 0, 0.02], axes: [[1, 0, 0], [0, 1, 0], [0, 0, 1]], sky_seed: 5 } as never);

  /** Stars in a cone of `radius` radians about a direction. */
  const inCone = (look: SkyLook, centre: Vec3, radius: number, density = 1) => {
    const f = starField(look, undefined, density);
    const cos = Math.cos(radius);
    let n = 0;
    for (let i = 0; i < f.sizes.length; i += 1) if (dot(f.directions, centre, i * 3) > cos) n += 1;
    return n;
  };

  it('is at least four times denser at the core than 90 degrees along the band, at the Sun\'s place', () => {
    const { along, across } = bandFrame(sunLook);
    const radius = (10 * Math.PI) / 180;
    expect(inCone(sunLook, sunLook.centre, radius)).toBeGreaterThanOrEqual(4 * inCone(sunLook, across, radius));
    expect(inCone(sunLook, sunLook.centre, radius)).toBeGreaterThanOrEqual(4 * inCone(sunLook, [-along[0], -along[1], -along[2]], radius));
  });

  it('is denser along the band than off it', () => {
    const { across } = bandFrame(sunLook);
    const radius = (8 * Math.PI) / 180;
    const off: Vec3 = [...sunLook.north];
    expect(inCone(sunLook, across, radius)).toBeGreaterThan(5 * inCone(sunLook, off, radius));
  });

  it('keeps the star count within the cap in every exposure, near the centre too', () => {
    for (const x of [0.6, 2, 4.5, 8.2, 13]) {
      const look = skyLook({ source: 'Derived', position_kpc: [x, 0, 0.02], axes: [[1, 0, 0], [0, 1, 0], [0, 0, 1]], sky_seed: 5 } as never);
      for (const density of [1, PHOTO_STAR_DENSITY]) {
        const counts = seededStarCounts(look, density);
        expect(counts.sky + counts.band + counts.core + counts.bulge).toBeLessThanOrEqual(SEEDED_STAR_MAX);
        expect(starField(look, undefined, density).sizes.length).toBe(counts.sky + counts.band + counts.core + counts.bulge);
      }
    }
  });

  it('has more stars in a long exposure, and more of them toward the core as the Sun-to-centre distance falls', () => {
    expect(starField(sunLook, undefined, PHOTO_STAR_DENSITY).sizes.length).toBeGreaterThan(starField(sunLook).sizes.length);
    const near = skyLook({ source: 'Derived', position_kpc: [4.5, 0, 0.02], axes: [[1, 0, 0], [0, 1, 0], [0, 0, 1]], sky_seed: 5 } as never);
    expect(seededStarCounts(near).core).toBeGreaterThan(seededStarCounts(sunLook).core);
  });
});
