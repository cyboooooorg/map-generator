import { render } from 'svelte/server';
import type { ArchetypeInfo } from '../../lib/api/types/ArchetypeInfo';
import { describe, expect, it } from 'vitest';
import type { ErrorDetail } from '../../lib/api/types/ErrorDetail';
import type { FormSpec } from '../../lib/api/types/FormSpec';
import type { Quantity } from '../../lib/api/types/Quantity';
import { t } from '../../lib/i18n/index.svelte';
import { type DrawnValues, initialModelValues, type ModelValues } from '../../lib/modelRequest';
import ModelFields from './ModelFields.svelte';

const spec = {
  eccentricity: { min: 0, max: 0.9 },
  references: [
    { body: 'earth', field: 'mass_earth', value: 1 }, { body: 'mars', field: 'mass_earth', value: 0.107 },
    { body: 'neptune', field: 'mass_earth', value: 17.15 }, { body: 'jupiter', field: 'mass_earth', value: 317.8 },
    { body: 'earth', field: 'core_mass_fraction', value: 0.325 }, { body: 'mercury', field: 'core_mass_fraction', value: 0.7 },
  ],
  inputs: {
    mass_earth: { min: 0.01, max: 4131 }, core_mass_fraction: { min: 0, max: 0.9 }, water_oceans: { min: 0, max: 1e6 },
    envelope_fraction: { min: 0, max: 0.999 }, co2: { min: 0, max: 100 }, n2: { min: 0, max: 100 }, ch4: { min: 0, max: 100 }, nh3: { min: 0, max: 100 }, s: { min: 0, max: 100 },
    radiogenic: { min: 0.1, max: 10 }, axial_tilt_deg: { min: 0, max: 90 }, age_gyr: { min: 0, max: 1e4 },
  },
} as unknown as FormSpec;
function html(over: { values?: ModelValues; drawn?: DrawnValues | null; radiusEarth?: number | null; ignored?: string[]; refusal?: ErrorDetail | null; open?: Set<string>; ageArchetype?: ArchetypeInfo | null; driven?: Quantity[] | ((key: string) => Quantity[]); explained?: boolean } = {}): string {
  return render(ModelFields, {
    props: {
      spec,
      values: over.values ?? initialModelValues(null),
      drawn: over.drawn ?? null,
      radiusEarth: over.radiusEarth ?? null,
      driven: (key: string) => (typeof over.driven === 'function' ? over.driven(key) : (over.driven ?? [])),
      explained: over.explained ?? false,
      ignored: over.ignored ?? [],
      refusal: over.refusal ?? null,
      open: over.open ?? new Set<string>(),
      ageArchetype: over.ageArchetype ?? null,
    },
  }).body;
}
/** The markup of one field. */
const field = (out: string, key: string) => {
  const start = out.indexOf(`id="model-${key}"`);
  expect(start).toBeGreaterThan(-1);
  const next = out.indexOf('id="model-', start + 1);
  return out.slice(start, next === -1 ? undefined : next);
};

const mars = { spec: 'mars-like', name: 'mars-like', locales: {}, inputs: { mass_earth: 0.107, age_gyr: 4.57 } } as unknown as ArchetypeInfo;
const jupiter = { spec: 'hot-jupiter', name: 'hot-jupiter', locales: {}, inputs: { mass_earth: 300 } } as unknown as ArchetypeInfo;

describe('ModelFields age and volatiles', () => {
  it('offers the NH3 and sulphur factors in the volatiles group', () => {
    const out = html();
    const group = out.slice(out.indexOf('data-group="volatiles"'), out.indexOf('data-group="life"'));
    expect(group).toContain('id="model-nh3"');
    expect(group).toContain('id="model-s"');
    expect(group.indexOf('id="model-ch4"')).toBeLessThan(group.indexOf('id="model-nh3"'));
  });

  it('shows the age an archetype sets, and who sets it', () => {
    const out = field(html({ ageArchetype: mars }), 'age_gyr');
    expect(out).toContain('4.57');
    expect(out).toContain(t.form.model.ageSetBy({ name: 'mars-like' }));
    expect(out).not.toContain(t.form.model.ageWarning({ name: 'mars-like', age: '4.57' }));
  });

  it('warns when another age is typed', () => {
    const values = initialModelValues({ age_gyr: 2 });
    const out = field(html({ values, ageArchetype: mars }), 'age_gyr');
    expect(out).toContain(t.form.model.ageWarning({ name: 'mars-like', age: '4.57' }));
  });

  it('says nothing for an archetype that sets no age, or none', () => {
    for (const ageArchetype of [jupiter, null]) {
      const out = field(html({ values: initialModelValues({ age_gyr: 2 }), ageArchetype }), 'age_gyr');
      expect(out).not.toContain(t.form.model.ageSetBy({ name: 'hot-jupiter' }));
      expect(out).not.toContain('Gyr old');
    }
  });
});

