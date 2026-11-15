// Where the reference marks of a slider go: a tick at the value's own place
// on the track (the thumb stays inside the track by half its width at each
// end), and a chip under it, stacked on another row when it would overlap the
// chip before it.

/** The width of the slider's thumb, px: the CSS of the slider uses the same. */
export const THUMB_PX = 16;
/** Space kept between two chips of a row, px. */
const GAP_PX = 4;
/** Air between two rows of chips, px. */
export const ROW_GAP_PX = 3;

/** The CSS `left` of the thumb's centre at `fraction` (0 to 1) of the slider, whatever its width. */
export function tickLeft(fraction: number, thumb = THUMB_PX): string {
  return `calc(${thumb / 2}px + ${fraction} * (100% - ${thumb}px))`;
}

/** The width a chip of `text` takes, px: an estimate for the 0.8 rem font, a pill of 8 px sides. Only used until the chip is measured. */
export function chipWidth(text: string): number {
  return Math.ceil(text.length * 6.6) + 18;
}

export interface Placed {
  /** Left edge of the chip, px from the slider's left (kept inside it). */
  left: number;
  /** Where its tick is, px from the slider's left: the chip is centred there unless it was held inside the slider. */
  centre: number;
  /** The chip is not under its tick (held at an end, or on a lower row): a leader line joins them. */
  led: boolean;
  /** 0 for the first row under the ticks; a chip that would overlap goes one row lower. */
  row: number;
}

/**
 * The place of each chip (in the order given) under a slider `track` px wide:
 * centred under its tick (`fraction` 0 to 1 of the thumb's travel), held
 * inside the slider, on the first row where it does not touch the chips
 * already there. `width` is the chip's own width, measured: the centring and
 * the overlap rule read the same number.
 */
export function placeChips(items: { fraction: number; width: number }[], track: number, thumb = THUMB_PX): Placed[] {
  // Not measured yet: one row, the CSS centres each chip under its tick.
  if (track <= 0) return items.map(() => ({ left: 0, centre: 0, row: 0, led: false }));
  const placed: Placed[] = new Array(items.length);
  const order = items.map((_, i) => i).sort((a, b) => items[a]!.fraction - items[b]!.fraction);
  const rowEnds: number[] = [];
  for (const i of order) {
    const { fraction, width } = items[i]!;
    const centre = thumb / 2 + fraction * (track - thumb);
    const left = Math.max(0, Math.min(centre - width / 2, track - width));
    let row = rowEnds.findIndex((end) => end + GAP_PX <= left);
    if (row === -1) row = rowEnds.length;
    rowEnds[row] = left + width;
    placed[i] = { left, centre, row, led: row > 0 || Math.abs(left + width / 2 - centre) > 0.5 };
  }
  return placed;
}
