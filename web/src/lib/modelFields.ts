// The planet-model inputs the generate form offers for a lone world: their
// groups, scales, limits and the Solar System marks on their sliders.
import type { ArchetypeInfo } from './api/types/ArchetypeInfo';
import type { FormSpec } from './api/types/FormSpec';
import type { InputField } from './api/types/InputField';
import type { Limits } from './api/types/Limits';
import type { ParamDefault } from './api/types/ParamDefault';

export type ModelFieldKey =
  | 'mass_earth' | 'core_mass_fraction' | 'water_oceans' | 'envelope_fraction' | 'co2' | 'n2' | 'ch4'
  | 'radiogenic' | 'life' | 'eccentricity' | 'age_gyr';

export const MODEL_GROUPS: { id: 'orbit' | 'composition' | 'heat' | 'volatiles' | 'life'; fields: ModelFieldKey[] }[] = [
  { id: 'orbit', fields: ['eccentricity', 'age_gyr'] },
  { id: 'composition', fields: ['mass_earth', 'core_mass_fraction', 'water_oceans', 'envelope_fraction'] },
  { id: 'heat', fields: ['radiogenic'] },
  { id: 'volatiles', fields: ['co2', 'n2', 'ch4'] },
  { id: 'life', fields: ['life'] },
];

/** The field a cause's input is set in; the insolation comes from the star and orbit step. */
export function fieldOf(input: InputField): ModelFieldKey | 'orbit' {
  return input === 'insolation' ? 'orbit' : input;
}

const LOG_FIELDS: ReadonlySet<ModelFieldKey> = new Set(['mass_earth', 'water_oceans', 'co2', 'n2', 'ch4', 'radiogenic']);

/** How a slider spans the limits; water's log slider starts at a "none" stop for 0. */
export function scaleOf(key: ModelFieldKey): 'log' | 'linear' {
  return LOG_FIELDS.has(key) ? 'log' : 'linear';
}

export type Mark = { body: 'earth' | 'mars' | 'venus' | 'titan'; value: number };

const MARK_BODIES: [Mark['body'], string][] = [
  ['earth', 'earth-like'], ['mars', 'mars-like'], ['venus', 'venus-like'], ['titan', 'titan-like'],
];

/** The limits of a field, undefined for the checkbox. */
export function limitsOf(key: Exclude<ModelFieldKey, 'life'>, spec: FormSpec): Limits {
  return key === 'eccentricity' ? spec.eccentricity : spec.inputs[key];
}

function centre(value: ParamDefault, log: boolean): number {
  if (typeof value === 'number') return value;
  const [lo, hi] = value;
  // A geometric mean needs positive bounds (Venus's water starts at 0).
  return log && lo > 0 && hi > 0 ? Math.sqrt(lo * hi) : (lo + hi) / 2;
}

/** Where the built-in Earth, Mars, Venus and Titan archetypes sit on a field, those in its limits only. */
export function marks(key: ModelFieldKey, archetypes: ArchetypeInfo[], limits?: Limits): Mark[] {
  if (key === 'life' || key === 'eccentricity' || key === 'age_gyr') return [];
  const out: Mark[] = [];
  for (const [body, spec] of MARK_BODIES) {
    const info = archetypes.find((a) => a.built_in && a.spec === spec);
    const set = info?.inputs[key];
    if (set === undefined || set === null) continue;
    const value = centre(set, scaleOf(key) === 'log');
    if (limits && (value < limits.min || value > limits.max)) continue;
    out.push({ body, value });
  }
  return out;
}