describe('ModelFields', () => {
  it('says the share of the mass and the depth of a set amount of water', () => {
    const values = initialModelValues({ mass_earth: 4.78, water_oceans: 2390 });
    const out = field(html({ values, radiusEarth: 1.6 }), 'water_oceans');
    expect(out).toContain(t.form.model.waterShare({ share: '12\u00a0%' }).replace('\u00a0', ' '));
    expect(out).toContain('km deep');
    // Without a preview radius only the share shows; a drawn water shows nothing.
    expect(field(html({ values }), 'water_oceans')).not.toContain('deep');
    expect(field(html({ values: initialModelValues(null), radiusEarth: 1 }), 'water_oceans')).not.toContain('of the planet');
  });

  it('folds every group by default and opens the ones asked', () => {
    const out = html();
    expect(out.match(/<details/g)).toHaveLength(5);
    expect(out).not.toMatch(/<details[^>]*\sopen/);
    expect(html({ open: new Set(['composition']) })).toMatch(/<details[^>]*open[^>]*data-group="composition"|<details open[^>]*data-group="composition"/);
  });

  it('says a field the planet has no use for is not used', () => {
    const out = html({ ignored: ['envelope_fraction'] });
    expect(field(out, 'envelope_fraction')).toContain(t.form.model.notUsed);
    expect(field(out, 'mass_earth')).not.toContain(t.form.model.notUsed);
  });

  it('says a field no cause tree of the body reads is not used, once a preview exists', () => {
    const driven = (key: string): Quantity[] => (key === 'water_oceans' ? [] : ['pressure']);
    const out = html({ driven, explained: true });
    expect(field(out, 'water_oceans')).toContain(t.form.model.notUsed);
    expect(field(out, 'mass_earth')).not.toContain(t.form.model.notUsed);
    // Before any preview nothing is known to be unused.
    expect(field(html({ driven, explained: false }), 'water_oceans')).not.toContain(t.form.model.notUsed);
  });

  it('shows a refusal at its own field only', () => {
    const refusal: ErrorDetail = { code: 'out_of_range', field: 'co2', min: 0, max: 100 };
    const out = html({ refusal });
    expect(field(out, 'co2')).toContain('role="alert"');
    expect(field(out, 'n2')).not.toContain('role="alert"');
    expect(out.match(/role="alert"/g)).toHaveLength(1);
  });

  it('puts the Earth and Mars marks under a pinned mass slider', () => {
    const values = initialModelValues({ mass_earth: 2 });
    const mass = field(html({ values }), 'mass_earth');
    expect(mass).toContain('type="range"');
    expect(mass).toContain(`aria-label="${t.form.model.referencesMass}.`);
    expect(mass).toContain(`${t.form.model.bodies.earth} 1`);
    expect(mass).toContain(`${t.form.model.bodies.mars} 0.107`);
    // A mass is also said in tonnes, on the chips and beside the unit.
    expect(mass).toContain('(5.97×10²¹ t)');
    expect(mass).toContain('(6.39×10²⁰ t)');
  });

  it('draws the ticks and the chips from the server’s values for every field with marks', () => {
    const cmf = field(html({ values: initialModelValues({ core_mass_fraction: 0.5 }) }), 'core_mass_fraction');
    expect(cmf).toContain(`${t.form.model.bodies.earth} 32.5 %`);
    expect(cmf).toContain(`${t.form.model.bodies.mercury} 70 %`);
    expect(cmf.match(/class="tick[ "]/g)).toHaveLength(2);
    // The field does not quote Mars: no Mars chip, nothing from an archetype.
    expect(cmf).not.toContain(t.form.model.bodies.mars);
  });

  it('quotes the same reference values in the help line as on the chips', () => {
    const out = html({ values: initialModelValues({ mass_earth: 2 }) });
    const line = field(out, 'mass_earth').match(/<p class="unit-help references[^"]*">([^<]*)<\/p>/)![1]!;
    expect(line).toBe('Reference masses: Mars 0.107 · Earth 1 · Neptune 17.15 · Jupiter 317.8');
    // The chips quote the same bodies as the line.
    for (const body of ['mars', 'earth', 'neptune', 'jupiter'] as const) expect(field(out, 'mass_earth')).toContain(`${t.form.model.bodies[body]} `);
    expect(field(out, 'core_mass_fraction')).toContain('Reference values: Earth 32.5 % · Mercury 70 %');
  });

  it('shows the drawn value, greyed, on a drawn field', () => {
    const out = html({ drawn: { mass_earth: 1.085, water_oceans: 0.9 } });
    const mass = field(out, 'mass_earth');
    expect(mass.replace(/<!--.*?-->/g, '')).toContain('Drawn: 1.085 <abbr title="Earth masses (5.97×10²¹ t)">M⊕</abbr> (6.48×10²¹ t)');
    expect(mass).not.toContain('type="range"');
    expect(field(out, 'water_oceans')).toContain('Drawn: 0.9 Earth oceans');
  });

  it('names what a field drives from the preview, else its static line', () => {
    const live = html({ driven: ['surface_temperature', 'pressure'] });
    expect(field(live, 'co2')).toContain(`${t.causes.quantities.surface_temperature}, ${t.causes.quantities.pressure}`);
    expect(field(html(), 'co2')).toContain(t.form.model.co2.drives);
  });

  it('offers life as drawn, with life or lifeless', () => {
    const life = field(html({ drawn: { life: true } }), 'life');
    expect(life.match(/type="radio"/g)).toHaveLength(3);
    expect(life).toContain(t.form.model.lifeDrawnValue({ arose: true }));
    const pinned = field(html({ values: initialModelValues({ life: false }) }), 'life');
    expect(pinned).not.toContain(t.form.model.lifeDrawnValue({ arose: true }));
  });

  it('counts the fields a group has pinned', () => {
    const out = html({ values: initialModelValues({ co2: 2, n2: 1 }) });
    expect(out).toContain(t.form.model.groupSet({ count: 2 }));
    expect(out).toContain(t.form.model.groupDrawn);
  });
});
