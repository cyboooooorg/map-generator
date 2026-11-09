import { describe, expect, it, vi } from 'vitest';
import { FrameStats, frameRegistry, frameStatsEnabled } from './frameStats';

/** A clock that advances by the next duration each time it is read in pairs. */
function fakeClock(durations: number[]): () => number {
  let t = 0;
  let reads = 0;
  return () => {
    const value = t;
    reads += 1;
    // Even reads open a frame, odd reads close it.
    if (reads % 2 === 1) t += durations[(reads - 1) / 2] ?? 0;
    return value;
  };
}

const gl = () => ({ readPixels: vi.fn(), RGBA: 1, UNSIGNED_BYTE: 2 }) as never;

describe('frameStats', () => {
  it('is off unless asked', () => {
    expect(frameStatsEnabled('')).toBe(false);
    expect(frameStatsEnabled('?frames=0')).toBe(false);
    expect(frameStatsEnabled('?frames=1')).toBe(true);
    expect(frameStatsEnabled('?a=b&frames=1')).toBe(true);
  });

  it('summarises a known series', () => {
    const series = Array.from({ length: 100 }, (_, i) => i + 1);
    const stats = new FrameStats('test', 300, fakeClock(series), true);
    for (let i = 0; i < 100; i++) stats.measure(gl(), () => {});
    const s = stats.summary();
    expect(s?.frames).toBe(100);
    expect(s?.medianMs).toBeCloseTo(50.5, 6);
    expect(s?.p95Ms).toBeCloseTo(95.05, 6);
  });

  it('keeps only the last window', () => {
    const series = Array.from({ length: 25 }, (_, i) => i + 1);
    const stats = new FrameStats('test', 10, fakeClock(series), true);
    for (let i = 0; i < 25; i++) stats.measure(gl(), () => {});
    const s = stats.summary();
    expect(s?.frames).toBe(10);
    expect(s?.medianMs).toBeCloseTo(20.5, 6);
  });

  it('forces the GPU to finish when enabled', () => {
    const g = gl();
    const stats = new FrameStats('test', 10, fakeClock([1]), true);
    stats.measure(g, () => {});
    expect((g as unknown as { readPixels: unknown }).readPixels).toHaveBeenCalledTimes(1);
  });

  it('only draws when disabled', () => {
    const g = gl();
    const draw = vi.fn();
    const clock = vi.fn(() => 0);
    const stats = new FrameStats('off', 10, clock, false);
    stats.measure(g, draw);
    expect(draw).toHaveBeenCalledTimes(1);
    expect(clock).not.toHaveBeenCalled();
    expect((g as unknown as { readPixels: unknown }).readPixels).not.toHaveBeenCalled();
    expect(stats.summary()).toBeNull();
    expect(frameRegistry().has('off')).toBe(false);
  });

  it('registers while enabled and leaves on dispose', () => {
    const stats = new FrameStats('reg', 10, fakeClock([]), true);
    expect(frameRegistry().get('reg')).toBe(stats);
    stats.dispose();
    expect(frameRegistry().has('reg')).toBe(false);
  });
});
