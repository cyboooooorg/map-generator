import { afterEach, describe, expect, it, vi } from 'vitest';
import { contrast, loadTheme, THEME_NAMES, THEMES, TOKENS } from './themes';

describe('themes', () => {
  afterEach(() => vi.unstubAllGlobals());

  it('defines every token in every theme, Default with the original values', () => {
    for (const name of THEME_NAMES) {
      for (const token of TOKENS) expect(THEMES[name][token], `${name} ${token}`).toBeTruthy();
    }
    expect(THEMES.default.bg).toBe('#0b0e14');
    expect(THEMES.default.panel).toBe('#141924ee');
    expect(THEMES.default.accent).toBe('#336de0');
    expect(THEMES.default.muted).toBe('#8b95a7');
  });

  it('keeps text readable in every theme (WCAG AA, 4.5:1)', () => {
    for (const name of THEME_NAMES) {
      const theme = THEMES[name];
      for (const fg of ['text', 'muted', 'link'] as const) {
        for (const bg of ['panel-solid', 'bg', 'control-bg'] as const) {
          expect(contrast(theme[fg], theme[bg]), `${name}: ${fg} on ${bg}`).toBeGreaterThanOrEqual(4.5);
        }
      }
      expect(contrast(theme['accent-text'], theme.accent), `${name}: accent text`).toBeGreaterThanOrEqual(4.5);
    }
  });

  it('computes contrast as WCAG does', () => {
    expect(contrast('#000', '#fff')).toBeCloseTo(21, 5);
    expect(contrast('#777', '#777')).toBeCloseTo(1, 5);
  });

  it('falls back to Default for an unknown theme, or without storage', () => {
    const store = new Map<string, string>([['mapgen.theme', JSON.stringify('neon-pink')]]);
    vi.stubGlobal('localStorage', { getItem: (k: string) => store.get(k) ?? null, setItem: () => {}, removeItem: () => {} });
    expect(loadTheme()).toBe('default');
    store.set('mapgen.theme', JSON.stringify('holo'));
    expect(loadTheme()).toBe('holo');
    vi.stubGlobal('localStorage', {
      getItem: () => {
        throw new Error('blocked');
      },
    });
    expect(loadTheme()).toBe('default');
  });
});
