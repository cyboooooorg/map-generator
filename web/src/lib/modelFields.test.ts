import { describe, expect, it } from 'vitest';
import type { FormSpec } from './api/types/FormSpec';
import type { InputField } from './api/types/InputField';
import { fieldOf, marks, REFERENCE_BODIES, referencesOf, scaleOf } from './modelFields';

const FIELDS: Record<InputField, true> = {
  mass_earth: true, core_mass_fraction: true, water_oceans: true, envelope_fraction: true, co2: true, n2: true, ch4: true,
  radiogenic: true, life: true, insolation: true, eccentricity: true, age_gyr: true,
};
const ref = (body: string, field: string, value: number) => ({ body, field, value });
const spec = {
  references: [
    ref('earth', 'mass_earth', 1), ref('mars', 'mass_earth', 0.107), ref('venus', 'mass_earth', 0.815), ref('titan', 'mass_earth', 0.0225),
    ref('neptune', 'mass_earth', 17.15), ref('jupiter', 'mass_earth', 317.8),
    ref('earth', 'water_oceans', 1), ref('mars', 'water_oceans', 0), ref('venus', 'water_oceans', 0),
    ref('earth', 'ch4', 1),
  ],
} as unknown as Pick<FormSpec, 'references'>;

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
  it('reads the references of a field by body', () => {
    expect(referencesOf('mass_earth', spec).get('jupiter')).toBe(317.8);
    expect(referencesOf('life', spec).size).toBe(0);
  });
  it('marks the six mass bodies at the values the server sends, never at an archetype’s mean', () => {
    const found = marks('mass_earth', spec);
    expect(found.map((m) => [m.bodies[0], m.value])).toEqual([
      ['titan', 0.0225], ['mars', 0.107], ['venus', 0.815], ['earth', 1], ['neptune', 17.15], ['jupiter', 317.8],
    ]);
  });
  it('quotes the same bodies on the chips and in the help', () => {
    expect(REFERENCE_BODIES.mass_earth).toEqual(['titan', 'mars', 'venus', 'earth', 'neptune', 'jupiter']);
  });
  it('skips a body the server has no value for', () => {
    expect(marks('ch4', spec).map((m) => m.bodies)).toEqual([['earth']]);
  });
  it('groups the bodies that sit at one value', () => {
    expect(marks('water_oceans', spec).map((m) => m.bodies)).toEqual([['mars', 'venus'], ['earth']]);
  });
  it('drops a mark outside the limits', () => {
    const found = marks('mass_earth', spec, { min: 0.05, max: 10 });
    expect(found.map((m) => m.bodies[0])).toEqual(['mars', 'venus', 'earth']);
  });
});
