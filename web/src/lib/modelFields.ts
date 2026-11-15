// The planet-model inputs the generate form offers for a lone world: their
// groups, scales, limits and the Solar System marks on their sliders.
import type { FormSpec } from './api/types/FormSpec';
import type { InputField } from './api/types/InputField';
import type { Limits } from './api/types/Limits';

export type ModelFieldKey =
  | 'mass_earth' | 'core_mass_fraction' | 'water_oceans' | 'envelope_fraction' | 'co2' | 'n2' | 'ch4' | 'nh3' | 's'
  | 'radiogenic' | 'life' | 'eccentricity' | 'age_gyr';

export const MODEL_GROUPS: { id: 'orbit' | 'composition' | 'heat' | 'volatiles' | 'life'; fields: ModelFieldKey[] }[] = [
  { id: 'orbit', fields: ['eccentricity', 'age_gyr'] },
  { id: 'composition', fields: ['mass_earth', 'core_mass_fraction', 'water_oceans', 'envelope_fraction'] },
  { id: 'heat', fields: ['radiogenic'] },
  { id: 'volatiles', fields: ['co2', 'n2', 'ch4', 'nh3', 's'] },
  { id: 'life', fields: ['life'] },
];

/** The field a cause's input is set in; the insolation comes from the star and orbit step. */
export function fieldOf(input: InputField): ModelFieldKey | 'orbit' {
  return input === 'insolation' ? 'orbit' : input;
}

const LOG_FIELDS: ReadonlySet<ModelFieldKey> = new Set(['mass_earth', 'water_oceans', 'co2', 'n2', 'ch4', 'nh3', 's', 'radiogenic']);

/** How a slider spans the limits; water's log slider starts at a "none" stop for 0. */
export function scaleOf(key: ModelFieldKey): 'log' | 'linear' {
  return LOG_FIELDS.has(key) ? 'log' : 'linear';
}

/** A reference mark: the bodies that sit at one value of a field (Mars and Venus both hold no water). */
export type Mark = { bodies: string[]; value: number };

/** The Solar System values of a field, by body (lower-case name, as the server sends them). */
export function referencesOf(key: ModelFieldKey, spec: Pick<FormSpec, 'references'>): Map<string, number> {
  return new Map(spec.references.filter((r) => r.field === key).map((r) => [r.body, r.value]));
}

/**
 * The bodies a field quotes, in order: its slider's chips and its help line
 * use this one list (the numbers come from the server, `FormSpec.references`).
 */
export const REFERENCE_BODIES: Partial<Record<Exclude<ModelFieldKey, 'life'>, string[]>> = {
  mass_earth: ['titan', 'mars', 'venus', 'earth', 'neptune', 'jupiter'],
  core_mass_fraction: ['earth', 'mercury'],
  water_oceans: ['mars', 'venus', 'earth', 'titan'],
  envelope_fraction: ['earth', 'neptune', 'jupiter'],
  eccentricity: ['venus', 'earth', 'mars', 'mercury'],
  age_gyr: ['earth'],
  co2: ['earth'],
  n2: ['earth'],
  ch4: ['earth'],
  radiogenic: ['earth'],
};

/** The limits of a field, undefined for the checkbox. */
export function limitsOf(key: Exclude<ModelFieldKey, 'life'>, spec: FormSpec): Limits {
  return key === 'eccentricity' ? spec.eccentricity : spec.inputs[key];
}

/**
 * Where the bodies of a field (`REFERENCE_BODIES`) sit on it (the real Solar
 * System values the server sends, `FormSpec.references`), those in its limits only;
 * bodies at the same value share one mark.
 */
export function marks(key: ModelFieldKey, spec: Pick<FormSpec, 'references'>, limits?: Limits): Mark[] {
  if (key === 'life') return [];
  const refs = referencesOf(key, spec);
  const out: Mark[] = [];
  for (const body of REFERENCE_BODIES[key] ?? []) {
    const value = refs.get(body);
    if (value === undefined) continue;
    if (limits && (value < limits.min || value > limits.max)) continue;
    const same = out.find((m) => m.value === value);
    if (same) same.bodies.push(body);
    else out.push({ bodies: [body], value });
  }
  return out;
}
