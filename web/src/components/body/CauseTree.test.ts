import { render } from 'svelte/server';
import { describe, expect, it } from 'vitest';
import type { Cause } from '../../lib/api/types/Cause';
import CauseTree from './CauseTree.svelte';

const node = (over: Partial<Cause>): Cause => ({ quantity: 'pressure', value: 1, label: null, input: null, causes: [], ...over });
const tree = node({
  quantity: 'surface_temperature',
  value: 288,
  causes: [node({ quantity: 'light_received', value: 1, input: 'insolation' }), node({ quantity: 'greenhouse', value: 33 })],
});
const bare = (body: string) => body.replace(/<!--.*?-->/g, '');
const html = (props: { tree: Cause | null; open: boolean; ontoggle: () => void }) => bare(render(CauseTree, { props }).body);

describe('CauseTree', () => {
  it('shows nothing without a tree', () => expect(html({ tree: null, open: false, ontoggle: () => {} })).not.toContain('button'));
  it('is closed by default: a mark and no causes', () => {
    const out = html({ tree, open: false, ontoggle: () => {} });
    expect(out).toContain('aria-expanded="false"');
    expect(out).not.toContain('Greenhouse');
  });
  it('styles an input node as an input and names its field', () => {
    const out = html({ tree, open: true, ontoggle: () => {} });
    expect(out).toContain('aria-expanded="true"');
    const span = out.match(/<span[^>]*>[^<]*Light received[^<]*<\/span>/)?.[0] ?? '';
    expect(span).toMatch(/class="[^"]*\binput\b/);
    expect(span).toMatch(/title="[^"]*Light received/);
    expect(out).toContain('Greenhouse warming');
    expect(out.match(/\binput\b/g)?.length).toBe(1);
  });
  it('has buttons that never submit a form around the tree', () => {
    const out = bare(render(CauseTree, { props: { tree, open: true, ontoggle: () => {}, oninput: () => {} } }).body);
    const buttons = out.match(/<button\b[^>]*>/g) ?? [];
    expect(buttons.length).toBeGreaterThan(1);
    for (const button of buttons) expect(button, button).toContain('type="button"');
  });

  it('glosses a formula and a unit of a cause line', () => {
    const t2 = node({ quantity: 'pressure', value: 1, causes: [node({ quantity: 'co2', value: 0.5 })] });
    const out = html({ tree: t2, open: true, ontoggle: () => {} });
    expect(out).toContain('<abbr class="gloss"');
    expect(out).toContain('title="carbon dioxide"');
    expect(out).toContain('title="bar: pressure"');
  });
  it('keeps an input button free of focusable glosses', () => {
    const t2 = node({ quantity: 'pressure', value: 1, causes: [node({ quantity: 'co2', value: 0.5, input: 'insolation' })] });
    const out = bare(render(CauseTree, { props: { tree: t2, open: true, ontoggle: () => {}, oninput: () => {} } }).body);
    const button = out.match(/<button[^>]*class="input[ "][^>]*>[^]*?<\/button>/)?.[0] ?? '';
    expect(button).toContain('<abbr class="gloss"');
    expect(button).not.toContain('tabindex');
  });
});
