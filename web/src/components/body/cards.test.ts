import { render } from 'svelte/server';
import { describe, expect, it } from 'vitest';
import type { Component } from 'svelte';
import type { Body } from '../../lib/api/types/Body';
import type { System } from '../../lib/api/types/System';
import type { WorldMetadata } from '../../lib/api/types/WorldMetadata';
import type { WorldStats } from '../../lib/stats';
import { setLocale } from '../../lib/i18n/index.svelte';
import { LoadedWorld } from '../../lib/world.svelte';
import AtmosphereCard from './AtmosphereCard.svelte';
import BodyHeader from './BodyHeader.svelte';
import InteriorCard from './InteriorCard.svelte';
import LegendCard from './LegendCard.svelte';
import MapCard from './MapCard.svelte';
import MoonsCard from './MoonsCard.svelte';
import OrbitCard from './OrbitCard.svelte';
import SurfaceCard from './SurfaceCard.svelte';

const surface = {
  gases: { n2: 0.78, o2: 0.21, co2: 0.0004, ch4: 0, h2o: 0.01, h2: 0, he: 0, nh3: 0, so2: 0 },
  climate: { temperature_k: 288, liquid: 'water', regulated: true },
  core: { state: 'liquid', dynamo: true, field_ut: 50 },
  life: { arose: true, oxygenated: true },
  regime: 'plates',
  water: { runaway: false, boiled: false, kept_kg: 1, oceans: 1 },
};
const model = {
  surface,
  interior: { layers: [{ material: 'Iron', mass_earth: 0.3 }, { material: 'Rock', mass_earth: 0.7 }] },
  heat: { radiogenic_w: 2e13, tidal_w: 0, secular_w: 2.6e13 },
  ice_shell: null,
};
const body = (over: Record<string, unknown>): Body =>
  ({
    id: 'c',
    designation: 'MGS 1 c',
    kind: 'planet',
    type: 'rocky',
    parent: null,
    orbit: { semi_major_axis_au: 1, semi_major_axis_km: 1.5e8, eccentricity: 0.02, period_days: 365 },
    physical: { radius_km: 6371, mass_earth: 1, density: 5.5, gravity: 1 },
    spin: { period_hours: 24, axial_tilt_deg: 23, locked: false },
    surroundings: { flux_earth: 1, equilibrium_k: 255 },
    preview: { colours: [[0.2, 0.3, 0.5]] },
    params: { age_gyr: 4.5, surface_age_ga: 2 },
    generated: true,
    model,
    ...over,
  }) as unknown as Body;

const biome = (id: number, name: string) => ({ id, name, color: '#123456', surface_color: '#654321' });
const stats = {
  surfaceKm2: 5e8,
  land: 0.3,
  water: 0.7,
  shares: [
    { biome: biome(1, 'Ocean'), share: 0.7, areaKm2: 3.5e8 },
    { biome: biome(2, 'Desert'), share: 0.3, areaKm2: 1.5e8 },
  ],
} as unknown as WorldStats;

const earthMeta = {
  seed: 7,
  class: 'rocky',
  circumference_km: 40000,
  sea_level: 0.5,
  volcanic_intensity: 1,
  erosion: 1,
  gravity_modifier: 1,
  axial_tilt_deg: 23,
  pressure_bar: 1,
  look: { atmosphere: { clouds: 'some' } },
  star: { angular_diameter_deg: 0.53 },
  moons: { moons: [], tide: 1, tilt_steadied: false, binary: false },
  giants: { giants: [], impact_factor: 1 },
} as unknown as WorldMetadata;

function worldOf(bodies: Body[], shown: string, meta: WorldMetadata | null): LoadedWorld {
  const world = new LoadedWorld();
  world.system = { star: { designation: 'MGS 1', type: 'G2V' }, bodies } as unknown as System;
  world.systemId = 'mgs-1';
  world.systemNames = { c: 'Gaia' };
  world.systemBody = shown;
  world.meta = meta;
  world.stats = stats;
  world.id = meta ? 'mgs-1/c' : null;
  return world;
}

const NOOP = () => {};
const renderCard = (card: Component<any>, props: Record<string, unknown>): string => render(card, { props }).body.replace(/<!--.*?-->/g, '');
const noAbbr = (html: string) => html.replace(/<abbr[^>]*>|<\/abbr>/g, '');
const labels = (html: string) => [...noAbbr(html).matchAll(/<dt[^>]*>([^<]*)<\/dt>/g)].map((m) => m[1] ?? '');

/** The labels of every card of a body, together. */
function allLabels(world: LoadedWorld): string[] {
  const map = { world, onVary: NOOP, onReroll: async () => {}, link: () => '', onDelete: NOOP };
  return [OrbitCard, InteriorCard, AtmosphereCard, SurfaceCard, MoonsCard, MapCard].flatMap((card) =>
    labels(renderCard(card, map)),
  );
}

