import { render } from 'svelte/server';
import { afterEach, describe, expect, it } from 'vitest';
import { setLocale, t } from '../lib/i18n/index.svelte';
import { panelUnits } from '../lib/units';
import UnitsPanel from './UnitsPanel.svelte';

const html = () => render(UnitsPanel, { props: { onClose: () => {} } }).body;

describe('UnitsPanel', () => {
  afterEach(() => setLocale('en'));

  it('is a dialog titled Units, with one abbr per unit', () => {
    const out = html();
    expect(out).toContain('role="dialog"');
    expect(out).toContain(t.units.title);
    expect(out.match(/<abbr/g)?.length).toBe(panelUnits().length);
  });
  it('explains units only, never molecules', () => {
    expect(html()).not.toContain('CO₂');
  });
  it('has a close button', () => expect(html()).toContain(`aria-label="${t.viewer.close}"`));
  it('lists one row per unit in each language', () => {
    const rows = (): string[] => [...html().matchAll(/<abbr>([^<]*)<\/abbr>/g)].map((m) => m[1] ?? '');
    const en = rows();
    expect(en).toContain('AU');
    expect(en).not.toContain('UA');
    expect(en).toContain('Ga');
    expect(en).not.toContain('Gyr');
    setLocale('fr');
    const fr = rows();
    expect(fr).toContain('UA');
    expect(fr).not.toContain('AU');
    expect(fr).toContain('Ga');
    expect(fr).not.toContain('Gyr');
  });
  it('mentions Gyr inside the English Ga entry', () => expect(t.units.Ga.name).toContain('Gyr'));
});
