// The planet-model inputs of the generate form as it holds them (each one
// drawn or pinned), what they add to a request, and what the seed drew for
// the ones left to it, read back from the preview.
import type { ResolvedSystem } from './api/types/ResolvedSystem';
import type { WorldRequest } from './api/types/WorldRequest';
import { EARTH_MASS_KG, OCEAN_KG } from './waterAmount';
import { MODEL_GROUPS, type ModelFieldKey } from './modelFields';

/** A field is drawn (`random`) or pinned at `value`; `life` pins a boolean, the others a number. */
export type ModelValue = { random: boolean; value: number | boolean };
export type ModelValues = Record<ModelFieldKey, ModelValue>;
/** What the seed drew, by field, in the units a request pins them in. */
export type DrawnValues = Partial<Record<ModelFieldKey, number | boolean>>;

export const MODEL_KEYS: ModelFieldKey[] = MODEL_GROUPS.flatMap((g) => g.fields);

/** What a field takes when pinned before a preview has said what was drawn. */
export const PIN_DEFAULTS: Record<ModelFieldKey, number | boolean> = {
  mass_earth: 1, core_mass_fraction: 0.325, water_oceans: 1, envelope_fraction: 0, co2: 1, n2: 1, ch4: 1, nh3: 1, s: 1,
  radiogenic: 1, life: true, eccentricity: 0.017, age_gyr: 4.6,
};

/** Every field drawn, except those `from` pins (a varied world's inputs). */
export function initialModelValues(from: WorldRequest | null): ModelValues {
  const values = {} as ModelValues;
  for (const key of MODEL_KEYS) {
    const given = from?.[key];
    values[key] = given === undefined ? { random: true, value: PIN_DEFAULTS[key] } : { random: false, value: given };
  }
  return values;
}

/**
 * `next` (the values of a newly picked archetype) with the age the user
 * typed in `previous` kept: an age typed always wins over an archetype's.
 */
export function keepAge(next: ModelValues, previous: ModelValues): ModelValues {
  if (!previous.age_gyr.random) next.age_gyr = { ...previous.age_gyr };
  return next;
}

/** The pinned fields as request members: numbers that are numbers, `life` as a boolean. */
export function modelRequest(values: ModelValues): Partial<WorldRequest> {
  const request: Record<string, number | boolean> = {};
  for (const key of MODEL_KEYS) {
    const { random, value } = values[key];
    if (random) continue;
    if (key === 'life' ? typeof value === 'boolean' : typeof value === 'number' && Number.isFinite(value)) {
      request[key] = value;
    }
  }
  return request as Partial<WorldRequest>;
}

/** How many of `keys` are pinned. */
export function pinnedCount(values: ModelValues, keys: readonly ModelFieldKey[]): number {
  return keys.filter((key) => !values[key].random).length;
}

const EARTH_KG = EARTH_MASS_KG;

/**
 * What the seed drew for a lone world, as the request would pin it: the
 * mass it formed with, the water in Earth oceans and the envelope at
 * formation (as `PlanetModel::formation` reads them back), the orbit's
 * eccentricity and the star's age, and the CO₂, N₂, CH₄, NH₃ and sulphur factors the
 * server read back from the formation rule (`ResolvedSystem.formation`).
 */
export function drawnValues(result: ResolvedSystem | null): DrawnValues | null {
  const focus = result?.system.focus;
  const body = focus ? result?.system.bodies.find((b) => b.id === focus) : undefined;
  const model = body?.model;
  if (!result || !body || !model) return null;
  const i = model.inputs;
  const drawn = model.drawn_mass_earth;
  const envelope = i.mass_earth === drawn ? i.envelope_fraction : (i.mass_earth * i.envelope_fraction + drawn - i.mass_earth) / drawn;
  return {
    mass_earth: drawn,
    core_mass_fraction: i.core_mass_fraction,
    water_oceans: (i.water_fraction * drawn * EARTH_KG) / OCEAN_KG,
    envelope_fraction: envelope,
    radiogenic: i.radiogenic,
    // The volatile factors depend on the formation rule: the server reads them back.
    ...(result.formation ? volatileFactors(result.formation.overrides) : {}),
    ...(model.surface ? { life: model.surface.life.arose } : {}),
    eccentricity: body.orbit.eccentricity,
    age_gyr: result.system.star.info.age_gyr,
  };
}

function volatileFactors(overrides: NonNullable<ResolvedSystem['formation']>['overrides']): DrawnValues {
  const out: DrawnValues = {};
  for (const key of ['co2', 'n2', 'ch4', 'nh3', 's'] as const) {
    const value = overrides[key];
    if (typeof value === 'number') out[key] = value;
  }
  return out;
}

/** Sets a pinned number; one that is not a number (a cleared box reads NaN) leaves the previous value. */
export function setPinned(values: ModelValues, key: Exclude<ModelFieldKey, 'life'>, value: number): void {
  if (Number.isFinite(value)) values[key].value = value;
}

/** The radius (Earth radii) of the planet the preview drew, or null before an answer. */
export function previewRadiusEarth(result: ResolvedSystem | null): number | null {
  const focus = result?.system.focus;
  const body = focus ? result?.system.bodies.find((b) => b.id === focus) : undefined;
  return body?.model?.interior.radius_earth ?? null;
}