const moon = body({ id: 'c-1', kind: 'moon', type: 'rocky-moon', parent: 'c', model: null, physical: { radius_km: 1700, mass_earth: 0.012, density: 3, gravity: 0.17, bulk: { rock: 0.7, ice: 0.2, carbon: 0.1 } } });
const earthLike = body({});
const giantBody = body({ id: 'd', type: 'gas-giant', model: null, physical: { radius_km: 70000, mass_earth: 318, density: 1.3, gravity: 2.5 } });
const giantMeta = {
  ...earthMeta,
  giant: {
    mass_earth: 318, radius_earth: 11.2, density: 1.3, rotation_hours: 10, hot: false, flattening: 0.06, class: 'ammonia', effective_k: 124,
    jets: 12, wind_west: 100, wind_east: 300, heat_ratio: 1.7, kind: 'gas', great_spot: true, poles: ['hexagon', 'cyclone'], polar_cyclones: [0, 1], rings: null,
  },
} as unknown as WorldMetadata;

// The steam world of #492: a planet under 409 942 bar of steam, drawn
// with a giant's grey cloud deck (`cloud_deck`), no `giant`.
const steamSurface = {
  ...surface,
  gases: { n2: 30, o2: 0, co2: 200, ch4: 0, h2o: 409942, h2: 0, he: 0, nh3: 0, so2: 0 },
  climate: { temperature_k: 1679, liquid: 'none', regulated: false },
  life: { arose: false, oxygenated: false },
  regime: 'stagnant',
  water: { runaway: true, boiled: true, kept_kg: 2e23, oceans: 150 },
};
const steamBody = body({
  type: 'steam-world',
  physical: { radius_km: 10835, mass_earth: 4.78, density: 3.4, gravity: 1.65 },
  model: {
    ...model,
    class: 'steam-world',
    surface: steamSurface,
    interior: { layers: [{ material: 'Iron', mass_earth: 1.06 }, { material: 'Rock', mass_earth: 3.16 }, { material: 'Steam', mass_earth: 0.56 }] },
  },
});
const steamMeta = {
  ...earthMeta,
  class: 'steam-world',
  pressure_bar: 0.3,
  cloud_deck: { ...(giantMeta.giant as object), kind: 'steam', class: 4, effective_k: 40, mass_earth: 4.78, radius_earth: 1.7 },
} as unknown as WorldMetadata;

