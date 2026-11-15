import { describe, expect, it } from 'vitest';
import { fromPosition, fromShown, SLIDER_STEPS, toPosition, toShown } from './modelScale';

const mass = { min: 0.01, max: 4131 };
const water = { min: 0, max: 1e6 };

describe('modelScale', () => {
  it('spans a log slider from the minimum to the maximum', () => {
    expect(toPosition('mass_earth', 0.01, mass)).toBeCloseTo(0);
    expect(toPosition('mass_earth', 4131, mass)).toBeCloseTo(SLIDER_STEPS);
    expect(fromPosition('mass_earth', 0, mass)).toBeCloseTo(0.01);
    const middle = fromPosition('mass_earth', toPosition('mass_earth', 1, mass), mass);
    expect(middle).toBeCloseTo(1, 1);
  });
  it('gives water a "none" stop at 0', () => {
    expect(toPosition('water_oceans', 0, water)).toBe(0);
    expect(fromPosition('water_oceans', 0, water)).toBe(0);
    expect(fromPosition('water_oceans', 1, water)).toBeGreaterThan(0);
    expect(toPosition('water_oceans', 1, water)).toBeGreaterThan(1);
    expect(fromPosition('water_oceans', SLIDER_STEPS, water)).toBe(1e6);
  });
  it('keeps a linear slider linear and caps the age at the universe', () => {
    expect(fromPosition('eccentricity', SLIDER_STEPS / 2, { min: 0, max: 0.9 })).toBeCloseTo(0.45);
    expect(fromPosition('age_gyr', SLIDER_STEPS, { min: 0, max: 1e4 })).toBe(14);
    expect(toPosition('age_gyr', 500, { min: 0, max: 1e4 })).toBe(SLIDER_STEPS);
  });
  it('shows shares as percent', () => {
    expect(toShown('core_mass_fraction', 0.325)).toBeCloseTo(32.5);
    expect(fromShown('envelope_fraction', 50)).toBeCloseTo(0.5);
    expect(toShown('mass_earth', 2)).toBe(2);
  });
});
