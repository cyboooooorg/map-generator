import { describe, expect, it } from 'vitest';
import type { GeneratorNote } from './api/types/GeneratorNote';
import type { System } from './api/types/System';
import type { RegeneratedBody } from './api/types/RegeneratedBody';
import { bodyTitle, evictedKey, forgetTold, toTell, toTellEvicted } from './evicted';

const note: GeneratorNote = { version: '0.1.0+aaaa', format: 3, current_version: '0.1.0+bbbb', current_format: 3 };

describe('evicted bodies', () => {
  it('tells the user of each older world once per body', () => {
    const told = new Set<string>();
    const of = (...bodies: string[]): RegeneratedBody[] => bodies.map((body) => ({ body, from: note }));
    expect(toTell('mgs-1', [], told)).toEqual([]);
    // A planet's job makes its evicted moons again: each is named.
    expect(toTell('mgs-1', of('c', 'c-1'), told)).toEqual(['c', 'c-1']);
    expect(toTell('mgs-1', of('c', 'c-1'), told)).toEqual([]);
    expect(toTell('mgs-1', of('c-1', 'c-2'), told)).toEqual(['c-2']);
    expect(toTell('mgs-2', of('c'), told)).toEqual(['c']);
  });

  it('forgets what a system was told, and only that', () => {
    const told = new Set([evictedKey('mgs-1', 'c'), evictedKey('mgs-1', 'c-1'), evictedKey('mgs-2', 'c')]);
    forgetTold('mgs-1', told);
    expect([...told]).toEqual(['mgs-2/c']);
    expect(toTell('mgs-1', [{ body: 'c', from: note }], told)).toEqual(['c']);
  });

  it('names a body by its name, else its designation, else its id', () => {
    const system = { bodies: [{ id: 'c', designation: 'MGS 48213 c' }] } as unknown as System;
    expect(bodyTitle(system, { c: 'Gaia' }, 'c')).toBe('Gaia');
    expect(bodyTitle(system, {}, 'c')).toBe('MGS 48213 c');
    expect(bodyTitle(system, {}, 'zz')).toBe('zz');
    expect(bodyTitle(null, {}, 'c')).toBe('c');
  });
});

describe('the notice of an evicted body made again', () => {
  it('tells once per opening of a body, and again after the system was generated', () => {
    const told = new Set<string>();
    expect(toTellEvicted('mgs-1', 'c-1', told)).toBe(true);
    expect(toTellEvicted('mgs-1', 'c-1', told)).toBe(false);
    expect(toTellEvicted('mgs-1', 'c', told)).toBe(true);
    expect(toTellEvicted('mgs-2', 'c-1', told)).toBe(true);
    forgetTold('mgs-1', new Set(), told);
    expect([...told]).toEqual(['mgs-2/c-1']);
    expect(toTellEvicted('mgs-1', 'c-1', told)).toBe(true);
  });
});
