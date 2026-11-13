import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { BodySummary } from './api/types/BodySummary';
import type { SystemSummary } from './api/types/SystemSummary';
import {
  NO_FILTERS,
  filterSystems,
  groupSections,
  hasHabitableWorld,
  isHabitableWorld,
  loadView,
  msToMidnight,
  parseView,
  pruneChosen,
  presentClasses,
  sectionOf,
  starClass,
  starLetter,
  strip,
  toggleCompact,
} from './menuCards';
import { saveSetting } from './settings';

const body = (id: string, orbit_au: number, extra: Partial<BodySummary> = {}): BodySummary => ({
  id,
  designation: `MGS 1 ${id}`,
  kind: 'planet',
  body_type: 'terran',
  colour: [0.2, 0.3, 0.5],
  generated: true,
  orbit_au,
  eccentricity: 0,
  periapsis_deg: 0,
  ...extra,
});

const system = (extra: Partial<SystemSummary> = {}): SystemSummary => ({
  id: 'mgs-1-7',
  seed: 7,
  width: 1024,
  height: 512,
  star: 'G2V',
  designation: 'MGS 1',
  star_light: [1, 1, 1],
  hz_inner_au: 0.9,
  hz_outer_au: 1.5,
  planets: {},
  moons: 0,
  bodies: [],
  names: {},
  modified_ms: 0,
  pinned: false,
  outdated: false,
  ...extra,
});

// 2026-11-08 15:00 local.
const NOW = new Date(2026, 10, 8, 15, 0).getTime();
const MIDNIGHT = new Date(2026, 10, 8, 0, 0).getTime();

describe('sections', () => {
  it('pins first, then today since local midnight, then older', () => {
    expect(sectionOf(system({ pinned: true, modified_ms: 0 }), NOW)).toBe('pinned');
    expect(sectionOf(system({ modified_ms: MIDNIGHT }), NOW)).toBe('today');
    expect(sectionOf(system({ modified_ms: MIDNIGHT - 1 }), NOW)).toBe('older');
    expect(sectionOf(system({ modified_ms: NOW + 60_000 }), NOW)).toBe('today');
  });

  it('hides empty sections and keeps the order of the list', () => {
    const a = system({ id: 'a', modified_ms: NOW });
    const b = system({ id: 'b', modified_ms: 1 });
    const c = system({ id: 'c', modified_ms: NOW - 1000 });
    const groups = groupSections([a, b, c], NOW);
    expect(groups.map((g) => g.section)).toEqual(['today', 'older']);
    expect(groups[0]?.items.map((s) => s.id)).toEqual(['a', 'c']);
    expect(groupSections([], NOW)).toEqual([]);
  });
});

describe('star class', () => {
  it('reads the first letter', () => {
    expect(starLetter('K4.5V')).toBe('K');
    expect(starLetter('m1v')).toBe('M');
    expect(starLetter('')).toBeNull();
    expect(starLetter('X')).toBeNull();
    expect(starClass('A5V')).toBe('white');
    expect(starClass('B2V')).toBe('white');
    expect(starClass('F8V')).toBe('F');
    expect(starClass('Q')).toBeNull();
  });

  it('lists only the classes present, in the chips order', () => {
    expect(presentClasses([{ star: 'A5V' }, { star: 'M1V' }, { star: 'M3V' }, { star: 'G2V' }])).toEqual(['M', 'G', 'white']);
    expect(presentClasses([])).toEqual([]);
  });
});

