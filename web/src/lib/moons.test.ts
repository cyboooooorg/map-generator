import { describe, expect, it } from 'vitest';
import { bodyPath } from './api/client';
import { bodyFileName, moonBody, moonsOf, parseBody } from './moons';
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
});
