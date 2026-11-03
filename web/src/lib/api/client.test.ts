import { describe, expect, it } from 'vitest';
import { worldFile, worldThumbnail } from './client';

describe('world URLs', () => {
  it('name the revision of the world', () => {
    expect(worldFile('terran-1', 'world.png', 42)).toBe('/worlds/terran-1/world.png?rev=42');
    expect(worldThumbnail('mars-like-7', 1730000000000)).toBe('/api/worlds/mars-like-7/thumb?rev=1730000000000');
    expect(worldThumbnail('terran-1')).toBe('/api/worlds/terran-1/thumb');
    expect(worldThumbnail('terran-1', null)).toBe('/api/worlds/terran-1/thumb');
  });
});
