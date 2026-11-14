import { createRawSnippet } from 'svelte';
import { render } from 'svelte/server';
import { describe, expect, it } from 'vitest';
import { t } from '../../lib/i18n/index.svelte';
import Dock, { type DockCard, type DockPanel } from './Dock.svelte';

const snippet = (text: string) => createRawSnippet(() => ({ render: () => `<p>${text}</p>` }));
const card = (id: string, extra: Partial<DockCard> = {}): DockCard => ({ id, title: `Title ${id}`, content: snippet(id), ...extra });

const html = (panels: DockPanel[]): string => render(Dock, { props: { kind: 'planet', panels, defaultActive: panels[0]!.id } }).body;

describe('Dock cards', () => {
  const panels: DockPanel[] = [
    { id: 'info', title: 'Info', icon: 'info', wide: true, cards: [card('info.a'), card('info.b', { full: true }), card('info.c', { detachable: false })] },
  ];

  it('renders every card of the open panel in the grid', () => {
    const out = html(panels);
    for (const id of ['a', 'b', 'c']) expect(out).toContain(`Title info.${id}`);
    expect(out).toContain('cards');
    expect(out).toMatch(/class="body[^"]*wide/);
  });

  it('gives a detach button to each card but the non-detachable one', () => {
    const out = html(panels);
    const section = (id: string) => out.split('<section').find((s) => s.includes(`Title ${id}`)) ?? '';
    expect(section('info.a')).toContain('icon-button');
    expect(section('info.b')).toContain('icon-button');
    expect(section('info.c')).not.toContain('icon-button');
  });

  it('marks a full card as spanning both columns', () => {
    const out = html(panels);
    const section = (id: string) => out.split('<section').find((s) => s.includes(`Title ${id}`)) ?? '';
    expect(section('info.b')).toMatch(/class="card[^"]*full/);
    expect(section('info.a')).not.toMatch(/class="card[^"]*full/);
  });

  it('offers no panel-level detach on a panel of cards, but one on a plain panel', () => {
    const header = (out: string) => out.split('<header')[1]!.split('</header>')[0]!;
    expect(header(html(panels))).not.toContain(`aria-label="${t.dock.detach}"`);
    expect(header(html([{ id: 'p', title: 'Plain', icon: 'info', content: snippet('x') }]))).toContain(`aria-label="${t.dock.detach}"`);
  });

  it('still renders a plain panel from its content', () => {
    const out = html([{ id: 'p', title: 'Plain', icon: 'info', content: snippet('plain-body') }]);
    expect(out).toContain('plain-body');
  });
});
