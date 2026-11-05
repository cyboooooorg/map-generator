import { describe, expect, it } from 'vitest';
import { bodyApi, parseSystemBody, systemBodyId, worldFile, worldThumbnail } from './client';

describe('world URLs', () => {
  it('name the revision of the world', () => {
    const id = systemBodyId('mgs-1-7', 'b');
    expect(worldFile(id, 'world.png', 42)).toBe('/systems/mgs-1-7/bodies/b/world.png?rev=42');
    expect(worldThumbnail(id, 1730000000000)).toBe('/api/systems/mgs-1-7/bodies/b/thumb?rev=1730000000000');
    expect(worldThumbnail(id)).toBe('/api/systems/mgs-1-7/bodies/b/thumb');
    expect(worldThumbnail(id, null)).toBe('/api/systems/mgs-1-7/bodies/b/thumb');
  });

  it('serves the bodies of a system under /systems/ and /api/systems/', () => {
    const id = systemBodyId('mgs-48213-42', 'c-1');
    expect(parseSystemBody(id)).toEqual({ system: 'mgs-48213-42', body: 'c-1' });
    expect(parseSystemBody('terran-1/moons/2')).toBeNull();
    expect(worldFile(id, 'raw_data/world.json', 7)).toBe('/systems/mgs-48213-42/bodies/c-1/raw_data/world.json?rev=7');
    expect(bodyApi(id)).toBe('/api/systems/mgs-48213-42/bodies/c-1');
  });
});
