// Finds chemical formulas and units in display strings, so the interface can
// show their names in tooltips. Pure text analysis: the names live in the
// `glossary` block of the i18n catalogs.
import { t } from './i18n/index.svelte';

export type GlossKind = 'formula' | 'unit';
export type Segment = { text: string } | { text: string; kind: GlossKind; name: string };

/** Molecules, in subscript form only: ASCII `CO2`, `He`, `Ar` stay plain text. */
export const FORMULAS = ['CO₂', 'N₂', 'O₂', 'CH₄', 'C₂H₆', 'H₂O', 'NH₃', 'SO₂', 'H₂', 'H₂/He'] as const;
export type Formula = (typeof FORMULAS)[number];

/** Units whose symbol is unambiguous: glossed anywhere. */
export const SYMBOL_UNITS = [
  'M⊕', 'R⊕', 'M♃', 'R♃', 'M☉', 'L☉', 'R☉', 'S⊕', 'W/m²', 'µT', 'm/s²', 'mm/s²', '°C',
] as const;

/** Units made of plain letters: glossed only right after a number. */
export const LETTER_UNITS = [
  'bar', 'K', 'g', 'Ga', 'Ma', 'Gyr', 'AU', 'UA', 'RJ', 'g/cm³', 'km', 'km²', 'km/s', 'TW', 'GW', 'MW', 'W', 'M km²',
] as const;

export const GLOSS_UNITS = [...SYMBOL_UNITS, ...LETTER_UNITS] as const;
export type GlossUnit = (typeof GLOSS_UNITS)[number];

/** A space as typed in a key, or no-break, or narrow no-break (French). */
const SPACE = '[ \\u00a0\\u202f]';

const longestFirst = (terms: readonly string[]): string =>
  [...terms]
    .sort((a, b) => b.length - a.length)
    .map((s) => s.replace(/[.*+?^${}()|[\]\\/]/g, '\\$&').replace(/ /g, SPACE))
    .join('|');

const SUBSCRIPTS = '₀-₉';
const FORMULA_RE = new RegExp(
  `(?<![\\p{L}\\p{N}${SUBSCRIPTS}])(${longestFirst(FORMULAS)})(?![\\p{L}\\p{N}${SUBSCRIPTS}])`,
  'gu',
);
const SYMBOL_RE = new RegExp(
  `(?<![\\p{L}\\p{N}])(${longestFirst(SYMBOL_UNITS)})(?![\\p{L}\\p{N}²³])`,
  'gu',
);
// After a digit or a superscript digit (`×10²⁶`), and a space, no-break or narrow no-break.
const LETTER_RE = new RegExp(
  `(?<=[\\d⁰¹²³⁴-⁹][ \\u00a0\\u202f])(${longestFirst(LETTER_UNITS)})(?![\\p{L}\\p{N}²³/]|-\\p{L})`,
  'gu',
);

type Hit = { start: number; end: number; kind: GlossKind; term: string };

function hits(text: string): Hit[] {
  const found: Hit[] = [];
  const scan = (re: RegExp, kind: GlossKind) => {
    for (const m of text.matchAll(re)) {
      found.push({ start: m.index, end: m.index + m[0].length, kind, term: m[0] });
    }
  };
  scan(FORMULA_RE, 'formula');
  scan(SYMBOL_RE, 'unit');
  scan(LETTER_RE, 'unit');
  found.sort((a, b) => a.start - b.start || b.end - a.end);
  const kept: Hit[] = [];
  for (const h of found) if (h.start >= (kept.at(-1)?.end ?? 0)) kept.push(h);
  return kept;
}

/** The name of `term` in the active language, or null when it is not a known term. */
export function glossName(kind: GlossKind, term: string): string | null {
  const names: Record<string, string> = kind === 'formula' ? t.glossary.formulas : t.glossary.units;
  const key = term.replace(/[\u00a0\u202f]/g, ' ');
  return Object.hasOwn(names, key) ? (names[key] ?? null) : null;
}

/** Splits `text` into plain and glossed parts; joining every `text` gives back the input. */
export function glossSegments(text: string): Segment[] {
  const out: Segment[] = [];
  let at = 0;
  for (const h of hits(text)) {
    const name = glossName(h.kind, h.term);
    if (name === null) continue;
    if (h.start > at) out.push({ text: text.slice(at, h.start) });
    out.push({ text: h.term, kind: h.kind, name });
    at = h.end;
  }
  if (at < text.length) out.push({ text: text.slice(at) });
  return out;
}
