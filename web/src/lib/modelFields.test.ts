import { describe, expect, it } from 'vitest';
import type { ArchetypeInfo } from './api/types/ArchetypeInfo';
import type { InputField } from './api/types/InputField';
import { fieldOf, marks, scaleOf } from './modelFields';

const FIELDS: Record<InputField, true> = {
  mass_earth: true, core_mass_fraction: true, water_oceans: true, envelope_fraction: true, co2: true, n2: true, ch4: true,
  radiogenic: true, life: true, insolation: true, eccentricity: true, age_gyr: true,
};
const archetype = (spec: string, inputs: ArchetypeInfo['inputs']): ArchetypeInfo =>
  ({ spec, name: spec, description: '', built_in: true, inputs }) as unknown as ArchetypeInfo;
const archetypes = [
  archetype('earth-like', { mass_earth: [0.9, 1.1], co2: 1 }),
  archetype('mars-like', { mass_earth: [0.096, 0.118], co2: 0.3 }),
  archetype('titan-like', { mass_earth: [0.02, 0.025], ch4: [1, 2] }),
];

describe('modelFields', () => {
  it('places every input field', () => {
    for (const field of Object.keys(FIELDS) as InputField[]) expect(fieldOf(field)).toBeTruthy();
    expect(fieldOf('insolation')).toBe('orbit');
    expect(fieldOf('co2')).toBe('co2');
  });
  it('scales mass logarithmically and eccentricity linearly', () => {
    expect(scaleOf('mass_earth')).toBe('log');
    expect(scaleOf('eccentricity')).toBe('linear');
  });
  it('marks Earth at 1 and Mars at about 0.107 on the mass', () => {
    const found = marks('mass_earth', archetypes);
    expect(found.find((m) => m.body === 'earth')!.value).toBeCloseTo(1, 1);
    expect(found.find((m) => m.body === 'mars')!.value).toBeCloseTo(0.107, 2);
  });
  it('skips a body that does not set the field', () => {
    expect(marks('ch4', archetypes).map((m) => m.body)).toEqual(['titan']);
  });
  it('drops a mark outside the limits', () => {
    const found = marks('mass_earth', archetypes, { min: 0.05, max: 10 });
    expect(found.map((m) => m.body)).toEqual(['earth', 'mars']);
  });
});
