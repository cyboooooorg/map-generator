import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { api, ApiFailure } from './api/client';
import type { Cause } from './api/types/Cause';
import type { ResolvedSystem } from './api/types/ResolvedSystem';
import type { SystemRequest } from './api/types/SystemRequest';
import { Preview } from './preview.svelte';

vi.mock('./api/client', async (importOriginal) => {
  const original = await importOriginal<typeof import('./api/client')>();
  return { ...original, api: { resolveSystem: vi.fn() } };
});

const resolve = vi.mocked(api.resolveSystem);
const request = (seed: number): SystemRequest => ({ around: { seed } });
const root = (quantity: 'surface_temperature', value: number): Cause => ({ quantity, value, label: null, input: null, causes: [] });
const answer = (temperature: number): ResolvedSystem =>
  ({
    system: { focus: 'p1' },
    extras: { p1: { explanation: { trees: [root('surface_temperature', temperature)] }, similar: null } },
    ignored: [],
  }) as unknown as ResolvedSystem;

describe('Preview', () => {
  let preview: Preview;
  beforeEach(() => {
    vi.useFakeTimers();
    resolve.mockReset();
    preview = new Preview();
  });
  afterEach(() => {
    preview.dispose();
    vi.useRealTimers();
  });

  it('makes one call for three requests within the debounce', async () => {
    resolve.mockResolvedValue(answer(288));
    preview.request(request(1));
    await vi.advanceTimersByTimeAsync(100);
    preview.request(request(2));
    await vi.advanceTimersByTimeAsync(100);
    preview.request(request(3));
    expect(preview.pending).toBe(true);
    await vi.advanceTimersByTimeAsync(300);
    expect(resolve).toHaveBeenCalledTimes(1);
    expect(resolve.mock.calls[0]![0]).toEqual(request(3));
    expect(preview.result).not.toBeNull();
    expect(preview.pending).toBe(false);
  });

  it('drops the answer of an older request', async () => {
    let first!: (value: ResolvedSystem) => void;
    resolve.mockImplementationOnce(() => new Promise((r) => (first = r)));
    resolve.mockResolvedValueOnce(answer(300));
    preview.request(request(1));
    await vi.advanceTimersByTimeAsync(300);
    preview.request(request(2));
    await vi.advanceTimersByTimeAsync(300);
    expect(preview.result?.extras.p1?.explanation?.trees[0]?.value).toBe(300);
    first(answer(100));
    await vi.advanceTimersByTimeAsync(0);
    expect(preview.result?.extras.p1?.explanation?.trees[0]?.value).toBe(300);
  });

  it('aborts the call in flight when a new request comes', async () => {
    resolve.mockImplementation(() => new Promise(() => {}));
    preview.request(request(1));
    await vi.advanceTimersByTimeAsync(300);
    const signal = resolve.mock.calls[0]![1]!;
    preview.request(request(2));
    expect(signal.aborted).toBe(true);
  });

  it('sets the refusal of a 400 with a detail, then clears it on success', async () => {
    const detail = { code: 'bad_age', age_gyr: 12, lifetime_gyr: 3 } as const;
    resolve.mockRejectedValueOnce(new ApiFailure(400, 'bad age', detail));
    preview.request(request(1));
    await vi.advanceTimersByTimeAsync(300);
    expect(preview.refusal).toEqual(detail);
    expect(preview.unavailable).toBe(false);
    resolve.mockResolvedValueOnce(answer(288));
    preview.request(request(2));
    await vi.advanceTimersByTimeAsync(300);
    expect(preview.refusal).toBeNull();
  });

  it('is unavailable on a 500 and keeps the last result', async () => {
    resolve.mockResolvedValueOnce(answer(288));
    preview.request(request(1));
    await vi.advanceTimersByTimeAsync(300);
    resolve.mockRejectedValueOnce(new ApiFailure(500, 'boom'));
    preview.request(request(2));
    await vi.advanceTimersByTimeAsync(300);
    expect(preview.unavailable).toBe(true);
    expect(preview.refusal).toBeNull();
    expect(preview.result).not.toBeNull();
  });

  it('lists the quantities a change moved', async () => {
    resolve.mockResolvedValueOnce(answer(288));
    preview.request(request(1));
    await vi.advanceTimersByTimeAsync(300);
    resolve.mockResolvedValueOnce(answer(320));
    preview.request(request(2));
    await vi.advanceTimersByTimeAsync(300);
    expect(preview.changed.has('surface_temperature')).toBe(true);
  });
});
