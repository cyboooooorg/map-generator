import { describe, expect, it } from 'vitest';
import { MAX_BAR, PRESSURE_STEPS, pressureToSlider, sliderToPressure } from './pressure';

describe('pressure slider', () => {
  it('puts airless at the left end and the server limit at the right', () => {
    expect(sliderToPressure(0)).toBe(0);
    expect(pressureToSlider(0)).toBe(0);
    expect(sliderToPressure(1)).toBe(0.001);
    expect(sliderToPressure(PRESSURE_STEPS)).toBe(MAX_BAR);
  });

  it('reaches Mars, Earth and Venus by dragging', () => {
    for (const [bar, near] of [
      [0.006, 0.006],
      [1, 1],
      [92, 92],
    ] as const) {
      const position = pressureToSlider(bar);
      const reached = [position - 1, position, position + 1].map(sliderToPressure);
      expect(reached.some((r) => Math.abs(r - near) / near < 0.02)).toBe(true);
    }
    expect(sliderToPressure(pressureToSlider(0.006))).toBe(0.006);
  });

  it('round-trips a typed value to the nearest step', () => {
    for (const bar of [0.0002, 0.05, 3.3, 500, 5000]) {
      const position = pressureToSlider(bar);
      expect(position).toBeGreaterThanOrEqual(1);
      expect(position).toBeLessThanOrEqual(PRESSURE_STEPS);
    }
    expect(pressureToSlider(0.0002)).toBe(1);
    expect(pressureToSlider(5000)).toBe(PRESSURE_STEPS);
    // Monotonic.
    let previous = -1;
    for (let p = 0; p <= PRESSURE_STEPS; p += 1) {
      const bar = sliderToPressure(p);
      expect(bar).toBeGreaterThanOrEqual(previous);
      previous = bar;
    }
  });
});
