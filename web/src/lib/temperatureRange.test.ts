import { describe, expect, it } from 'vitest';
import { temperatureRange, temperatureSpan } from './temperatureRange';

describe('temperatureRange', () => {
  it('keeps -30 to 40 for Earth-like worlds', () => {
    expect(temperatureRange({ min: -45, max: 38 })).toEqual([-30, 40]);
    expect(temperatureRange({ min: -10, max: 30 })).toEqual([-30, 40]);
    expect(temperatureRange({ min: -30, max: 40 })).toEqual([-30, 40]);
  });
  it('fits a hot world', () => {
    expect(temperatureRange({ min: -170, max: 427 })).toEqual([-170, 430]);
  });
  it('fits a frozen world', () => {
    expect(temperatureRange({ min: -123, max: -61 })).toEqual([-130, -60]);
  });
  it('never narrows below 20 degrees', () => {
    expect(temperatureRange({ min: 301, max: 303 })).toEqual([295, 315]);
  });
  it('falls back without a span', () => {
    expect(temperatureRange(null)).toEqual([-30, 40]);
  });
});

describe('temperatureSpan', () => {
  // The file covers -200 to 600 °C on its 16 bits, whatever the data holds.
  const info = { min: -200, max: 600 };
  const at = (celsius: number) => Math.round(((celsius - info.min) / (info.max - info.min)) * 65535);
  it('reads the data range, not the encoding range', () => {
    const pixels = Uint16Array.from({ length: 1000 }, (_, i) => at(100 + (i / 999) * 300));
    const span = temperatureSpan(pixels, info);
    expect(span.min).toBeGreaterThan(99);
    expect(span.min).toBeLessThan(102);
    expect(span.max).toBeGreaterThan(398);
    expect(span.max).toBeLessThan(401);
    expect(temperatureRange(span)).toEqual([100, 400]);
  });
  it('leaves a few extreme pixels out', () => {
    const pixels = new Uint16Array(10000).fill(at(20));
    pixels[0] = at(500);
    const span = temperatureSpan(pixels, info);
    expect(span.max).toBeCloseTo(20, 0);
  });
  it('converts the old scale', () => {
    const span = temperatureSpan(new Uint16Array(10).fill(65535), { min: 0, max: 1 }, (v) => v * 100);
    expect(span.max).toBeCloseTo(100);
  });
});
