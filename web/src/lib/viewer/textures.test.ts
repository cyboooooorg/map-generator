import { describe, expect, it } from 'vitest';
import { downsample, fitFactor } from './textures';

describe('fitFactor', () => {
  it('is 1 when the image fits', () => {
    expect(fitFactor(8192, 4096, 16384)).toBe(1);
    expect(fitFactor(4096, 2048, 4096)).toBe(1);
  });

  it('shrinks the widest side under the limit', () => {
    expect(fitFactor(8192, 4096, 4096)).toBe(2);
    expect(fitFactor(8193, 4096, 4096)).toBe(3);
  });
});

describe('downsample', () => {
  it('returns the same array at factor 1', () => {
    const data = Uint8Array.from([1, 2, 3, 4]);
    expect(downsample(data, 2, 2, 1, 1).data).toBe(data);
  });

  it('keeps the centre pixel of each block, every channel, with the same type', () => {
    // 4 × 2 pixels of two channels: value = 10 × pixel index + channel.
    const data = Uint16Array.from({ length: 16 }, (_, k) => 10 * Math.floor(k / 2) + (k % 2));
    const { data: out, width, height } = downsample(data, 4, 2, 2, 2);
    expect(out).toBeInstanceOf(Uint16Array);
    expect([width, height]).toEqual([2, 1]);
    // Block centres: pixel (1, 1) = index 5 and (3, 1) = index 7.
    expect(Array.from(out)).toEqual([50, 51, 70, 71]);
  });

  it('clamps the last partial block to the image', () => {
    const data = Uint8Array.from([0, 1, 2]);
    const { data: out, width } = downsample(data, 3, 1, 1, 2);
    expect(width).toBe(2);
    expect(Array.from(out)).toEqual([1, 2]);
  });
});
