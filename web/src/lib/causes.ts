// Cause trees: the values a body's cards derive, unfolded into what they come
// from (the server's `explain`), worded for the reader.
import type { Cause } from './api/types/Cause';
import type { Explanation } from './api/types/Explanation';
import type { Quantity } from './api/types/Quantity';
import { formatNumber, formatUpTo, withUnit } from './format';
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
  if (Math.abs(value) >= 1e6 || Math.abs(value) < 1e-3) return Number(value.toPrecision(3)).toExponential().replace('e+', '×10^').replace('e-', '×10^-');
  return formatUpTo(Number(value.toPrecision(3)), 6);
}

/**
 * A cause's value with its unit, or its key worded. `unitOf` is the
 * quantity whose unit an escape takes (its parent's).
 */
export function formatCause(cause: Cause, unitOf: Quantity = cause.quantity): string {
  if (cause.label !== null) return keyLabel(cause.quantity, cause.label);
  const percent = PERCENT.has(cause.quantity);
  const unit = t.causes.units[cause.quantity === 'escape' ? unitOf : cause.quantity];
  const text = percent ? formatNumber(Math.round(cause.value * 100)) : significant(cause.value);
  return unit ? withUnit(text, percent ? '%' : unit) : text;
}
