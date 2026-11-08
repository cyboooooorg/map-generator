import { describe, expect, it } from 'vitest';
import type { SystemSummary } from './api/types/SystemSummary';
import { sortSystems, systemName } from './systemList';

const system = (id: string, seed: number, modified_ms: number, pinned = false, planets = 0): SystemSummary => ({
  id,
  seed,
  width: 1024,
  height: 512,
  star: 'G2V',
  designation: `MGS ${seed}`,
  star_light: [1, 1, 1],
  hz_inner_au: 0.9,
  hz_outer_au: 1.4,
  planets: {},
  moons: 0,
  bodies: Array.from({ length: planets }, (_, i) => ({
    id: `p${i}`,
    designation: `MGS ${seed} p${i}`,
    kind: 'planet',
    body_type: 'terran',
    colour: [0.5, 0.5, 0.5],
    generated: false,
    orbit_au: 1 + i,
    eccentricity: 0,
    periapsis_deg: 0,
  })),
  names: {},
  modified_ms,
  pinned,
});

describe('sortSystems', () => {
  const a = system('a', 3, 100);
  const b = system('b', 1, 300, false, 2);
  const c = system('c', 2, 200, true);
  const d = system('d', 4, 50, true, 1);
  const ids = (list: SystemSummary[]) => list.map((s) => s.id);

  it('lists the pinned systems first, each group in the chosen order', () => {
    expect(ids(sortSystems([a, b, c, d], 'newest'))).toEqual(['c', 'd', 'b', 'a']);
    expect(ids(sortSystems([a, b, c, d], 'oldest'))).toEqual(['d', 'c', 'a', 'b']);
    expect(ids(sortSystems([a, b, c, d], 'seed'))).toEqual(['c', 'd', 'b', 'a']);
    expect(ids(sortSystems([a, b, c, d], 'planets'))).toEqual(['d', 'c', 'b', 'a']);
  });

  it('is the plain order when nothing is pinned, and leaves its input alone', () => {
    const input = [a, b];
    expect(ids(sortSystems(input, 'newest'))).toEqual(['b', 'a']);
    expect(ids(input)).toEqual(['a', 'b']);
  });
});

describe('system names', () => {
  it('shows the name given to the star, else the designation', () => {
    expect(systemName({ names: {}, designation: 'MGS 1' })).toBe('MGS 1');
    expect(systemName({ names: { star: 'Helia' }, designation: 'MGS 1' })).toBe('Helia');
  });
});
