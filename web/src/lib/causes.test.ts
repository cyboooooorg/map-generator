import { describe, expect, it } from 'vitest';
import type { Cause } from './api/types/Cause';
import type { Explanation } from './api/types/Explanation';
import type { InputField } from './api/types/InputField';
import type { Quantity } from './api/types/Quantity';
import { dominantFrost, drivenBy, formatCause, treeFor } from './causes';
import { formatMassEarth, formatSurfaceAge, formatSurfaceTemperature, formatUpTo, withUnit } from './format';
import { setLocale, t } from './i18n/index.svelte';

const node = (quantity: Quantity, value: number, over: Partial<Cause> = {}): Cause => ({ quantity, value, label: null, input: null, causes: [], ...over });

/** Every code the server sends: a new Rust code fails the type check until it is listed here. */
const QUANTITIES: Record<Quantity, true> = {
  surface_temperature: true, equilibrium_temperature: true, light_received: true, greenhouse: true, internal_warming: true,
  heat_flux: true, radiogenic_heat: true, tidal_heat: true, secular_heat: true, bond_albedo: true, pressure: true, co2: true, n2: true, h2o: true, ch4: true, o2: true,
  nh3: true, so2: true, outgassed: true, escape: true, destroyed: true, frozen_out: true, from_ammonia: true, weathered: true,
  liquid: true, regime: true, magnetic_field: true, core_state: true, core_mass_fraction: true, class: true, mass: true,
  radius: true, envelope_fraction: true, envelope_lost: true, water_formed: true, water_kept: true, age: true, shell_thickness: true, ocean_depth: true, polar_shell: true,
  moment_of_inertia: true, resonant_heat: true, host_dissipation: true, surface_age: true, terrain_unit: true, emissivity: true, cover_share: true, note: true,
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
  it('writes a surface age and a terrain unit with their renewal', () => {
    setLocale('en');
    expect(formatCause(node('surface_age', 4.5))).toBe(formatSurfaceAge(4.5));
    expect(formatCause(node('terrain_unit', 0.65, { label: 'tidal-episode' }))).toBe(t.causes.renewalValue({ renewal: t.causes.renewals['tidal-episode'], value: '65 %' }));
    expect(formatCause(node('surface_age', 2, { label: 'ice-lid' }))).toContain(t.causes.renewals['ice-lid']);
    expect(formatCause(node('emissivity', 0.89))).toBe('0.89');
  });
  it('writes the new notes in both languages', () => {
    for (const locale of ['en', 'fr'] as const) {
      setLocale(locale);
      for (const key of ['exosphere', 'liquid_cycle', 'volatile_ice', 'tidal_episode', 'volcanic_frost', 'fallout']) {
        expect(formatCause(node('note', 0, { label: key })), `${locale} ${key}`).toBe(t.causes.notes[key]);
        expect(t.causes.notes[key], `${locale} ${key}`).toBeTypeOf('string');
      }
    }
    setLocale('en');
  });
  it('writes the parts of an albedo with their share of the disc, in both languages', () => {
    for (const locale of ['en', 'fr'] as const) {
      setLocale(locale);
      for (const key of Object.keys(t.causes.covers)) {
        const text = formatCause(node('cover_share', 0.63, { label: key }));
        expect(text, `${locale} ${key}`).toContain(t.causes.covers[key]);
        expect(text, `${locale} ${key}`).toMatch(/63\s?%/);
        if (key.includes('_')) expect(text, `${locale} ${key}`).not.toContain(key);
      }
    }
    setLocale('en');
    expect(Object.keys(t.causes.covers)).toEqual(['ocean', 'land', 'ice', 'crust', 'frost_n2', 'frost_ch4', 'frost_co2', 'frost_so2', 'cloud', 'haze', 'deck']);
  });
  it('writes the haze as a strength, not a share of the disc', () => {
    setLocale('en');
    expect(formatCause(node('cover_share', 0.55, { label: 'haze' }))).toBe('methane haze, strength 55 %');
    setLocale('fr');
    expect(formatCause(node('cover_share', 0.55, { label: 'haze' }))).toContain('intensité');
    setLocale('en');
  });
  it('names a frost over most of the disc, and nothing else', () => {
    setLocale('en');
    const tree = (parts: [string, number][]) => node('bond_albedo', 0.7, { causes: parts.map(([k, v]) => node('cover_share', v, { label: k })) });
    expect(dominantFrost(tree([['frost_n2', 0.9], ['cloud', 0.3]]))).toBe('nitrogen frost');
    expect(dominantFrost(tree([['frost_so2', 0.4], ['land', 0.6]]))).toBeNull();
    expect(dominantFrost(tree([['ice', 0.9]]))).toBeNull();
    expect(dominantFrost(null)).toBeNull();
  });
  it('names a body with a surface liquid by its liquid, not by a frost over it', () => {
    setLocale('en');
    const tree = (parts: [string, number][]) => node('bond_albedo', 0.29, { causes: parts.map(([k, v]) => node('cover_share', v, { label: k })) });
    expect(dominantFrost(tree([['frost_co2', 0.8], ['haze', 1]]), 'hydrocarbons')).toBe(t.cards.liquids.hydrocarbons);
    expect(dominantFrost(tree([['frost_n2', 0.9]]), 'none')).toBe('nitrogen frost');
    expect(dominantFrost(tree([['frost_n2', 0.9]]), null)).toBe('nitrogen frost');
    expect(dominantFrost(tree([['land', 0.7]]), 'water')).toBeNull();
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
        if (q !== 'note' && q !== 'cover_share') expect(t.causes.quantities[q], q).not.toBe('');
      }
      for (const f of Object.keys(INPUTS) as InputField[]) expect(t.causes.inputs[f], f).toBeTruthy();
      setLocale('en');
    });
    it(`words every note the server sends in ${locale}`, () => {
      setLocale(locale);
      for (const key of ['thermostat', 'life', 'runaway', 'boiled', 'snowball', 'frozen', 'co2_build_up', 'co2_condenses', 'reducing_air', 'methane_trap', 'not_converged']) {
        expect(t.causes.notes[key], key).toBeTruthy();
        expect(formatCause(node('note', 0, { label: key })), key).toBe(t.causes.notes[key]);
      }
      // The ocean that boiled and condensed again names the water lost (#517).
      const recondensed = formatCause(node('note', 2.5, { label: 'boiled_recondensed' }));
      expect(recondensed).toBe(t.causes.boiledRecondensed({ lost: formatCause(node('water_kept', 2.5)) }));
      expect(recondensed).toContain(locale === 'en' ? '2.5' : '2,5');
      expect(formatCause(node('bond_albedo', 0.306))).toBe(locale === 'en' ? '0.306' : '0,306');
      // The host's locked k2/Q is a value without a unit (#533).
      expect(formatCause(node('host_dissipation', 2.2e-4))).toBe(locale === 'fr' ? '2,2×10⁻⁴' : '2.2×10⁻⁴');
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
