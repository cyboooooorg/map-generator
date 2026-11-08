import { describe, expect, it } from 'vitest';
import { celsius } from './climate';
import { temperatureRange } from './temperatureRange';

describe('temperatureRange', () => {
  it('keeps -30 to 40 for Earth-like worlds', () => {
    expect(temperatureRange({ min: -45, max: 38 }, 4)).toEqual([-30, 40]);
    expect(temperatureRange({ min: -10, max: 30 }, 4)).toEqual([-30, 40]);
    expect(temperatureRange({ min: -30, max: 40 }, 4)).toEqual([-30, 40]);
  });
  it('fits a hot world', () => {
    expect(temperatureRange({ min: -170, max: 427 }, 4)).toEqual([-170, 430]);
  });
  it('fits a frozen world', () => {
    expect(temperatureRange({ min: -123, max: -61 }, 4)).toEqual([-130, -60]);
  });
  it('never narrows below 20 degrees', () => {
    expect(temperatureRange({ min: 301, max: 303 }, 4)).toEqual([295, 315]);
  });
  it('converts the old [0, 1] scale', () => {
    expect(temperatureRange({ min: 0, max: 1 }, 3)).toEqual([-30, 40]);
    expect(celsius(0)).toBe(-18);
  });
  it('falls back without a layer', () => {
    expect(temperatureRange(null, 4)).toEqual([-30, 40]);
  });
});
