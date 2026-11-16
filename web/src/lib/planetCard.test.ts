import { describe, expect, it } from 'vitest';
import { CARD_GAP, TOP_MARGIN, atmosphereLine, bulkLine, cardBox, extremes, layoutCards, geysersLine, linkEnd, momentOfInertiaLine, oceanUnderIce, tidalHeatLine, shownCards, zonePlace } from './planetCard';
import type { Explanation } from './api/types/Explanation';
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
  const model = (ocean_km: number | null, pressure = 1.5, convecting = false) =>
    ({
      surface: { gases: { n2: pressure * 0.94, co2: 0, h2o: 0, ch4: pressure * 0.05, nh3: 0, so2: 0, o2: 0 } },
      ice_shell: ocean_km === null ? null : { shell_km: 60, ocean_km, convecting },
    }) as unknown as PlanetModel;

  it('shows the ocean under the ice', () => {
    setLocale('en');
    expect(oceanUnderIce(model(120))).toBe('ocean of 120 km under 60 km of ice (conductive)');
    expect(oceanUnderIce(model(120, 1.5, true))).toBe('ocean of 120 km under 60 km of ice (convective)');
    expect(oceanUnderIce(model(0))).toBeNull();
    expect(oceanUnderIce(model(null))).toBeNull();
  });

  it('words the ocean line in French', () => {
    setLocale('fr');
    expect(oceanUnderIce(model(120))).toMatch(/^océan de 120.km sous 60.km de glace \(conductive\)$/);
    setLocale('en');
  });

  it('shows the tidal heat, and the resonance when it sets it', () => {
    setLocale('en');
    const heat = { heat: { tidal_w: 3e12 } } as unknown as PlanetModel;
    const resonance = { trees: [{ quantity: 'heat_flux', value: 0, label: null, input: null, causes: [{ quantity: 'tidal_heat', value: 3e12, label: null, input: null, causes: [{ quantity: 'resonant_heat', value: 3e12, label: null, input: null, causes: [] }] }] }] } as Explanation;
    expect(tidalHeatLine(heat, { trees: [] })).toMatch(/^3\s?TW$/);
    expect(tidalHeatLine(heat, resonance)).toMatch(/^3\s?TW \(2:1 resonance\)$/);
    expect(tidalHeatLine({ heat: { tidal_w: 0 } } as unknown as PlanetModel, null)).toBeNull();
    setLocale('fr');
    expect(tidalHeatLine(heat, resonance)).toContain('résonance 2:1');
    setLocale('en');
  });

  it('writes the tidal heat with an SI prefix', () => {
    setLocale('en');
    const w = (tidal_w: number) => ({ heat: { tidal_w } }) as unknown as PlanetModel;
    expect(tidalHeatLine(w(1.4e10), null)).toMatch(/^14\s?GW$/);
    expect(tidalHeatLine(w(9.5e7), null)).toMatch(/^95\s?MW$/);
    expect(tidalHeatLine(w(1.05e14), null)).toMatch(/^105\s?TW$/);
  });

  it('shows a near-zero shell as under 0.1 km', () => {
    setLocale('en');
    const m = { ice_shell: { shell_km: 0.03, ocean_km: 5, convecting: false } } as unknown as PlanetModel;
    expect(oceanUnderIce(m)).toBe('ocean of 5 km under < 0.1 km of ice (conductive)');
    setLocale('fr');
    expect(oceanUnderIce(m)).toMatch(/sous < 0,1.km de glace/);
    setLocale('en');
  });

  it('shows the moment of inertia of a moon or an ice-shelled body only', () => {
    setLocale('en');
    const m = (c: number, shell: boolean, mixed = c > 0.33) =>
      ({
        moment_of_inertia: c,
        ice_shell: shell ? { shell_km: 1, ocean_km: 1 } : null,
        interior: { layers: [{ material: 'Rock' }, ...(mixed ? [{ material: 'RockIce' }] : [])] },
      }) as unknown as PlanetModel;
    expect(momentOfInertiaLine(m(0.31, true), false)).toBe('0.31 — differentiated');
    expect(momentOfInertiaLine(m(0.355, false), true)).toBe('0.355 — partially differentiated');
    // Io: C/MR² about 0.35 yet no rock-ice mix left, so it is differentiated.
    expect(momentOfInertiaLine(m(0.35, false, false), true)).toBe('0.35 — differentiated');
    // Europa: rock below, the ice shell is its own layer.
    expect(momentOfInertiaLine(m(0.346, true, false), true)).toBe('0.346 — differentiated');
    expect(momentOfInertiaLine(m(0.354, false), false)).toBeNull();
    expect(momentOfInertiaLine(m(0, true), true)).toBeNull();
    setLocale('fr');
    expect(momentOfInertiaLine(m(0.31, true), false)).toBe('0,31 — différencié');
    setLocale('en');
  });

  it('shows the geysers with their pole, only when there are some', () => {
    setLocale('en');
    const g = (south: boolean) => ({ geysers: { south, fractures: [] } }) as unknown as PlanetModel;
    expect(geysersLine(g(true))).toBe('active at the south pole');
    expect(geysersLine(g(false))).toBe('active at the north pole');
    expect(geysersLine({ geysers: null } as unknown as PlanetModel)).toBeNull();
    setLocale('fr');
    expect(geysersLine(g(true))).toBe('actifs au pôle sud');
    setLocale('en');
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
