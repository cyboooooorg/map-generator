import { describe, expect, it } from 'vitest';
import { bodyApi, bodySvg, parseSystemBody, sizeName, systemBodyId, worldFile, worldThumbnail } from './client';

describe('SVG export', () => {
  it('is drawn on request by the server, at the size asked', () => {
    const id = systemBodyId('mgs-1-7', 'b');
    expect(bodySvg(id, '2048x1024')).toBe('/api/systems/mgs-1-7/bodies/b/world.svg?size=2048x1024');
  });
});

describe('world URLs', () => {
  it('name the revision of the world', () => {
    const id = systemBodyId('mgs-1-7', 'b');
    expect(worldFile(id, '2048x1024', 'world.png', 42)).toBe('/systems/mgs-1-7/bodies/b/sizes/2048x1024/world.png?rev=42');
    expect(worldThumbnail(id, 1730000000000)).toBe('/api/systems/mgs-1-7/bodies/b/thumb?rev=1730000000000');
    expect(worldThumbnail(id)).toBe('/api/systems/mgs-1-7/bodies/b/thumb');
    expect(worldThumbnail(id, null)).toBe('/api/systems/mgs-1-7/bodies/b/thumb');
  });

  it('serves the bodies of a system under /systems/ and /api/systems/', () => {
    const id = systemBodyId('mgs-48213-42', 'c-1');
    expect(parseSystemBody(id)).toEqual({ system: 'mgs-48213-42', body: 'c-1' });
    expect(parseSystemBody('terran-1/moons/2')).toBeNull();
    expect(worldFile(id, '1024x512', 'raw_data/world.json', 7)).toBe(
      '/systems/mgs-48213-42/bodies/c-1/sizes/1024x512/raw_data/world.json?rev=7',
    );
    expect(bodyApi(id)).toBe('/api/systems/mgs-48213-42/bodies/c-1');
  });

  it('serve each resolution of a body under sizes/', () => {
    expect(worldFile('systems/a/bodies/b', '2048x1024', 'raw_data/world.json', 5)).toBe(
      '/systems/a/bodies/b/sizes/2048x1024/raw_data/world.json?rev=5',
    );
    expect(worldFile('systems/a/bodies/b', '1024x512', 'raw_data/')).toBe('/systems/a/bodies/b/sizes/1024x512/raw_data/');
    expect(sizeName(4096, 2048)).toBe('4096x2048');
  });
});

import { systemExportUrl } from './client';

describe('system export', () => {
  it('is one download named after the system', () => {
    expect(systemExportUrl('mgs-48213-42')).toBe('/api/systems/mgs-48213-42/export');
    expect(systemExportUrl('a b')).toBe('/api/systems/a%20b/export');
  });
});
