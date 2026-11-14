import { describe, expect, it } from 'vitest';
import { linkedChanges, linkedPart, type LinkedDisplay } from './display';

const main: LinkedDisplay = {
  layer: 'elevation',
  relief: 0.5,
  sun: true,
  season: 0.25,
  sunLongitude: 90,
  atmosphere: false,
  sky: 'system',
  skyExposure: 'photo',
  starLight: false,
  clouds: true,
  graticule: true,
};

describe('linked display', () => {
  it('keeps only the linked settings', () => {
    const full = { ...main, mode: 'map', hexColumns: 64, cloudDrift: true };
    expect(linkedPart(full)).toEqual(main);
  });

  it('lists what a compare pane must change to mirror the main one', () => {
    const compare: LinkedDisplay = { ...main, layer: 'biomes', relief: 0, sun: false };
    expect(linkedChanges(compare, main)).toEqual({ layer: 'elevation', relief: 0.5, sun: true });
    expect(linkedChanges(main, main)).toEqual({});
  });
});
