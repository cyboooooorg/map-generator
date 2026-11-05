import { describe, expect, it } from 'vitest';
import { bodyApi, parseSystemBody, systemBodyId, worldFile, worldThumbnail } from './client';

describe('world URLs', () => {
  it('name the revision of the world', () => {
    expect(worldFile('terran-1', 'world.png', 42)).toBe('/worlds/terran-1/world.png?rev=42');
    expect(worldThumbnail('mars-like-7', 1730000000000)).toBe('/api/worlds/mars-like-7/thumb?rev=1730000000000');
    expect(worldThumbnail('terran-1')).toBe('/api/worlds/terran-1/thumb');
    expect(worldThumbnail('terran-1', null)).toBe('/api/worlds/terran-1/thumb');
  });

  it('serves the bodies of a system under /systems/ and /api/systems/', () => {
    const id = systemBodyId('mgs-48213-42', 'c-1');
    expect(parseSystemBody(id)).toEqual({ system: 'mgs-48213-42', body: 'c-1' });
    expect(parseSystemBody('terran-1/moons/2')).toBeNull();
    expect(worldFile(id, 'raw_data/world.json', 7)).toBe('/systems/mgs-48213-42/bodies/c-1/raw_data/world.json?rev=7');
    expect(bodyApi(id)).toBe('/api/systems/mgs-48213-42/bodies/c-1');
    expect(bodyApi('terran-1/moons/2')).toBe('/api/worlds/terran-1/moons/2');
  });
});
