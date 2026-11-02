import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { debouncedSetting, loadSetting } from './settings';

/** A Map-backed stand-in for localStorage, counting writes. */
function fakeStorage() {
  const items = new Map<string, string>();
  const storage = {
    writes: 0,
    getItem: (key: string) => items.get(key) ?? null,
    setItem: (key: string, value: string) => {
      storage.writes += 1;
      items.set(key, value);
    },
  };
  return storage;
}

describe('debouncedSetting', () => {
  let storage: ReturnType<typeof fakeStorage>;

  beforeEach(() => {
    vi.useFakeTimers();
    storage = fakeStorage();
    vi.stubGlobal('localStorage', storage);
  });

  afterEach(() => {
    vi.useRealTimers();
    vi.unstubAllGlobals();
  });

  it('writes the last value once the changes stop', () => {
    const setting = debouncedSetting<number>('season', 300);
    for (let i = 1; i <= 60; i += 1) {
      setting.save(i / 60);
      vi.advanceTimersByTime(16);
    }
    expect(storage.writes).toBe(0);
    vi.advanceTimersByTime(300);
    expect(storage.writes).toBe(1);
    expect(loadSetting('season', 0)).toBe(1);
  });

  it('writes a pending value on flush, and nothing without one', () => {
    const setting = debouncedSetting<string>('mode', 300);
    setting.flush();
    expect(storage.writes).toBe(0);
    setting.save('map');
    setting.flush();
    expect(storage.writes).toBe(1);
    expect(loadSetting('mode', 'globe')).toBe('map');
    vi.advanceTimersByTime(1000);
    expect(storage.writes).toBe(1);
  });
});
