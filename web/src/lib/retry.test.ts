import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { retryWhileBusy, sleep } from './retry';

class Busy extends Error {}

const isBusy = (e: unknown): boolean => e instanceof Busy;

describe('retryWhileBusy', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });
  afterEach(() => {
    vi.useRealTimers();
  });

  it('retries after the delay while busy, then returns the result', async () => {
    const answers = [new Busy(), new Busy(), 'grid'];
    const attempt = vi.fn(() => {
      const next = answers.shift();
      return next instanceof Error ? Promise.reject(next) : Promise.resolve(next);
    });
    const onBusy = vi.fn();
    const result = retryWhileBusy(attempt, { signal: new AbortController().signal, isBusy, delayMs: 5000, onBusy });
    await vi.advanceTimersByTimeAsync(4999);
    expect(attempt).toHaveBeenCalledTimes(1);
    await vi.advanceTimersByTimeAsync(1);
    expect(attempt).toHaveBeenCalledTimes(2);
    await vi.advanceTimersByTimeAsync(5000);
    await expect(result).resolves.toBe('grid');
    expect(attempt).toHaveBeenCalledTimes(3);
    expect(onBusy).toHaveBeenCalledTimes(2);
  });

  it('rethrows other failures at once', async () => {
    const attempt = vi.fn(() => Promise.reject(new Error('no world')));
    await expect(
      retryWhileBusy(attempt, { signal: new AbortController().signal, isBusy, delayMs: 5000 }),
    ).rejects.toThrow('no world');
    expect(attempt).toHaveBeenCalledTimes(1);
  });

  it('stops waiting when aborted, with the abort reason', async () => {
    const controller = new AbortController();
    const attempt = vi.fn(() => Promise.reject(new Busy()));
    const result = retryWhileBusy(attempt, { signal: controller.signal, isBusy, delayMs: 5000 });
    const settled = expect(result).rejects.toMatchObject({ name: 'AbortError' });
    await vi.advanceTimersByTimeAsync(100);
    controller.abort();
    await settled;
    await vi.advanceTimersByTimeAsync(10_000);
    expect(attempt).toHaveBeenCalledTimes(1);
  });

  it('drops a result that arrives after the abort', async () => {
    const controller = new AbortController();
    const result = retryWhileBusy(
      () => {
        controller.abort();
        return Promise.resolve('stale');
      },
      { signal: controller.signal, isBusy, delayMs: 5000 },
    );
    await expect(result).rejects.toMatchObject({ name: 'AbortError' });
  });

  it('sleep rejects at once on an aborted signal', async () => {
    const controller = new AbortController();
    controller.abort();
    await expect(sleep(1000, controller.signal)).rejects.toMatchObject({ name: 'AbortError' });
  });
});
