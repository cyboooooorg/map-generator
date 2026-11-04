import { describe, expect, it } from 'vitest';
import { bodyPath } from './api/client';
import { bodyFileName, moonBody, moonChoiceOf, moonsOf, moonsRequest, parseBody, parseMoonChoice } from './moons';
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
});
