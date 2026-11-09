import { describe, expect, it } from 'vitest';
import { retryDelay, TILE_RENDER_VERSION, tileUrl } from './TileLayer';

describe('retryDelay', () => {
  it('backs off exponentially after network and server errors, up to a minute', () => {
    expect(retryDelay(1, null)).toBe(2000);
    expect(retryDelay(2, 500)).toBe(4000);
    expect(retryDelay(3, 503)).toBe(8000);
    expect(retryDelay(10, null)).toBe(60_000);
  });

  it('gives up on client errors, except timeouts and rate limits', () => {
    expect(retryDelay(1, 404)).toBe(Infinity);
    expect(retryDelay(1, 400)).toBe(Infinity);
    expect(retryDelay(1, 408)).toBe(2000);
    expect(retryDelay(1, 429)).toBe(2000);
  });
});

describe('tileUrl', () => {
  it('asks for the resolution shown', () => {
    const world = { id: 'systems/a/bodies/b', size: '2048x1024', revision: 5, epochMa: 0 };
    expect(tileUrl(world, '3/1/2')).toBe(`/api/systems/a/bodies/b/tiles/3/1/2?size=2048x1024&rev=5&render=${TILE_RENDER_VERSION}`);
    expect(tileUrl({ ...world, epochMa: 300 }, '3/1/2')).toContain('size=2048x1024');
    expect(tileUrl({ ...world, epochMa: 300 }, '3/1/2')).toMatch(/&epoch=300$/);
  });
});