describe('habitable worlds', () => {
  it('needs a terran planet inside the zone, edges included', () => {
    expect(isHabitableWorld(body('a', 1), 0.9, 1.5)).toBe(true);
    expect(isHabitableWorld(body('a', 0.9), 0.9, 1.5)).toBe(true);
    expect(isHabitableWorld(body('a', 1.5), 0.9, 1.5)).toBe(true);
    expect(isHabitableWorld(body('a', 0.8), 0.9, 1.5)).toBe(false);
    expect(isHabitableWorld(body('a', 2), 0.9, 1.5)).toBe(false);
  });

  it('counts the classes that can hold liquid water', () => {
    for (const body_type of ['rocky', 'super-earth', 'ocean-world']) {
      expect(isHabitableWorld(body('a', 1, { body_type }), 0.9, 1.5)).toBe(true);
    }
    for (const body_type of ['ice-world', 'lava-world', 'mini-neptune', 'hot-jupiter']) {
      expect(isHabitableWorld(body('a', 1, { body_type }), 0.9, 1.5)).toBe(false);
    }
  });

  it('does not count dead, frozen, burning or acid rocks, giants or moons', () => {
    for (const body_type of ['barren', 'frozen', 'volcanic', 'caustic', 'gas-giant', 'ice-giant']) {
      expect(isHabitableWorld(body('a', 1, { body_type }), 0.9, 1.5)).toBe(false);
    }
    expect(isHabitableWorld(body('a', 1, { kind: 'moon' }), 0.9, 1.5)).toBe(false);
  });

  it('does not count a moon of a giant in the zone, whatever its orbit', () => {
    const moon = body('m', 1, { kind: 'moon', body_type: 'terran' });
    expect(isHabitableWorld(moon, 0.9, 1.5)).toBe(false);
    expect(hasHabitableWorld(system({ bodies: [body('g', 1, { body_type: 'gas-giant' }), moon] }))).toBe(false);
  });

  it('judges an eccentric planet by its semi-major axis only', () => {
    expect(isHabitableWorld(body('e', 2, { eccentricity: 0.6 }), 0.9, 1.5)).toBe(false);
    expect(isHabitableWorld(body('e', 1.2, { eccentricity: 0.6 }), 0.9, 1.5)).toBe(true);
  });

  it('is never true when the zone is unknown', () => {
    expect(isHabitableWorld(body('a', 0), 0, 0)).toBe(false);
    expect(hasHabitableWorld(system({ hz_inner_au: 0, hz_outer_au: 0, bodies: [body('a', 1)] }))).toBe(false);
  });
});

describe('filters and search', () => {
  const sun = system({ id: 'sun', seed: 1, star: 'G2V', bodies: [body('p1', 1)], names: { star: 'Sol' } });
  const dwarf = system({ id: 'dwarf', seed: 2, star: 'M3V', bodies: [body('p1', 0.1, { body_type: 'barren' })] });
  const dwarfHab = system({ id: 'dwarf-hab', seed: 3, star: 'M5V', hz_inner_au: 0.05, hz_outer_au: 0.1, bodies: [body('p1', 0.07)] });
  const all = [sun, dwarf, dwarfHab];
  const ids = (list: SystemSummary[]) => list.map((s) => s.id);

  it('without filters or query keeps everything', () => {
    expect(ids(filterSystems(all, '', NO_FILTERS))).toEqual(['sun', 'dwarf', 'dwarf-hab']);
  });

  it('filters by class (alternatives) and by habitable world', () => {
    expect(ids(filterSystems(all, '', { classes: ['M'], habitable: false }))).toEqual(['dwarf', 'dwarf-hab']);
    expect(ids(filterSystems(all, '', { classes: ['M', 'G'], habitable: false }))).toHaveLength(3);
    expect(ids(filterSystems(all, '', { classes: [], habitable: true }))).toEqual(['sun', 'dwarf-hab']);
    expect(ids(filterSystems(all, '', { classes: ['M'], habitable: true }))).toEqual(['dwarf-hab']);
  });

  it('composes with the search', () => {
    expect(ids(filterSystems(all, 'sol', NO_FILTERS))).toEqual(['sun']);
    expect(ids(filterSystems(all, 'sol', { classes: ['M'], habitable: false }))).toEqual([]);
    expect(ids(filterSystems(all, ' DWARF ', { classes: [], habitable: true }))).toEqual(['dwarf-hab']);
    expect(ids(filterSystems(all, '3', NO_FILTERS))).toContain('dwarf-hab');
  });
});

