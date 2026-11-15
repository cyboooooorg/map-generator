import { render } from 'svelte/server';
import { describe, expect, it } from 'vitest';
import type { Component } from 'svelte';
import type { Cause } from '../../lib/api/types/Cause';
import type { InputField } from '../../lib/api/types/InputField';
import type { ResolvedSystem } from '../../lib/api/types/ResolvedSystem';
import { previewSource } from '../../lib/cardSource';
import { formatSurfaceTemperature } from '../../lib/format';
import AtmosphereCard from './AtmosphereCard.svelte';
import BodyHeader from './BodyHeader.svelte';
import CauseTree from './CauseTree.svelte';
import InteriorCard from './InteriorCard.svelte';
import MoonsCard from './MoonsCard.svelte';
import OrbitCard from './OrbitCard.svelte';
import SurfaceCard from './SurfaceCard.svelte';
import earth from './fixtures/earth-like.json';
import jupiter from './fixtures/hot-jupiter.json';
import neptune from './fixtures/mini-neptune.json';

const earthLike = earth as unknown as ResolvedSystem;
const hotJupiter = jupiter as unknown as ResolvedSystem;
const miniNeptune = neptune as unknown as ResolvedSystem;

const cards: [string, Component<any>][] = [
  ['header', BodyHeader],
  ['orbit', OrbitCard],
  ['interior', InteriorCard],
  ['atmosphere', AtmosphereCard],
  ['surface', SurfaceCard],
  ['moons', MoonsCard],
];
const html = (card: Component<any>, result: ResolvedSystem) =>
  render(card, { props: { world: previewSource(result), onRenamed: () => {}, onGenerate: () => {} } }).body;
const count = (text: string, part: string) => text.split(part).length - 1;

describe('cards from a resolved body', () => {
  it('previewSource is the focus body with its extras and no map files', () => {
    const source = previewSource(earthLike);
    expect(source.shownSystemBody?.id).toBe(earthLike.system.focus);
    expect(source.shownExtras).toBe(earthLike.extras[earthLike.system.focus!]);
    expect([source.meta, source.stats, source.rivers]).toEqual([null, null, null]);
  });

  for (const [name, result] of [
    ['an Earth-like', earthLike],
    ['a hot Jupiter', hotJupiter],
    ['a mini-Neptune', miniNeptune],
  ] as const) {
    it(`renders all six cards for ${name} without map metadata`, () => {
      for (const [, card] of cards) expect(() => html(card, result)).not.toThrow();
    });
  }

  it('shows the surface temperature of an Earth-like once, in the atmosphere card', () => {
    const model = previewSource(earthLike).shownSystemBody!.model!;
    const temperature = formatSurfaceTemperature(model)!;
    const out = cards.map(([, card]) => html(card, earthLike));
    expect(out.map((o) => count(o, temperature))).toEqual([0, 0, 0, 1, 0, 0]);
    expect(out[3]).toContain('Why?');
  });

  it('has no surface tree for a giant', () => {
    for (const result of [hotJupiter, miniNeptune]) {
      const out = html(AtmosphereCard, result);
      expect(out).not.toContain('Surface temperature');
      expect(out).not.toContain('Why?');
    }
  });
});

describe('input leaves', () => {
  const node = (over: Partial<Cause>): Cause => ({ quantity: 'pressure', value: 1, label: null, input: null, causes: [], ...over });
  const tree = node({ quantity: 'surface_temperature', value: 288, causes: [node({ quantity: 'light_received', value: 1, input: 'insolation' })] });
  const out = (oninput?: ((field: InputField) => void) | undefined) => render(CauseTree, { props: { tree, open: true, ontoggle: () => {}, oninput } }).body;

  it('is a button when oninput is set', () => {
    const set = out(() => {});
    expect(set).toMatch(/<button[^>]*class="[^"]*\binput\b[^"]*"[^>]*>[^<]*Light received/);
  });
  it('is plain text otherwise', () => {
    const unset = out();
    expect(unset).toMatch(/<span[^>]*class="[^"]*\binput\b[^"]*"/);
    expect(unset.match(/<button/g)?.length).toBe(1);
  });
});
