import { describe, expect, it } from 'vitest';
import type { WorldNames } from './api/types/WorldNames';
import { designationBeside, givenNames, moonName, worldName } from './names';

const names: WorldNames = {
  star: 'MGS 48213',
  designation: 'MGS 48213 c',
  name: null,
  giants: ['MGS 48213 b'],
  moons: [
    { designation: 'MGS 48213 c I', name: 'Séléné' },
    { designation: 'MGS 48213 c II', name: null },
  ],
  host: null,
  siblings: [],
};

describe('names', () => {
  it('prefers a given name to the designation', () => {
    expect(worldName(names)).toBe('MGS 48213 c');
    expect(worldName({ ...names, name: 'Gaia' })).toBe('Gaia');
    expect(moonName(names, 1)).toBe('Séléné');
    expect(moonName(names, 2)).toBe('MGS 48213 c II');
    expect(moonName(names, 3)).toBeNull();
  });

  it('shows the designation beside a given name only', () => {
    expect(designationBeside('Gaia', 'MGS 48213 c')).toBe('MGS 48213 c');
    expect(designationBeside('MGS 48213 c', 'MGS 48213 c')).toBeNull();
    expect(designationBeside(null, 'MGS 48213 c')).toBeNull();
  });

  it('sends the edited names, trimmed, without the empty ones or designations', () => {
    expect(givenNames(names, '  Gaia ', ['', 'MGS 48213 c II'])).toEqual({ name: 'Gaia', moons: {} });
    expect(givenNames(names, 'MGS 48213 c', ['Séléné', ' Nyx'])).toEqual({ moons: { '1': 'Séléné', '2': 'Nyx' } });
  });
});
