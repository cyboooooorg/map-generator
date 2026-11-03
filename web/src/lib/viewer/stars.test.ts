import { describe, expect, it } from 'vitest';
import { STAR_COUNTS, starField } from './stars';

describe('starField', () => {
  const field = starField();
  const count = STAR_COUNTS.sky + STAR_COUNTS.band;

  it('is the same for the same seed', () => {
    expect(starField().directions).toEqual(field.directions);
    expect(starField(8).directions).not.toEqual(field.directions);
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
});
