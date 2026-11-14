import { describe, expect, it } from 'vitest';
import { CARD_GAP, TOP_MARGIN, atmosphereLine, bulkLine, cardBox, extremes, layoutCards, linkEnd, oceanUnderIce, shownCards, zonePlace } from './planetCard';
import type { PlanetModel } from './api/types/PlanetModel';
import { setLocale } from './i18n/index.svelte';

describe('zonePlace', () => {
  it('places an orbit inside, short of or beyond the habitable zone', () => {
    expect(zonePlace(1, 0.95, 1.4)).toEqual({ inside: true });
    expect(zonePlace(0.7, 0.95, 1.4)).toEqual({ inside: false, side: 'inner', gapAu: 0.95 - 0.7 });
    expect(zonePlace(5.2, 0.95, 1.4)).toEqual({ inside: false, side: 'outer', gapAu: 5.2 - 1.4 });
  });
});

describe('cardBox', () => {
  it('puts the card right of the planet, centred on it', () => {
    expect(cardBox(100, 300, 200, 120, 1000, 800)).toEqual({ left: 100 + CARD_GAP, top: 240, side: 'right' });
  });

  it('puts the card left of a planet near the right edge', () => {
    const box = cardBox(900, 300, 200, 120, 1000, 800);
    expect(box.side).toBe('left');
    expect(box.left).toBe(900 - CARD_GAP - 200);
  });

  it('keeps the card inside the view', () => {
    const box = cardBox(50, 5, 200, 120, 1000, 800);
    expect(box.top).toBe(TOP_MARGIN);
    const low = cardBox(50, 795, 200, 120, 1000, 800);
    expect(low.top).toBe(800 - 120 - 8);
    // Too narrow for either side: right, pushed back in.
    const narrow = cardBox(150, 300, 200, 120, 300, 800);
    expect(narrow.side).toBe('right');
    expect(narrow.left).toBe(300 - 200 - 8);
  });
});

describe('linkEnd', () => {
  it('ends on the side facing the planet, at its height when it can', () => {
    expect(linkEnd({ left: 128, top: 240, side: 'right' }, 200, 120, 300)).toEqual({ x: 128, y: 300 });
    expect(linkEnd({ left: 672, top: 240, side: 'left' }, 200, 120, 300)).toEqual({ x: 872, y: 300 });
    expect(linkEnd({ left: 128, top: 56, side: 'right' }, 200, 120, 0)).toEqual({ x: 128, y: 62 });
  });
});

describe('shownCards', () => {
  const ids = ['b', 'c', 'd'];

  it('shows the pinned cards in pinning order, then the hovered one', () => {
    expect(shownCards(ids, ['d', 'b'], 1)).toEqual([
      { index: 2, pinned: true },
      { index: 0, pinned: true },
      { index: 1, pinned: false },
    ]);
  });

  it('shows a hovered pinned planet once, and skips ids the system lacks', () => {
    expect(shownCards(ids, ['c', 'x'], 1)).toEqual([{ index: 1, pinned: true }]);
    expect(shownCards(ids, [], null)).toEqual([]);
  });
});

describe('layoutCards', () => {
  const overlap = (a: { left: number; top: number }, b: { left: number; top: number }, w: number, h: number) =>
    a.left < b.left + w && b.left < a.left + w && a.top < b.top + h && b.top < a.top + h;

  it('places a lone card as cardBox does', () => {
    expect(layoutCards([{ x: 100, y: 300, width: 200, height: 120 }], 1000, 800)).toEqual([cardBox(100, 300, 200, 120, 1000, 800)]);
  });

  it('keeps the first card in place and moves the next one off it', () => {
    const [a, b] = layoutCards(
      [
        { x: 100, y: 300, width: 200, height: 120 },
        { x: 110, y: 310, width: 200, height: 120 },
      ],
      1000,
      800,
    );
    expect(a).toEqual(cardBox(100, 300, 200, 120, 1000, 800));
    expect(overlap(a!, b!, 200, 120)).toBe(false);
  });

  it('tries the other side of the planet before moving a card up or down', () => {
    const [, b] = layoutCards(
      [
        { x: 500, y: 300, width: 200, height: 120 },
        { x: 500, y: 300, width: 200, height: 120 },
      ],
      1000,
      800,
    );
    expect(b).toEqual({ left: 500 - CARD_GAP - 200, top: 240, side: 'left' });
  });
});

describe('extremes', () => {
  const row = (radius: number, moons: number) => ({
    radius,
    mass: 1,
    orbit: 1,
    speed: 30,
    year: 365,
    day: 24,
    light: 1,
    gravity: 1,
    equilibrium: 255,
    moons,
  });

  it('marks nothing on a lone card', () => {
    expect(extremes([row(1, 1)])).toEqual([{}]);
  });

  it('marks the highest and lowest of each line, every card that holds it, and skips equal lines', () => {
    expect(extremes([row(1, 2), row(11, 0), row(1, 2)])).toEqual([
      { radius: 'min', moons: 'max' },
      { radius: 'max', moons: 'min' },
      { radius: 'min', moons: 'max' },
    ]);
  });
});

describe('a body card of the planet model', () => {
  const model = (ocean_km: number | null, pressure = 1.5) =>
    ({
      surface: { gases: { n2: pressure * 0.94, co2: 0, h2o: 0, ch4: pressure * 0.05, nh3: 0, so2: 0, o2: 0 } },
      ice_shell: ocean_km === null ? null : { shell_km: 60, ocean_km },
    }) as unknown as PlanetModel;

  it('shows the ocean under the ice', () => {
    setLocale('en');
    expect(oceanUnderIce(model(120))).toBe('ocean under 60 km of ice');
    expect(oceanUnderIce(model(0))).toBeNull();
    expect(oceanUnderIce(model(null))).toBeNull();
  });

  it('shows the surface pressure and the main gases', () => {
    setLocale('en');
    expect(atmosphereLine(model(null))).toBe('1.48 bar · N₂ 95 % · CH₄ 5 %');
    expect(atmosphereLine(model(null, 0))).toBeNull();
  });

  it('shows a small moons bulk', () => {
    setLocale('en');
    expect(bulkLine({ rock: 0.6, ice: 0.3, carbon: 0.1 })).toBe('rock 60 % · ice 30 % · carbon 10 %');
    setLocale('fr');
    expect(bulkLine({ rock: 0.6, ice: 0.3, carbon: 0.1 })).toContain('roche');
    setLocale('en');
  });
});
