import { describe, expect, it } from 'vitest';
import type { BodySizes } from './api/types/BodySizes';
import { chipStates } from './sizes';

const sizes = (over: Partial<BodySizes> = {}): BodySizes => ({
  original: [2048, 1024],
  displayed: [2048, 1024],
  present: [
    { width: 2048, height: 1024, revision: 1 },
    { width: 4096, height: 2048, revision: 2 },
  ],
  pending: [],
  presets: [
    [1024, 512],
    [2048, 1024],
    [4096, 2048],
    [8192, 4096],
  ],
  max_size: 16384,
  max_pixels: 8_388_608,
  ...over,
});

describe('chipStates', () => {
  it('offers the presets and the sizes on disk, smallest first', () => {
    const chips = chipStates(sizes(), '2048x1024', null);
    expect(chips.map((c) => c.size)).toEqual(['1024x512', '2048x1024', '4096x2048', '8192x4096']);
    expect(chips.map((c) => c.width)).toEqual([1024, 2048, 4096, 8192]);
  });

  it('tells what each chip can do', () => {
    const [small, original, larger, largest] = chipStates(sizes(), '2048x1024', null);
    expect(small).toMatchObject({ present: false, shown: false, deletable: false, tooLarge: false, generating: false });
    expect(original).toMatchObject({ present: true, shown: true, deletable: false, tooLarge: false, generating: false });
    expect(larger).toMatchObject({ present: true, shown: false, deletable: true, tooLarge: false, generating: false });
    expect(largest).toMatchObject({ present: false, shown: false, deletable: false, tooLarge: true, generating: false });
  });

  it('refuses a width over the largest the server accepts', () => {
    const chips = chipStates(sizes({ max_size: 2048, max_pixels: 1e12 }), '2048x1024', null);
    expect(chips.find((c) => c.width === 4096)?.tooLarge).toBe(false); // on disk already
    expect(chips.find((c) => c.width === 8192)?.tooLarge).toBe(true);
  });

  it('marks the size being generated, here or in another tab', () => {
    const here = chipStates(sizes(), '2048x1024', '1024x512');
    expect(here[0]).toMatchObject({ present: false, generating: true });
    const elsewhere = chipStates(
      sizes({ pending: [{ width: 1024, height: 512, job_id: 'j' }] }),
      '2048x1024',
      null,
    );
    expect(elsewhere[0]).toMatchObject({ present: false, generating: true });
  });

  it('keeps a size on disk that is no longer offered', () => {
    const chips = chipStates(
      sizes({ present: [{ width: 2048, height: 1024, revision: 1 }, { width: 3000, height: 1500, revision: 3 }] }),
      '3000x1500',
      null,
    );
    expect(chips.map((c) => c.size)).toEqual(['1024x512', '2048x1024', '3000x1500', '4096x2048', '8192x4096']);
    expect(chips[2]).toMatchObject({ present: true, shown: true, deletable: true });
  });
});
