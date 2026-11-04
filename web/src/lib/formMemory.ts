// The generation form's last inputs, remembered per browser so that "New
// world" starts where the previous generation left off. The seed is not
// kept: the same planet and seed would name an existing world.
import { loadSetting, removeSetting, saveSetting } from './settings';
import { parseStarChoice, type StarChoice } from './starForm';

const KEY = 'generateForm';

/** One parameter of the form: random, or fixed at `value`. */
export interface FieldMemory {
  random: boolean;
  value: number;
}

/** What the form remembers of its last submission. */
export interface FormMemory {
  /** Preset spec. */
  planet: string;
  randomPlanet: boolean;
  /** `<width>x<height>`. */
  size: string;
  seaBy: 'level' | 'land';
  landPercent: number;
  /** By parameter name; parameters missing here take their defaults. */
  fields: Record<string, FieldMemory>;
  /** The star and orbit; null takes the preset's. */
  star: StarChoice | null;
}

function isField(value: unknown): value is FieldMemory {
  if (typeof value !== 'object' || value === null) {
    return false;
  }
  const { random, value: v } = value as Record<string, unknown>;
  return typeof random === 'boolean' && typeof v === 'number' && Number.isFinite(v);
}

/**
 * Reads a stored form, keeping only well-formed values: `null` when the
 * essentials (planet, size) are missing, and only the parameters named in
 * `fieldNames` that are well formed.
 */
export function parseFormMemory(raw: unknown, fieldNames: readonly string[]): FormMemory | null {
  if (typeof raw !== 'object' || raw === null) {
    return null;
  }
  const r = raw as Record<string, unknown>;
  if (typeof r.planet !== 'string' || typeof r.size !== 'string' || !/^\d+x\d+$/.test(r.size)) {
    return null;
  }
  const fields: Record<string, FieldMemory> = {};
  const storedFields = typeof r.fields === 'object' && r.fields !== null ? (r.fields as Record<string, unknown>) : {};
  for (const name of fieldNames) {
    const field = storedFields[name];
    if (isField(field)) {
      fields[name] = { random: field.random, value: field.value };
    }
  }
  return {
    planet: r.planet,
    randomPlanet: r.randomPlanet === true,
    size: r.size,
    seaBy: r.seaBy === 'land' ? 'land' : 'level',
    landPercent: typeof r.landPercent === 'number' && Number.isFinite(r.landPercent) ? r.landPercent : 29,
    fields,
    star: parseStarChoice(r.star),
  };
}

/** The remembered form, or null (nothing stored, or unreadable). */
export function loadFormMemory(fieldNames: readonly string[]): FormMemory | null {
  return parseFormMemory(loadSetting<unknown>(KEY, null), fieldNames);
}

export function saveFormMemory(memory: FormMemory): void {
  saveSetting(KEY, memory);
}

/** Forgets the form: the next one starts from the preset defaults. */
export function clearFormMemory(): void {
  removeSetting(KEY);
}
