import { describe, expect, it } from 'vitest';
import type { FormSpec } from './api/types/FormSpec';
import { setLocale } from './i18n/index.svelte';
import { fieldValueText, referencesLabel, referencesLine } from './modelFormat';

const ref = (body: string, field: string, value: number) => ({ body, field, value });
const spec = {
  references: [
    ref('titan', 'mass_earth', 0.0225), ref('mars', 'mass_earth', 0.107), ref('venus', 'mass_earth', 0.815), ref('earth', 'mass_earth', 1),
    ref('neptune', 'mass_earth', 17.15), ref('jupiter', 'mass_earth', 317.8),
    ref('earth', 'co2', 1),
    ref('earth', 'core_mass_fraction', 0.325), ref('mercury', 'core_mass_fraction', 0.7),
    ref('earth', 'age_gyr', 4.6),
  ],
} as unknown as Pick<FormSpec, 'references'>;

describe('modelFormat', () => {
  it('writes a mass with its tonnes, and the other fields with their unit', () => {
    expect(fieldValueText('mass_earth', 1)).toBe('1 M⊕ (5.97×10²¹ t)');
    expect(fieldValueText('core_mass_fraction', 0.325)).toBe('32.5 %');
    expect(fieldValueText('water_oceans', 2)).toBe('2 Earth oceans');
  });

  it('quotes the references from the server’s values, under the reference-masses label for the mass', () => {
    expect(referencesLine('mass_earth', spec)).toBe('Reference masses: Titan 0.023 · Mars 0.107 · Venus 0.815 · Earth 1 · Neptune 17.15 · Jupiter 317.8');
    expect(referencesLine('core_mass_fraction', spec)).toBe('Reference values: Earth 32.5 % · Mercury 70 %');
    expect(referencesLine('age_gyr', spec)).toBe('Reference values: Earth 4.6 Gyr');
    expect(referencesLine('co2', spec)).toBe('Reference values: Earth 1');
  });

  it('says nothing for a field the server has no value for, or life', () => {
    expect(referencesLine('ch4', spec)).toBeNull();
    expect(referencesLine('life', spec)).toBeNull();
  });

  it('is localised', () => {
    setLocale('fr');
    expect(referencesLabel('mass_earth')).toBe('Masses de référence');
    expect(referencesLine('mass_earth', spec)).toContain('Terre 1');
    expect(referencesLine('mass_earth', spec)).toContain('Mars 0,107');
    expect(fieldValueText('mass_earth', 1)).toContain('5,97×10²¹ t');
  });
});
