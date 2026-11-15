// A mass of the generate form, said in the units a reader can feel: Earth
// masses with, in parentheses, tonnes (scientific notation, localised) and,
// from 30 Earth masses, Jupiter masses.
import { formatUpTo } from './format';

/** One Earth mass in tonnes (5.972×10²⁴ kg). */
export const TONNES_PER_EARTH = 5.972e21;
/** One Jupiter mass in Earth masses. */
export const EARTH_PER_JUPITER = 317.83;
/** The mass from which Jupiter masses are given too, Earth masses. */
export const JUPITER_FROM_EARTH = 30;

const SUPERSCRIPT: Record<string, string> = { '-': '⁻', '0': '⁰', '1': '¹', '2': '²', '3': '³', '4': '⁴', '5': '⁵', '6': '⁶', '7': '⁷', '8': '⁸', '9': '⁹' };

/** A tonnage as a power of ten with three significant digits: `5,97×10²¹ t` in French. */
export function formatTonnes(earth: number): string {
  const [mantissa, exponent] = (earth * TONNES_PER_EARTH).toExponential(2).split('e');
  const power = String(Number(exponent)).replace(/./g, (c) => SUPERSCRIPT[c] ?? c);
  return `${formatUpTo(Number(mantissa), 2)}×10${power} t`;
}

/** What goes in the parentheses after a mass: tonnes, and Jupiter masses from 30 Earth masses (`0,32 MJ`). */
export function massNote(earth: number): string {
  const tonnes = formatTonnes(earth);
  return earth >= JUPITER_FROM_EARTH ? `${tonnes} · ${formatUpTo(earth / EARTH_PER_JUPITER, 2)} MJ` : tonnes;
}

/** A mass in Earth masses with its note: `1 M⊕ (5,97×10²¹ t)`. */
export function formatMassWithNote(earth: number): string {
  return `${formatUpTo(earth, 3)} M⊕ (${massNote(earth)})`;
}