describe('selection pruning', () => {
  it('drops the ids no longer shown and keeps the order', () => {
    expect(pruneChosen(['a', 'b', 'c'], ['c', 'a'])).toEqual(['a', 'c']);
    expect(pruneChosen(['a', 'b'], [])).toEqual([]);
  });

  it('returns the same array when nothing is hidden', () => {
    const chosen = ['a', 'b'];
    expect(pruneChosen(chosen, new Set(['a', 'b', 'c']))).toBe(chosen);
  });

  it('follows a filter: a chip that hides the selected systems empties it', () => {
    const m = system({ id: 'm', star: 'M3V' });
    const g = system({ id: 'g', star: 'G2V' });
    const shown = filterSystems([m, g], '', { classes: ['G'], habitable: false });
    expect(pruneChosen(['m', 'g'], shown.map((s) => s.id))).toEqual(['g']);
  });
});

describe('midnight', () => {
  it('counts the ms to the next local midnight', () => {
    expect(msToMidnight(new Date(2026, 10, 8, 23, 0).getTime())).toBe(3_600_000);
    expect(msToMidnight(new Date(2026, 10, 8, 0, 0).getTime())).toBe(24 * 3_600_000);
  });
});

describe('planet strip', () => {
  it('places planets by log distance, giants flagged, inside the margins', () => {
    const s = strip(system({ bodies: [body('a', 0.1), body('b', 1, { body_type: 'gas-giant' }), body('c', 10)] }));
    const at = s.planets.map((p) => p.at);
    expect(at).toEqual([...at].sort((x, y) => x - y));
    expect(at.every((x) => x >= 6 && x <= 94)).toBe(true);
    expect(s.planets.map((p) => p.giant)).toEqual([false, true, false]);
    // Equal ratios, equal gaps.
    expect((at[1] ?? 0) - (at[0] ?? 0)).toBeCloseTo((at[2] ?? 0) - (at[1] ?? 0));
  });

  it('gives the zone as a band, and nothing for a system without orbits', () => {
    const s = strip(system({ bodies: [body('a', 1)] }));
    expect(s.zone?.width).toBeGreaterThan(0);
    expect(s.planets[0]?.at).toBeGreaterThan(s.zone?.from ?? 100);
    expect(strip(system({ bodies: [], hz_outer_au: 0 }))).toEqual({ planets: [], zone: null });
    expect(strip(system({ bodies: [body('m', 1, { kind: 'moon' })], hz_outer_au: 0 })).planets).toEqual([]);
  });
});

describe('view setting', () => {
  beforeEach(() => {
    const data = new Map<string, string>();
    vi.stubGlobal('localStorage', {
      getItem: (k: string) => data.get(k) ?? null,
      setItem: (k: string, v: string) => void data.set(k, v),
    });
  });

  it('falls back to the cards on anything unknown', () => {
    expect(parseView('list')).toBe('list');
    expect(parseView('compact')).toBe('compact');
    expect(parseView('grid')).toBe('cards');
    expect(parseView(undefined)).toBe('cards');
    expect(parseView(3)).toBe('cards');
  });

  it('is read from the systemList setting, next to the sort', () => {
    expect(loadView()).toBe('cards');
    saveSetting('systemList', { sort: 'seed', view: 'list' });
    expect(loadView()).toBe('list');
    saveSetting('systemList', { sort: 'seed', view: 'bogus' });
    expect(loadView()).toBe('cards');
    saveSetting('systemList', { sort: 'seed' });
    expect(loadView()).toBe('cards');
  });

  it('keeps a good view when the stored sort is bad', () => {
    saveSetting('systemList', { sort: 42, view: 'compact' });
    expect(loadView()).toBe('compact');
    saveSetting('systemList', { sort: null, view: 'list' });
    expect(loadView()).toBe('list');
  });

  it('toggles compact from the cards and back', () => {
    expect(toggleCompact('cards')).toBe('compact');
    expect(toggleCompact('compact')).toBe('cards');
  });
});
