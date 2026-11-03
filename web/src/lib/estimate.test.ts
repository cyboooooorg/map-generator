import { describe, expect, it } from 'vitest';
import { estimateSeconds, formatBytes, formatEstimate, serverPeakBytes, viewerBytes } from './estimate';

describe('estimates', () => {
  it('quotes the measured server peaks and interpolates between them', () => {
    const gib = 2 ** 30;
    expect(serverPeakBytes(1920 * 1080)).toBeCloseTo(0.16 * gib);
    expect(serverPeakBytes(4096 * 2048)).toBeCloseTo(0.55 * gib);
    expect(serverPeakBytes(8192 * 4096)).toBeCloseTo(2.0 * gib);
    const between = serverPeakBytes(2048 * 1024);
    expect(between).toBeGreaterThan(0.16 * gib);
    expect(between).toBeLessThan(0.55 * gib);
    expect(serverPeakBytes(1024 * 512)).toBeGreaterThan(0);
  });

  it('counts the viewer memory per pixel', () => {
    expect(viewerBytes(8192 * 4096)).toBeCloseTo(0.64e9, -7);
  });

  it('sums the learnt stage times, or gives up without all of them', () => {
    const stages = ['noise', 'rivers'];
    expect(estimateSeconds({ noise: 1, rivers: 0.5 }, stages, 4)).toBe(6);
    expect(estimateSeconds({ noise: 1 }, stages, 4)).toBeNull();
  });

  it('formats durations and sizes roughly', () => {
    expect(formatEstimate(0.4)).toBe('under a second');
    expect(formatEstimate(8.3)).toBe('about 8 s');
    expect(formatEstimate(124)).toBe('about 2 min');
    expect(formatEstimate(155)).toBe('about 2 min 40 s');
    expect(formatBytes(160 * 2 ** 20)).toBe('160 MiB');
    expect(formatBytes(2 * 2 ** 30)).toBe('2.0 GiB');
    expect(formatBytes(1e6)).toBe('10 MiB');
  });
});
