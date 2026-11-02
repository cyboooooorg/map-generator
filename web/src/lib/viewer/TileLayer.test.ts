import { describe, expect, it } from 'vitest';
import { retryDelay } from './TileLayer';

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
