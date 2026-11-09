// The resolution chips under a planet (components/SizeChips.svelte): one
// per size offered or on disk, and what each can do.
import { sizeName } from './api/client';
import type { BodySizes } from './api/types/BodySizes';

/** One chip: a resolution of the body shown. */
export interface SizeChip {
  /** `WxH` (see sizeName), as the API names it. */
  size: string;
  width: number;
  height: number;
  /** On disk: a click shows it. Not on disk: a click generates it. */
  present: boolean;
  /** The one shown. */
  shown: boolean;
  /** On disk and not the original: it can be deleted. */
  deletable: boolean;
  /** Not on disk and larger than this server accepts: it cannot be generated. */
  tooLarge: boolean;
  /** Being generated, here or in another tab. */
  generating: boolean;
}

/**
 * The chips of a body's resolutions: its presets and its sizes on disk,
 * smallest first. `shown` and `generating` are `WxH` names.
 */
export function chipStates(sizes: BodySizes, shown: string, generating: string | null): SizeChip[] {
  const all = new Map<string, [number, number]>();
  for (const [w, h] of sizes.presets) all.set(sizeName(w, h), [w, h]);
  for (const p of sizes.present) all.set(sizeName(p.width, p.height), [p.width, p.height]);
  const original = sizeName(...sizes.original);
  const onDisk = new Set(sizes.present.map((p) => sizeName(p.width, p.height)));
  const pending = new Set(sizes.pending.map((p) => sizeName(p.width, p.height)));
  return [...all.entries()]
    .sort(([, a], [, b]) => a[0] * a[1] - b[0] * b[1] || a[0] - b[0])
    .map(([size, [width, height]]) => {
      const present = onDisk.has(size);
      return {
        size,
        width,
        height,
        present,
        shown: present && size === shown,
        deletable: present && size !== original,
        tooLarge: !present && (Math.max(width, height) > sizes.max_size || width * height > sizes.max_pixels),
        generating: !present && (size === generating || pending.has(size)),
      };
    });
}
