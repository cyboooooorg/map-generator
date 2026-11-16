import { describe, expect, it } from 'vitest';
import type { Cause } from './api/types/Cause';
import type { Explanation } from './api/types/Explanation';
import type { InputField } from './api/types/InputField';
import type { Quantity } from './api/types/Quantity';
import { drivenBy, formatCause, treeFor } from './causes';
import { formatMassEarth, formatSurfaceTemperature, formatUpTo, withUnit } from './format';
import { setLocale, t } from './i18n/index.svelte';

const node = (quantity: Quantity, value: number, over: Partial<Cause> = {}): Cause => ({ quantity, value, label: null, input: null, causes: [], ...over });

/** Every code the server sends: a new Rust code fails the type check until it is listed here. */
const QUANTITIES: Record<Quantity, true> = {
  surface_temperature: true, equilibrium_temperature: true, light_received: true, greenhouse: true, internal_warming: true,
  heat_flux: true, radiogenic_heat: true, tidal_heat: true, secular_heat: true, pressure: true, co2: true, n2: true, h2o: true, ch4: true, o2: true,
  nh3: true, so2: true, outgassed: true, escape: true, destroyed: true, frozen_out: true, from_ammonia: true, weathered: true,
  liquid: true, regime: true, magnetic_field: true, core_state: true, core_mass_fraction: true, class: true, mass: true,
  radius: true, envelope_fraction: true, envelope_lost: true, water_formed: true, water_kept: true, age: true, note: true,
};
const INPUTS: Record<InputField, true> = {
  mass_earth: true, core_mass_fraction: true, water_oceans: true, envelope_fraction: true, co2: true, n2: true, ch4: true,
  radiogenic: true, life: true, insolation: true, eccentricity: true, age_gyr: true,
};

describe('treeFor', () => {
  const explanation: Explanation = { trees: [node('class', 0, { label: 'rocky' }), node('surface_temperature', 288)] };
  it('finds a root', () => expect(treeFor(explanation, 'surface_temperature')?.value).toBe(288));
  it('gives null for a missing root or no explanation', () => {
    expect(treeFor(explanation, 'pressure')).toBeNull();
    expect(treeFor(null, 'pressure')).toBeNull();
  });
});

describe('formatCause', () => {
  it('writes a value with its unit', () => {
    setLocale('en');
    expect(formatCause(node('greenhouse', 33.4))).toMatch(/^33\.4\s?K$/);
    expect(formatCause(node('pressure', 1.013))).toMatch(/^1\.01\s?bar$/);
    expect(formatCause(node('radiogenic_heat', 2e13))).toMatch(/W$/);
    expect(formatCause(node('core_mass_fraction', 0.33))).toMatch(/^33\s?%$/);
  });
  it('writes the surface temperature in °C, as the card does', () => {
    setLocale('en');
    expect(formatCause(node('surface_temperature', 93))).toBe(formatSurfaceTemperature({ surface: { climate: { temperature_k: 93 } } } as never));
    expect(formatCause(node('surface_temperature', 288.15))).toMatch(/^15\s?°C$/);
    expect(formatCause(node('equilibrium_temperature', 255))).toMatch(/°C$/);
    // A warming is a difference: kelvin.
    expect(formatCause(node('greenhouse', 33))).toMatch(/K$/);
  });
  it('writes the water and the mass as the cards do', () => {
    setLocale('en');
    expect(formatCause(node('water_kept', 38.978))).toBe(withUnit(formatUpTo(38.978, 3), 'oceans'));
    expect(formatCause(node('water_kept', 38.978))).toContain('38.978');
    expect(formatCause(node('mass', 0.022))).toBe(formatMassEarth(0.022));
    expect(formatCause(node('mass', 0.5))).toMatch(/^0\.5\s?M⊕$/);
  });
  it('writes a power of ten with superscript digits', () => {
    setLocale('en');
    expect(formatCause(node('radiogenic_heat', 1.76e13))).toMatch(/^1\.76×10¹³\s?W$/);
    expect(formatCause(node('radiogenic_heat', 2e-5))).toMatch(/^2×10⁻⁵\s?W$/);
  });
  it('writes a key in the active language', () => {
    setLocale('en');
    expect(formatCause(node('regime', 0, { label: 'plates' }))).toBe('plate tectonics');
    expect(formatCause(node('liquid', 0, { label: 'water' }))).toBe('water');
    expect(formatCause(node('note', 0, { label: 'thermostat' }))).toBe(t.causes.notes.thermostat);
    expect(formatCause(node('class', 0, { label: 'zzz' }))).toBe('zzz');
  });
});

describe('the cause catalog', () => {
  for (const locale of ['en', 'fr'] as const) {
    it(`words every quantity and input in ${locale}`, () => {
      setLocale(locale);
      for (const q of Object.keys(QUANTITIES) as Quantity[]) {
        expect(t.causes.units[q], q).toBeTypeOf('string');
        expect(t.causes.quantities[q], q).toBeTypeOf('string');
        if (q !== 'note') expect(t.causes.quantities[q], q).not.toBe('');
      }
      for (const f of Object.keys(INPUTS) as InputField[]) expect(t.causes.inputs[f], f).toBeTruthy();
      setLocale('en');
    });
  }
});

describe('drivenBy', () => {
  const explanation: Explanation = {
    trees: [
      node('surface_temperature', 288, { causes: [node('greenhouse', 33, { causes: [node('co2', 1, { input: 'co2' })] })] }),
      node('pressure', 1, { causes: [node('co2', 1, { input: 'co2' })] }),
      node('magnetic_field', 1, { causes: [node('mass', 1, { input: 'mass_earth' })] }),
    ],
  };
  it('lists the roots holding a leaf of the field, in tree order', () => {
    expect(drivenBy(explanation, 'co2')).toEqual(['surface_temperature', 'pressure']);
    expect(drivenBy(explanation, 'life')).toEqual([]);
    expect(drivenBy(null, 'co2')).toEqual([]);
  });
});