describe('themed body cards', () => {
  it("shows a steam world's model, not its cloud deck, as a planet's", () => {
    const world = worldOf([steamBody], 'c', steamMeta);
    const air = renderCard(AtmosphereCard, { world });
    // The model's pressure (its gases' sum) and temperature, not the deck's
    // render pressure, clouds or spot.
    expect(noAbbr(air)).toContain('410,172 bar');
    expect(air).toMatch(/<abbr class="gloss"[^>]*title="bar: pressure"/);
    expect(air).not.toContain('Venus');
    expect(noAbbr(air)).not.toContain('0.3 bar');
    expect(noAbbr(air)).toContain('1,406 °C');
    expect(labels(air)).not.toContain('Clouds');
    expect(air).not.toContain('spot');
    // Earth units and its steam layer, no Jupiter units or flattening.
    const interior = renderCard(InteriorCard, { world });
    expect(interior).toContain('steam');
    expect(interior).not.toContain('M♃');
    expect(interior).not.toContain('RJ');
    expect(labels(interior)).not.toContain('Flattening');
  });


  it('shows a label once among the cards of an Earth-like planet, a moon and a giant', () => {
    const cases = [
      worldOf([earthLike, moon], 'c', earthMeta),
      worldOf([earthLike, moon], 'c-1', { ...earthMeta, host: { period_days: 5, angular_diameter_deg: 20, tidal_flux: 3, inside_edge: false } } as unknown as WorldMetadata),
      worldOf([giantBody], 'd', giantMeta),
    ];
    for (const world of cases) {
      const seen = allLabels(world);
      expect(seen.length).toBeGreaterThan(5);
      expect(seen.filter((label, i) => seen.indexOf(label) !== i)).toEqual([]);
    }
  });

  it('gives a moon a button to its host, named, and none to a planet', () => {
    const header = (shown: string) => renderCard(BodyHeader, { world: worldOf([earthLike, moon], shown, null), onRenamed: NOOP, onGenerate: NOOP });
    expect(header('c-1')).toMatch(/<button type="button" class="host[ "][^>]*>↑ Gaia<\/button>/);
    expect(header('c')).not.toMatch(/class="host[ "]/);
  });

  it('keeps the host out of a moon, bar its day and tides', () => {
    const world = worldOf([earthLike, moon], 'c-1', { ...earthMeta, host: { period_days: 5, angular_diameter_deg: 20, tidal_flux: 3, inside_edge: false } } as unknown as WorldMetadata);
    const orbit = renderCard(OrbitCard, { world });
    expect(orbit).toContain('Tidal');
    expect(labels(renderCard(InteriorCard, { world }))).not.toContain('Host');
  });

  it('shows the ocean, the moment of inertia and the geysers of an icy moon', () => {
    const enceladus = body({
      id: 'c-1', kind: 'moon', type: 'ice-moon', parent: 'c',
      model: {
        ...model,
        heat: { radiogenic_w: 1e9, tidal_w: 3e11, secular_w: 0 },
        ice_shell: { shell_km: 25, ocean_km: 30, convecting: false, lid_km: 25, polar_shell_km: 5, liquidus_k: 270 },
        geysers: { south: true, fractures: [] },
        moment_of_inertia: 0.335,
      },
    });
    const html = renderCard(InteriorCard, { world: worldOf([earthLike, enceladus], 'c-1', null) });
    expect(noAbbr(html)).toContain('ocean of 30 km under 25 km of ice (conductive)');
    expect(html).toContain('0.335 — differentiated');
    expect(html).toContain('active at the south pole');
    expect(labels(html)).toContain('Tidal heat');
    // Earth has none of them.
    const earth = renderCard(InteriorCard, { world: worldOf([earthLike], 'c', null) });
    expect(earth).not.toContain('Geysers');
    expect(earth).not.toContain('Moment of inertia');
  });

  it('lists a share and an area per biome in the legend card', () => {
    const html = renderCard(LegendCard, { ramp: null, stats, colour: () => 'red' });
    expect(html).toContain('Ocean');
    expect(html).toContain('Desert');
    expect(html.match(/class="value[ "]/g)).toHaveLength(4);
    expect(html.match(/class="value area[ "]/g)).toHaveLength(2);
  });

  it('shows every biome as a segment of the share bar, sized by its share', () => {
    const html = renderCard(LegendCard, { ramp: null, stats, colour: () => 'red' });
    const bar = html.match(/<div class="bar[ "][^]*?<\/div>/)?.[0] ?? '';
    expect(bar.match(/<span/g)).toHaveLength(stats.shares.length);
    expect(bar).toContain(`flex-grow: ${stats.shares[0]?.share}`);
  });

  it('offers the causes of a derived value only with an explanation, closed', () => {
    const world = worldOf([earthLike, moon], 'c', earthMeta);
    expect(renderCard(AtmosphereCard, { world })).not.toContain('aria-expanded');
    const root = { quantity: 'surface_temperature', value: 288, label: null, input: null, causes: [{ quantity: 'light_received', value: 1, label: null, input: 'insolation', causes: [] }] };
    world.systemExtras = { c: { explanation: { trees: [root] }, similar: null } } as unknown as typeof world.systemExtras;
    const html = renderCard(AtmosphereCard, { world });
    expect(html).toContain('aria-expanded="false"');
    expect(html).not.toContain('Light received');
  });

  it('explains the Bond albedo on the surface card, naming a frost that covers the disc', () => {
    const part = (label: string, value: number) => ({ quantity: 'cover_share', value, label, input: null, causes: [] });
    const io = body({ model: { ...model, surface: { ...surface, climate: { ...surface.climate, bond_albedo: 0.63 } } } });
    const world = worldOf([io, moon], 'c', earthMeta);
    for (const locale of ['en', 'fr'] as const) {
      setLocale(locale);
      world.systemExtras = {} as typeof world.systemExtras;
      const dt = locale === 'en' ? 'Bond albedo' : 'Albédo de Bond';
      const bare = renderCard(SurfaceCard, { world });
      expect(labels(bare), locale).toContain(dt);
      expect(bare).toContain(locale === 'en' ? '0.63' : '0,63');
      expect(bare).not.toContain('aria-expanded');
      const tree = { quantity: 'bond_albedo', value: 0.63, label: null, input: null, causes: [part('frost_so2', 0.97), part('cloud', 0.02)] };
      world.systemExtras = { c: { explanation: { trees: [tree] }, similar: null } } as unknown as typeof world.systemExtras;
      const html = renderCard(SurfaceCard, { world });
      expect(html).toContain('aria-expanded="false"');
      expect(html).toContain(locale === 'en' ? '0.63 · volcanic SO₂ frost' : '0,63 · givre de SO₂ volcanique');
      expect(html).not.toMatch(/frost_so2|cover_share/);
    }
    setLocale('en');
  });
});
