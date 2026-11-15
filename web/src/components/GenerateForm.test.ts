import { render } from 'svelte/server';
import { describe, expect, it } from 'vitest';
import type { ArchetypeInfo } from '../lib/api/types/ArchetypeInfo';
import type { FormSpec } from '../lib/api/types/FormSpec';
import { t } from '../lib/i18n/index.svelte';
import GenerateForm from './GenerateForm.svelte';

const limits = { min: 0, max: 10 };
const spec = {
  hz_position: { limits: { min: -1, max: 4 }, fallback: [0, 1] },
  orbit_au: limits, eccentricity: { min: 0, max: 0.9 }, moon_count: { min: 1, max: 6 }, metallicity: { min: -1, max: 0.5 },
  inputs: {
    mass_earth: { min: 0.01, max: 4131 }, core_mass_fraction: limits, water_oceans: limits, envelope_fraction: limits, co2: limits,
    n2: limits, ch4: limits, radiogenic: { min: 0.1, max: 10 }, axial_tilt_deg: { min: 0, max: 90 }, age_gyr: limits,
  },
  default_width: 1024, default_height: 512, size_presets: [[1024, 512]], max_size: 4096, max_pixels: 1e8,
  archetypes: ['earth-like'], moon_archetypes: [], max_planets: 8,
} as unknown as FormSpec;
const archetypes = [
  { spec: 'earth-like', name: 'earth-like', description: '', built_in: true, inputs: { mass_earth: 1 }, orbit: { axial_tilt: 23.4 }, giant: false, moon: false, moon_host: null, locales: {} },
] as unknown as ArchetypeInfo[];

describe('GenerateForm', () => {
  it('puts the model fields in the planets step of a lone world, and the preview under a fold', () => {
    const out = render(GenerateForm, { props: { archetypes, spec, entry: 'around', onSubmitted: () => {}, onCancel: () => {} } }).body;
    expect(out).toContain(t.form.steps.planets);
    // The kind step shows no model fields; the preview is folded away from the wide layout.
    expect(out).toContain(t.form.preview.summary);
    expect(out).not.toContain('id="model-co2"');
  });

  it('lists the pinned model inputs of a varied world in the summary, with the seed it will use', () => {
    const initial = { planet: 'earth-like', seed: 42, co2: 2, ch4: 0.5, age_gyr: 3, life: false, eccentricity: 0.1 };
    const out = render(GenerateForm, { props: { archetypes, spec, initial, variedFrom: 'x', atSummary: true, onSubmitted: () => {}, onCancel: () => {} } }).body;
    for (const key of ['co2', 'ch4', 'age_gyr', 'life', 'eccentricity'] as const) expect(out).toContain(t.form.model[key].label);
    expect(out).toMatch(/Seed<\/dt> <dd[^>]*>42</);
  });
});
