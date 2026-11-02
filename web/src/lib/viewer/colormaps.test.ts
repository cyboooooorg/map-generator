import { describe, expect, it } from 'vitest';
import { cssGradient, glslRamp, RAMPS, type DataLayer } from './colormaps';

const layers = Object.keys(RAMPS) as DataLayer[];

describe('colormaps', () => {
  it('have stops in increasing order from 0 to 1', () => {
    for (const layer of layers) {
      const positions = RAMPS[layer].stops.map(([p]) => p);
      expect(positions[0]).toBe(0);
      expect(positions.at(-1)).toBe(1);
      expect([...positions].sort((a, b) => a - b)).toEqual(positions);
    }
  });

  it('generate one GLSL segment and one CSS stop per colour stop', () => {
    for (const layer of layers) {
      const stops = RAMPS[layer].stops.length;
      expect(glslRamp(layer)).toContain(`vec3 ramp_${layer}(float t)`);
      expect(glslRamp(layer).match(/if \(t <=/g)).toHaveLength(stops - 1);
      expect(cssGradient(layer).match(/rgb\(/g)).toHaveLength(stops);
    }
  });

  it('put the coastline in the middle of the elevation ramp', () => {
    const [, land] = RAMPS.elevation.stops.find(([p]) => p > 0.5)!;
    const [, sea] = RAMPS.elevation.stops.find(([p]) => p === 0.5)!;
    expect(land).not.toEqual(sea);
  });
});
