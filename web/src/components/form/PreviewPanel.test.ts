import { render } from 'svelte/server';
import { describe, expect, it } from 'vitest';
import type { ResolvedSystem } from '../../lib/api/types/ResolvedSystem';
import { Preview } from '../../lib/preview.svelte';
import { previewSource } from '../../lib/cardSource';
import { t } from '../../lib/i18n/index.svelte';
import earth from '../body/fixtures/earth-like.json';
import BodyHeader from '../body/BodyHeader.svelte';
import MoonsCard from '../body/MoonsCard.svelte';
import PreviewPanel from './PreviewPanel.svelte';

const earthLike = earth as unknown as ResolvedSystem;
const html = (preview: Preview) => render(PreviewPanel, { props: { preview, onjump: () => {}, onreroll: () => {} } }).body;

describe('PreviewPanel', () => {
  it('renders the cards of a result and its seed', () => {
    const preview = new Preview();
    preview.result = earthLike;
    const out = html(preview);
    expect(out).toContain(`<code>${earthLike.system.seed}</code>`);
    expect(out).toContain(t.form.preview.newDraw);
    // The orbit, interior and atmosphere cards are all there.
    expect(out).toContain(t.card.orbit);
    expect(out).toContain(t.causes.why);
    expect(out).not.toContain(t.form.preview.unavailable);
  });

  it('says so when the preview is unavailable', () => {
    const preview = new Preview();
    preview.unavailable = true;
    expect(html(preview)).toContain(t.form.preview.unavailable);
  });

  it('is quiet while updating and waits for a first answer', () => {
    const preview = new Preview();
    preview.pending = true;
    const out = html(preview);
    expect(out).toContain(t.form.preview.updating);
    expect(out).toContain(t.form.preview.waiting);
  });
});

describe('a preview in the cards', () => {
  const source = previewSource(earthLike, new Set());
  const header = (world: typeof source) => render(BodyHeader, { props: { world, onRenamed: () => {}, onGenerate: () => {} } }).body;

  it('offers neither renaming nor generating', () => {
    expect(source.preview).toBe(true);
    const out = header(source);
    expect(out).not.toContain(t.worldInfo.rename);
    expect(out).not.toContain(t.card.generate);
    // The same body of a loaded world would offer both.
    const out2 = header({ ...source, preview: false, systemId: 'a-1' });
    expect(out2).toContain(t.worldInfo.rename);
    expect(out2).toContain(t.card.generate);
  });

  it('shows moons as plain names, not links', () => {
    const planetWithMoons = earthLike.system.bodies.find((b) => earthLike.system.bodies.some((m) => m.parent === b.id));
    expect(planetWithMoons).toBeDefined();
    const world = { ...previewSource(earthLike, new Set()), shownSystemBody: planetWithMoons! };
    expect(render(MoonsCard, { props: { world } }).body).not.toMatch(/<button[^>]*class="link"/);
    expect(render(MoonsCard, { props: { world: { ...world, preview: false } } }).body).toMatch(/<button[^>]*class="link"/);
  });
});
