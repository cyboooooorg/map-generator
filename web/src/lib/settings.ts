// Per-browser preferences in localStorage. Storage can be missing or refuse
// writes (private windows, blocked site data): every access is guarded, and
// callers always get a usable value.

const PREFIX = 'mapgen.';

/** Reads a stored JSON value, or `fallback` when missing or unreadable. */
export function loadSetting<T>(key: string, fallback: T): T {
  try {
    const raw = localStorage.getItem(PREFIX + key);
    if (raw === null) {
      return fallback;
    }
    const parsed: unknown = JSON.parse(raw);
    // Keep only the fields the caller knows, with the expected types.
    if (typeof fallback === 'object' && fallback !== null && !Array.isArray(fallback)) {
      if (typeof parsed !== 'object' || parsed === null) {
        return fallback;
      }
      const merged: Record<string, unknown> = { ...(fallback as Record<string, unknown>) };
      for (const [k, v] of Object.entries(fallback)) {
        const stored = (parsed as Record<string, unknown>)[k];
        if (stored !== undefined && typeof stored === typeof v) {
          merged[k] = stored;
        }
      }
      return merged as T;
    }
    return fallback === null || typeof parsed === typeof fallback ? (parsed as T) : fallback;
  } catch {
    return fallback;
  }
}

/** Reads a stored object of numbers (unknown keys kept), or an empty one. */
export function loadNumbers(key: string): Record<string, number> {
  const stored = loadSetting<unknown>(key, null);
  const numbers: Record<string, number> = {};
  if (typeof stored === 'object' && stored !== null) {
    for (const [k, v] of Object.entries(stored)) {
      if (typeof v === 'number' && Number.isFinite(v)) {
        numbers[k] = v;
      }
    }
  }
  return numbers;
}

/** Stores a JSON value; silently does nothing when storage is unavailable. */
export function saveSetting(key: string, value: unknown): void {
  try {
    localStorage.setItem(PREFIX + key, JSON.stringify(value));
  } catch {
    // Preferences are a convenience: losing them is fine.
  }
}
