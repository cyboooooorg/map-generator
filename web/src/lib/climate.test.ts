import { describe, expect, it } from 'vitest';
import { celsius } from './climate';

describe('celsius', () => {
  it('matches Earth at the poles, 42° and the equator', () => {
    expect(celsius(0)).toBeCloseTo(-18, 5);
    expect(celsius(0.55)).toBeCloseTo(12, 0);
    expect(celsius(1)).toBeCloseTo(27, 5);
  });
});
