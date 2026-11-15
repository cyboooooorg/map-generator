import { describe, expect, it } from 'vitest';
import { setLocale } from './i18n/index.svelte';
import { formatMassWithNote, formatTonnes, massNote } from './massFormat';

describe('massFormat', () => {
  it('gives an Earth mass in tonnes, in scientific notation', () => {
    expect(formatTonnes(1)).toBe('5.97×10²¹ t');
    expect(formatTonnes(0.0123)).toBe('7.35×10¹⁹ t');
  });
  it('uses the decimal comma in French', () => {
    setLocale('fr');
    expect(formatTonnes(1)).toBe('5,97×10²¹ t');
    expect(formatMassWithNote(317.83)).toContain('1 M♃');
  });
  it('adds Jupiter masses from 30 Earth masses only', () => {
    expect(massNote(29.9)).not.toContain('M♃');
    expect(massNote(30)).toMatch(/ · 0\.09 M♃$/);
    expect(massNote(317.83)).toMatch(/ · 1 M♃$/);
  });
  it('writes a mass with its note', () => {
    expect(formatMassWithNote(1)).toBe('1 M⊕ (5.97×10²¹ t)');
    expect(formatMassWithNote(0.107)).toBe('0.107 M⊕ (6.39×10²⁰ t)');
  });
});
