// The less common units of the interface (time in Ma and Ga, pressure in
// bar, gravity in g): what they mean, in tooltips and in the help panel.

import { t } from './i18n/index.svelte';

/** Units explained to the user, in the order of the help panel. */
export const UNITS = ['Ga', 'Ma', 'bar', 'g'] as const;
export type Unit = (typeof UNITS)[number];

/** Tooltip explaining `units`, one line each: what the unit is, then reference values. */
export function unitTooltip(...units: Unit[]): string {
  return units.map((unit) => `${t.units[unit].name}. ${t.units[unit].examples}`).join('\n');
}
