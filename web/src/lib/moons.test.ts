import { describe, expect, it } from 'vitest';
import { bodyPath } from './api/client';
import { bodyFileName, moonChoiceOf, moonsOf, moonsRequest, parseMoonChoice, worldCardOwner } from './moons';
import type { WorldMetadata } from './api/types/WorldMetadata';

describe('moons', () => {
  it('names a body of a system in URLs and file names', () => {
    // Each segment encoded on its own: the server routes the body's path.
    expect(bodyPath('systems/my sys-1/bodies/c-1')).toBe('systems/my%20sys-1/bodies/c-1');
    expect(bodyFileName('systems/mgs-1-2/bodies/c-1')).toBe('systems-mgs-1-2-bodies-c-1');
  });

  it('lists no moons for a world without them', () => {
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

describe('the world card of a body shown', () => {
  const bodies = [
    { id: 'f', designation: 'MGS 77749 f', parent: null },
    { id: 'f-2', designation: 'MGS 77749 f II', parent: 'f' },
  ];
  it('names a planet with its own name', () => {
    expect(worldCardOwner(bodies, { f: 'Gaia' }, 'f')).toEqual({ planet: 'Gaia', ofMoon: false });
  });
  it('names the planet, not the moon, for a moon: the card holds the planet’s data', () => {
    expect(worldCardOwner(bodies, {}, 'f-2')).toEqual({ planet: 'MGS 77749 f', ofMoon: true });
    expect(worldCardOwner(bodies, { f: 'Gaia', 'f-2': 'Luna' }, 'f-2')).toEqual({ planet: 'Gaia', ofMoon: true });
  });
  it('has no owner when no body is shown', () => {
    expect(worldCardOwner(bodies, {}, null)).toBeNull();
  });
});
