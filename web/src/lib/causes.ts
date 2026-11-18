// Cause trees: the values a body's cards derive, unfolded into what they come
// from (the server's `explain`), worded for the reader.
import type { Cause } from './api/types/Cause';
import type { Explanation } from './api/types/Explanation';
import type { InputField } from './api/types/InputField';
import type { Quantity } from './api/types/Quantity';
import { formatCelsius, formatMassEarth, formatNumber, formatScientific, formatSurfaceAge, formatUpTo, withUnit } from './format';
import { t } from './i18n/index.svelte';
import { className } from './i18n/names.svelte';

/** Shares the server sends as 0 to 1, shown as percent. */
const PERCENT: ReadonlySet<Quantity> = new Set(['core_mass_fraction', 'envelope_fraction', 'envelope_lost']);

/** The root of a quantity in the explanation, null without one. */
export function treeFor(explanation: Explanation | null, quantity: Quantity): Cause | null {
  return explanation?.trees.find((tree) => tree.quantity === quantity) ?? null;
}

/** The label of a key that is not a number, the key itself when unknown. */
function keyLabel(quantity: Quantity, key: string): string {
  const tables: Partial<Record<Quantity, Record<string, string>>> = {
    regime: t.regimes as Record<string, string>,
    liquid: t.cards.liquids as Record<string, string>,
    core_state: t.cards.coreStates as Record<string, string>,
    note: t.causes.notes,
  };
  if (quantity === 'class') return className(key);
  return tables[quantity]?.[key] ?? key;
}

/** A number to three significant digits. */
function significant(value: number): string {
  if (value === 0) return '0';
  if (Math.abs(value) >= 1e6 || Math.abs(value) < 1e-3) return formatScientific(value);
  return formatUpTo(Number(value.toPrecision(3)), 6);
}

/**
 * A cause's value with its unit, or its key worded. `unitOf` is the
 * quantity whose unit an escape takes (its parent's).
 */
export function formatCause(cause: Cause, unitOf: Quantity = cause.quantity): string {
  // A part of a surface (its share) or an age that a renewal set: both the key and the value.
  if (cause.label !== null && (cause.quantity === 'terrain_unit' || cause.quantity === 'surface_age')) {
    const value = cause.quantity === 'terrain_unit' ? withUnit(formatNumber(Math.round(cause.value * 100)), '%') : formatSurfaceAge(cause.value);
    return t.causes.renewalValue({ renewal: (t.causes.renewals as Record<string, string>)[cause.label] ?? cause.label, value });
  }
  if (cause.label !== null) return keyLabel(cause.quantity, cause.label);
  if (cause.quantity === 'surface_age') return formatSurfaceAge(cause.value);
  // The same quantity is written as the cards write it.
  if (cause.quantity === 'surface_temperature' || cause.quantity === 'equilibrium_temperature') return formatCelsius(cause.value);
  if (cause.quantity === 'mass') return formatMassEarth(cause.value);
  if (cause.quantity === 'water_kept' || cause.quantity === 'water_formed') return withUnit(formatUpTo(cause.value, 3), t.causes.units[cause.quantity]);
  const percent = PERCENT.has(cause.quantity);
  const unit = t.causes.units[cause.quantity === 'escape' ? unitOf : cause.quantity];
  const text = percent ? formatNumber(Math.round(cause.value * 100)) : significant(cause.value);
  return unit ? withUnit(text, percent ? '%' : unit) : text;
}

/** Whether a cause or one below it comes from the request's `field`. */
function hasInput(cause: Cause, field: InputField): boolean {
  return cause.input === field || cause.causes.some((below) => hasInput(below, field));
}

/** The roots whose tree has a leaf set by `field`, in tree order. */
export function drivenBy(explanation: Explanation | null, field: InputField): Quantity[] {
  return (explanation?.trees ?? []).filter((tree) => hasInput(tree, field)).map((tree) => tree.quantity);
}
