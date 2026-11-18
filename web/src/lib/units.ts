// The units of the interface, explained in the Units panel (long name and
// reference values) and named in short in tooltips.

import { getLocale } from './i18n/index.svelte';
import { glossName, type GlossUnit } from './glossary';

/**
 * Units explained in the Units panel, in its order: every glossed unit but
 * `°C`, which the `K` entry covers.
 */
export const UNITS = [
  'Ga', 'Ma', 'Gyr', 'bar', 'g', 'K', 'AU', 'UA', 'L☉', 'R☉', 'M⊕', 'R⊕', 'M♃', 'R♃', 'S⊕',
  'W/m²', 'µT', 'm/s²', 'mm/s²', 'km', 'km²', 'M km²', 'km/s', 'TW', 'GW', 'MW', 'W',
] as const satisfies readonly GlossUnit[];
export type Unit = (typeof UNITS)[number];

/** Tooltip naming `units`, one short name per line, without reference values. */
export function unitTooltip(...units: Unit[]): string {
  return units.map((unit) => glossName('unit', unit) ?? unit).join('\n');
}

/**
 * The units the panel lists in the active language: one row per unit, so
 * the French spelling UA replaces AU in French, and Gyr folds into Ga.
 */
export function panelUnits(): Unit[] {
  const skipped: Unit[] = getLocale() === 'fr' ? ['AU', 'Gyr'] : ['UA', 'Gyr'];
  return UNITS.filter((unit) => !skipped.includes(unit));
}
