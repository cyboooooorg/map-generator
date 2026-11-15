import { describe, expect, it } from 'vitest';
import type { ArchetypeInfo } from './api/types/ArchetypeInfo';
import { ageSources, archetypeAge } from './archetypeAge';

const make = (name: string, age?: number | [number, number] | null): ArchetypeInfo =>
  ({ spec: name, name, description: '', locales: {}, inputs: { mass_earth: 1, ...(age === undefined ? {} : { age_gyr: age }) } }) as unknown as ArchetypeInfo;

const catalog = [make('mars-like', 4.57), make('titan-like', 4.57), make('hot-jupiter'), make('ranged', [3, 5])];

describe('archetypeAge', () => {
  it('is the fixed age', () => expect(archetypeAge(catalog[0]!)).toBe(4.57));
  it('is the midpoint of a range', () => expect(archetypeAge(catalog[3]!)).toBe(4));
  it('is null without an age', () => {
    expect(archetypeAge(catalog[2]!)).toBeNull();
    expect(archetypeAge(make('x', null))).toBeNull();
  });
});

describe('ageSources', () => {
  it('names each asked archetype that sets an age, once, in name order', () => {
    expect(ageSources(catalog, { 'mars-like': 1, 'titan-like': 1, 'hot-jupiter': 2 })).toEqual(['mars-like', 'titan-like']);
  });
  it('is empty when none is asked', () => expect(ageSources(catalog, { 'mars-like': 0 })).toEqual([]));
});
