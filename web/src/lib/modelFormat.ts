// How the form says the value of a model field: its shown unit, a mass with
// its note in tonnes, and the Solar System values the help of a field quotes.
import type { FormSpec } from './api/types/FormSpec';
import { formatUpTo } from './format';
import { t } from './i18n/index.svelte';
import { formatMassWithNote } from './massFormat';
import { REFERENCE_BODIES, type ModelFieldKey, referencesOf } from './modelFields';
import { toShown } from './modelScale';

type NumericKey = Exclude<ModelFieldKey, 'life'>;

/** A value as the field shows it, without a unit beside a mass (its note carries the unit). */
function shownNumber(key: NumericKey, value: number): string {
  return formatUpTo(toShown(key, value), 3);
}

/** A value with its unit: `0.3 Earth oceans`, `32.5 %`; a mass is `1 M⊕ (5.97×10²¹ t)`. */
export function fieldValueText(key: NumericKey, value: number): string {
  if (key === 'mass_earth') return formatMassWithNote(value);
  return `${shownNumber(key, value)} ${t.form.model[key].unit}`.trim();
}

/** A Solar System value as a chip or the help line writes it: the number, and its unit when it is short (`32.5 %`, `4.6 Gyr`). */
export function referenceValueText(key: NumericKey, value: number): string {
  const unit = key === 'core_mass_fraction' || key === 'envelope_fraction' ? ' %' : key === 'age_gyr' ? ` ${t.form.model.age_gyr.unit}` : '';
  return `${shownNumber(key, value)}${unit}`;
}

/** The names of the bodies of a mark, joined (`Mars · Venus`). */
export function markName(bodies: readonly string[]): string {
  return bodies.map((body) => (t.form.model.bodies as Record<string, string>)[body] ?? body).join(' · ');
}

/** The label of the Solar System values of a field: reference masses for the mass, else reference values. */
export function referencesLabel(key: ModelFieldKey): string {
  return key === 'mass_earth' ? t.form.model.referencesMass : t.form.model.references;
}

/**
 * The line of Solar System values a field's help quotes (`Reference masses:
 * Mars 0.107 · Earth 1 · …`), built from what the server sends, or null for a
 * field with none.
 */
export function referencesLine(key: ModelFieldKey, spec: Pick<FormSpec, 'references'>): string | null {
  if (key === 'life') return null;
  const wanted = REFERENCE_BODIES[key];
  const refs = referencesOf(key, spec);
  const items = wanted.flatMap((body) => {
    const value = refs.get(body);
    return value === undefined ? [] : [`${markName([body])} ${referenceValueText(key, value)}`];
  });
  return items.length > 0 ? `${referencesLabel(key)}: ${items.join(' · ')}` : null;
}
