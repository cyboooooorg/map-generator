import { describe, expect, it } from 'vitest';
import type { GeneratorNote } from './api/types/GeneratorNote';
import type { System } from './api/types/System';
import { bodyTitle, evictedKey, forgetTold, shouldTell } from './evicted';

const note: GeneratorNote = { version: '0.1.0+aaaa', format: 3, current_version: '0.1.0+bbbb', current_format: 3 };

describe('evicted bodies', () => {
  it('tells the user of an older world once per body', () => {
    const told = new Set<string>();
    expect(shouldTell(null, 'mgs-1/c', told)).toBe(false);
    expect(shouldTell(note, 'mgs-1/c', told)).toBe(true);
    expect(shouldTell(note, 'mgs-1/c', told)).toBe(false);
    expect(shouldTell(note, 'mgs-1/c-1', told)).toBe(true);
  });

  it('forgets what a system was told, and only that', () => {
    const told = new Set([evictedKey('mgs-1', 'c'), evictedKey('mgs-1', 'c-1'), evictedKey('mgs-2', 'c')]);
    forgetTold('mgs-1', told);
    expect([...told]).toEqual(['mgs-2/c']);
    expect(shouldTell(note, 'mgs-1/c', told)).toBe(true);
  });

  it('names a body by its name, else its designation, else its id', () => {
    const system = { bodies: [{ id: 'c', designation: 'MGS 48213 c' }] } as unknown as System;
    expect(bodyTitle(system, { c: 'Gaia' }, 'c')).toBe('Gaia');
    expect(bodyTitle(system, {}, 'c')).toBe('MGS 48213 c');
    expect(bodyTitle(system, {}, 'zz')).toBe('zz');
    expect(bodyTitle(null, {}, 'c')).toBe('c');
  });
});
