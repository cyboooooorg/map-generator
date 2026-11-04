import { describe, expect, it } from 'vitest';
import { bodyPath } from './api/client';
import { bodyFileName, bodyId, bodyKey, moonBody, moonChoiceOf, moonsOf, moonsRequest, parseBody, parseBodyKey, parseMoonChoice, systemBodies } from './moons';
import type { WorldMetadata } from './api/types/WorldMetadata';

describe('moons', () => {
  it('names a moon as a body of its world', () => {
    expect(moonBody('terran-42', 2)).toBe('terran-42/moons/2');
    expect(parseBody('terran-42/moons/2')).toEqual({ world: 'terran-42', moon: 2 });
    expect(parseBody('terran-42')).toEqual({ world: 'terran-42', moon: null });
    // Each segment encoded on its own: the server routes the moon's path.
    expect(bodyPath('my world-1/moons/1')).toBe('my%20world-1/moons/1');
    expect(bodyFileName('terran-42/moons/2')).toBe('terran-42-moons-2');
  });

  it('lists no moons for a world made before them', () => {
    expect(moonsOf(null)).toEqual([]);
    expect(moonsOf({} as WorldMetadata)).toEqual([]);
  });

  it('turns the form choice into a request and back', () => {
    expect(moonsRequest('auto')).toBeUndefined();
    expect(moonsRequest('none')).toBe('none');
    expect(moonsRequest(3)).toEqual({ count: 3 });
    expect(moonChoiceOf(undefined)).toBe('auto');
    expect(moonChoiceOf('none')).toBe('none');
    expect(moonChoiceOf({ count: 2 })).toBe(2);
    expect(parseMoonChoice(4, 6)).toBe(4);
    expect(parseMoonChoice(0, 6)).toBe('auto');
  });

  it('names the bodies of a system and lists them in its order', () => {
    expect(bodyId('terran-5', { kind: 'world' })).toBe('terran-5');
    expect(bodyId('terran-5', { kind: 'moon', n: 2 })).toBe('terran-5/moons/2');
    expect(bodyId('terran-5', { kind: 'host' })).toBe('terran-5/host');
    expect(bodyId('terran-5', { kind: 'sibling', n: 1 })).toBe('terran-5/host/siblings/1');
    for (const key of ['world', 'host', 'moon:3', 'sibling:1']) {
      expect(bodyKey(parseBodyKey(key)!)).toBe(key);
    }
    expect(parseBodyKey('moon:0')).toBeNull();
    expect(parseBodyKey('star')).toBeNull();
    // A giant's moon: the giant, then its moons by distance, the world among them.
    const exomoon = { host: { distance_km: 600_000, siblings: [{ distance_km: 1_000_000 }, { distance_km: 400_000 }] } } as unknown as WorldMetadata;
    expect(systemBodies(exomoon).map(bodyKey)).toEqual(['host', 'sibling:2', 'world', 'sibling:1']);
    const planet = { moons: { moons: [{}, {}] } } as unknown as WorldMetadata;
    expect(systemBodies(planet).map(bodyKey)).toEqual(['world', 'moon:1', 'moon:2']);
    expect(systemBodies(null).map(bodyKey)).toEqual(['world']);
  });
});
