// Cause trees: the values a body's cards derive, unfolded into what they come
// from (the server's `explain`), worded for the reader.
import type { Cause } from './api/types/Cause';
import type { Explanation } from './api/types/Explanation';
import type { InputField } from './api/types/InputField';
import type { Quantity } from './api/types/Quantity';
import { formatCelsius, formatMassEarth, formatNumber, formatUpTo, withUnit } from './format';
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

const SUPERSCRIPT: Record<string, string> = { '-': '⁻', '0': '⁰', '1': '¹', '2': '²', '3': '³', '4': '⁴', '5': '⁵', '6': '⁶', '7': '⁷', '8': '⁸', '9': '⁹' };

/** A power of ten written with superscript digits: 1.76×10¹³. */
function scientific(value: number): string {
  const [mantissa, exponent] = Number(value.toPrecision(3)).toExponential().split('e');
  const digits = String(Number(exponent)).replace(/./g, (c) => SUPERSCRIPT[c] ?? c);
  return `${mantissa}×10${digits}`;
}

/** A number to three significant digits. */
function significant(value: number): string {
  if (value === 0) return '0';
  if (Math.abs(value) >= 1e6 || Math.abs(value) < 1e-3) return scientific(value);
  return formatUpTo(Number(value.toPrecision(3)), 6);
}

/**
 * A cause's value with its unit, or its key worded. `unitOf` is the
 * quantity whose unit an escape takes (its parent's).
 */
export function formatCause(cause: Cause, unitOf: Quantity = cause.quantity): string {
  if (cause.label !== null) return keyLabel(cause.quantity, cause.label);
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
