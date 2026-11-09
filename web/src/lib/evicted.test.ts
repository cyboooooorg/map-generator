import { describe, expect, it } from 'vitest';
import type { GeneratorNote } from './api/types/GeneratorNote';
import { evictedKey, regenerationToQueue, shouldTell } from './evicted';

const note: GeneratorNote = { version: '0.1.0', format: 3, current_version: '0.2.0', current_format: 4 };

describe('evicted bodies', () => {
  it('queues only an evicted body, once', () => {
    const asked = new Set<string>();
    expect(regenerationToQueue('mgs-1', 'c', ['c'], asked)).toBe('mgs-1/c');
    // A body never generated waits for the user's click on its card.
    expect(regenerationToQueue('mgs-1', 'b', ['c'], asked)).toBeNull();
    expect(regenerationToQueue(null, 'c', ['c'], asked)).toBeNull();
    expect(regenerationToQueue('mgs-1', null, ['c'], asked)).toBeNull();
    asked.add(evictedKey('mgs-1', 'c'));
    expect(regenerationToQueue('mgs-1', 'c', ['c'], asked)).toBeNull();
    // The same body of another system is another body.
    expect(regenerationToQueue('mgs-2', 'c', ['c'], asked)).toBe('mgs-2/c');
  });

  it('tells the user of an older world once per body', () => {
    const told = new Set<string>();
    expect(shouldTell(null, 'mgs-1/c', told)).toBe(false);
    expect(shouldTell(note, 'mgs-1/c', told)).toBe(true);
    expect(shouldTell(note, 'mgs-1/c', told)).toBe(false);
    expect(shouldTell(note, 'mgs-1/c-1', told)).toBe(true);
  });
});
