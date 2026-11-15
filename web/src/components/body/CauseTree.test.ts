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
const html = (props: { tree: Cause | null; open: boolean; ontoggle: () => void }) => render(CauseTree, { props }).body;

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
});
