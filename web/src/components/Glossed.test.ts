import { render } from 'svelte/server';
import { describe, expect, it } from 'vitest';
import Glossed from './Glossed.svelte';

const html = (props: { text: string; inert?: boolean }) => render(Glossed, { props }).body;

describe('Glossed', () => {
  it('renders CO₂ as a focusable abbr with its name', () => {
    const body = html({ text: 'CO₂' });
    expect(body).toContain('<abbr class="gloss"');
    expect(body).toContain('title="carbon dioxide"');
    expect(body).toContain('tabindex="0"');
    expect(body).toContain('CO₂');
  });

  it('inert glossed terms are not focusable', () => {
    const body = html({ text: 'CO₂', inert: true });
    expect(body).toContain('title="carbon dioxide"');
    expect(body).not.toContain('tabindex');
  });

  it('plain text stays plain', () => {
    expect(html({ text: 'no units here' })).not.toContain('<abbr');
  });
});
