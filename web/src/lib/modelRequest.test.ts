import { describe, expect, it } from 'vitest';
import type { ResolvedSystem } from './api/types/ResolvedSystem';
import earth from '../components/body/fixtures/earth-like.json';
import { drawnValues, initialModelValues, MODEL_KEYS, modelRequest, pinnedCount } from './modelRequest';

describe('modelRequest', () => {
  it('sends pinned fields only, life as a boolean or nothing', () => {
    const values = initialModelValues(null);
    expect(modelRequest(values)).toEqual({});
    values.co2 = { random: false, value: 2 };
    values.life = { random: false, value: false };
    values.age_gyr = { random: false, value: 3.5 };
    expect(modelRequest(values)).toEqual({ co2: 2, life: false, age_gyr: 3.5 });
    values.life = { random: true, value: true };
    expect(modelRequest(values)).toEqual({ co2: 2, age_gyr: 3.5 });
    expect(pinnedCount(values, ['co2', 'life'])).toBe(1);
  });

  it('drops a number the user emptied', () => {
    const values = initialModelValues(null);
    values.mass_earth = { random: false, value: Number.NaN };
    expect(modelRequest(values)).toEqual({});
  });

  it('starts pinned on what a request sets', () => {
    const values = initialModelValues({ ch4: 0.5, age_gyr: 2, life: true, eccentricity: 0.1 });
    expect(modelRequest(values)).toEqual({ ch4: 0.5, age_gyr: 2, life: true, eccentricity: 0.1 });
    expect(MODEL_KEYS).toHaveLength(11);
  });
});

describe('drawnValues', () => {
  const result = earth as unknown as ResolvedSystem;
  it('reads what the seed drew, in the request units', () => {
    const drawn = drawnValues(result);
    expect(drawn?.mass_earth).toBeCloseTo(1.0849, 3);
    expect(drawn?.water_oceans).toBeCloseTo(0.9, 1);
    expect(drawn?.core_mass_fraction).toBeCloseTo(0.325, 3);
    expect(drawn?.eccentricity).toBeCloseTo(0.017, 3);
    expect(drawn?.age_gyr).toBeCloseTo(4.59, 2);
    expect(drawn?.life).toBe(true);
  });
  it('has nothing without a preview', () => {
    expect(drawnValues(null)).toBeNull();
  });
});
