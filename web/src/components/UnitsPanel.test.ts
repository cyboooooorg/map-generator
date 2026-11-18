import { render } from 'svelte/server';
import { describe, expect, it } from 'vitest';
import { t } from '../lib/i18n/index.svelte';
import { UNITS } from '../lib/units';
import UnitsPanel from './UnitsPanel.svelte';

const html = () => render(UnitsPanel, { props: { onClose: () => {} } }).body;

describe('UnitsPanel', () => {
  it('is a dialog titled Units, with one abbr per unit', () => {
    const out = html();
    expect(out).toContain('role="dialog"');
    expect(out).toContain(t.units.title);
    expect(out.match(/<abbr/g)?.length).toBe(UNITS.length);
  });
  it('explains units only, never molecules', () => {
    expect(html()).not.toContain('CO₂');
  });
  it('has a close button', () => expect(html()).toContain(`aria-label="${t.viewer.close}"`));
});
