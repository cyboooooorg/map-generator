import { render } from 'svelte/server';
import { describe, expect, it } from 'vitest';
import type { ResolvedSystem } from '../../lib/api/types/ResolvedSystem';
import { formatAu } from '../../lib/format';
import { Preview } from '../../lib/preview.svelte';
import { previewSource } from '../../lib/cardSource';
import { detailText } from '../../lib/i18n/errors';
import { t } from '../../lib/i18n/index.svelte';
import earth from '../body/fixtures/earth-like.json';
import BodyHeader from '../body/BodyHeader.svelte';
import MoonsCard from '../body/MoonsCard.svelte';
import PreviewPanel from './PreviewPanel.svelte';

const earthLike = earth as unknown as ResolvedSystem;
const planet = earthLike.system.bodies.find((b) => b.id === earthLike.system.focus)!;
/** A whole system: no focus body, one planet. */
const systemResult = {
  ...earthLike,
  extras: {},
  system: { ...earthLike.system, focus: null, bodies: [{ ...planet, id: 'b', designation: 'MGS 1 b', parent: null }] },
} as unknown as ResolvedSystem;
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

  it('says the cards are not the current request while it is refused, and no button of them submits the form', () => {
    const preview = new Preview();
    preview.result = earthLike;
    expect(html(preview)).not.toContain(t.form.preview.mismatch);
    preview.refusal = { code: 'out_of_range', field: 'co2', min: 0, max: 100 } as never;
    const out = html(preview);
    expect(out).toContain(t.form.preview.mismatch);
    expect(out).toContain(detailText(preview.refusal));
    // The old cards stay under the notice, greyed and out of reach.
    expect(out).toMatch(/class="content[^"]*mismatch[^"]*"[^>]*inert/);
    // The panel sits inside the generate form: a button with no type would submit it.
    const buttons = out.match(/<button\b[^>]*>/g) ?? [];
    expect(buttons.length).toBeGreaterThan(0);
    for (const button of buttons) expect(button, button).toContain('type="button"');
  });

  it('also marks the last answer when the server does not answer', () => {
    const preview = new Preview();
    preview.result = earthLike;
    preview.unavailable = true;
    expect(html(preview)).toContain(t.form.preview.mismatchUnavailable);
  });

  it('keeps no title of its own when folded under the Preview summary', () => {
    const preview = new Preview();
    preview.result = earthLike;
    expect(html(preview)).toMatch(/<h3\b/);
    const folded = render(PreviewPanel, { props: { preview, folded: true, onjump: () => {}, onreroll: () => {} } }).body;
    expect(folded).not.toMatch(/<h3\b/);
    expect(folded).toContain(t.form.preview.newDraw);
  });

  it('puts the "Why?" of the class on the class line, not on a line of its own', () => {
    const preview = new Preview();
    preview.result = earthLike;
    const out = html(preview);
    expect(out).toMatch(/data-line="class">\s*<small[^>]*>[^<]*<\/small>\s*<!--\[0--><button[^>]*class="why/);
  });

  it('shows a whole system as its star and its planets, and none of the lone world’s cards', () => {
    const preview = new Preview();
    preview.result = systemResult;
    const out = render(PreviewPanel, { props: { preview, kind: 'system', onreroll: () => {} } }).body;
    expect(out).toContain(t.form.preview.system.planets);
    expect(out).toContain('MGS 1 b');
    expect(out).toContain(formatAu(planet.orbit.semi_major_axis_au));
    expect(out).not.toContain('class="cards');
    // The answer of the other kind of request is not shown.
    const lone = render(PreviewPanel, { props: { preview, kind: 'around', onreroll: () => {} } }).body;
    expect(lone).toContain(t.form.preview.waiting);
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
  const source = previewSource(earthLike);
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
    const world = { ...previewSource(earthLike), shownSystemBody: planetWithMoons! };
    expect(render(MoonsCard, { props: { world } }).body).not.toMatch(/<button[^>]*class="link"/);
    expect(render(MoonsCard, { props: { world: { ...world, preview: false } } }).body).toMatch(/<button[^>]*class="link"/);
  });
});
