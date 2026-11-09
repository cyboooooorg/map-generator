import { describe, expect, it } from 'vitest';
import type { Vec3 } from '../skyBodies';
import { discBasis, hostStarDetail, hostStarSpriteSize } from './hostStarSurface';
import { hostStarFragment, hostStarVertex } from './shaders';

const IDENTITY: [Vec3, Vec3, Vec3] = [
  [1, 0, 0],
  [0, 1, 0],
  [0, 0, 1],
];

/** `m` (a column-major mat3) times `v`. */
const apply = (m: number[], v: Vec3): Vec3 => [
  m[0]! * v[0] + m[3]! * v[1] + m[6]! * v[2],
  m[1]! * v[0] + m[4]! * v[1] + m[7]! * v[2],
  m[2]! * v[0] + m[5]! * v[1] + m[8]! * v[2],
];

const expectClose = (a: Vec3, b: Vec3) => a.forEach((x, i) => expect(x).toBeCloseTo(b[i]!, 9));

describe('host star surface', () => {
  it("faces the star's visible hemisphere", () => {
    // Star axes = the world's, the camera on +z of the star looking at it, up +y.
    const m = discBasis([0, 0, 1], [0, 1, 0], IDENTITY);
    expect(m).toHaveLength(9);
    expectClose(apply(m, [0, 0, 1]), [0, 0, 1]);
    expectClose(apply(m, [1, 0, 0]), [1, 0, 0]);
    expectClose(apply(m, [0, 1, 0]), [0, 1, 0]);
  });

  it('turns with the star frame', () => {
    // The star turned 90° about y (−90°, so that its +x faces the camera
    // on the world's +z): its axes x → +z, y → y, z → −x in the world.
    const turned: [Vec3, Vec3, Vec3] = [
      [0, 0, 1],
      [0, 1, 0],
      [-1, 0, 0],
    ];
    const m = discBasis([0, 0, 1], [0, 1, 0], turned);
    // The line of sight is the star's +x (longitude 0); its +z, the
    // world's −x, is on the left, so screen right is its −z.
    expectClose(apply(m, [0, 0, 1]), [1, 0, 0]);
    expectClose(apply(m, [1, 0, 0]), [0, 0, -1]);
    expectClose(apply(m, [0, 1, 0]), [0, 1, 0]);
  });

  it('keeps the screen up even when the up vector leans towards the line of sight', () => {
    const m = discBasis([0, 0, 1], [0, 1, 0.5], IDENTITY);
    expectClose(apply(m, [0, 1, 0]), [0, 1, 0]);
    expectClose(apply(m, [1, 0, 0]), [1, 0, 0]);
  });

  it('writes into a reusable array', () => {
    const out = new Array<number>(9).fill(NaN);
    const m = discBasis([0, 0, 1], [0, 1, 0], IDENTITY, out);
    expect(m).toBe(out);
    expect(m).toEqual(discBasis([0, 0, 1], [0, 1, 0], IDENTITY));
  });

  it('shows no detail on a small disc', () => {
    expect(hostStarDetail(14)).toBe(0);
    expect(hostStarDetail(30)).toBe(1);
    expect(hostStarDetail(0)).toBe(0);
    const mid = hostStarDetail(20);
    expect(mid).toBeGreaterThan(0);
    expect(mid).toBeLessThan(1);
  });

  it('sizes the sprite to hold the disc and its glow, whatever its size', () => {
    // Today's size while the disc is small.
    expect(hostStarSpriteSize(14, 1)).toBe(14 * 6 + 24);
    expect(hostStarSpriteSize(60, 2)).toBe(60 * 6 + 48);
    // The glow fades from a quarter of the size: never inside the disc.
    for (const disk of [80, 120, 200, 300, 500, 710, 1500, 4000]) {
      const size = hostStarSpriteSize(disk, 1);
      expect(0.25 * size).toBeGreaterThanOrEqual(disk / 2);
      expect(size).toBeGreaterThanOrEqual(Math.min(disk * 6 + 24, 512));
    }
  });

  it('draws the living surface on a quad, not a capped point sprite', () => {
    expect(hostStarFragment).toContain('#define STAR_NO_GRANULATION');
    // The star view's exposure on the disc, not the plain disc's 1.4 (which clipped it to white).
    expect(hostStarFragment).toMatch(/\? starSurfaceExposed\(n, mu\)/);
    expect(hostStarFragment).toMatch(/disk \* inside \+ uColor \* glow/);
    expect(hostStarFragment).toContain('uDiscBasis');
    // The old look without a surface.
    expect(hostStarFragment).toContain('0.35');
    expect(hostStarFragment).not.toContain('gl_PointCoord');
    expect(hostStarVertex).not.toContain('gl_PointSize');
  });
});
