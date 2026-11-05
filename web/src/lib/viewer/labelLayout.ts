// Keeps the labels of the star view apart: with up to fifteen planets,
// each with its name, distance and speed, labels of close orbits would
// pile up into an unreadable mass.

/** A label's box on screen, CSS px: its top-left corner and its size. */
export interface LabelBox {
  x: number;
  y: number;
  width: number;
  height: number;
}

/** Where a label goes: shifted down by `dy` px (up when negative), or hidden. */
export interface LabelPlace {
  dy: number;
  visible: boolean;
}

/** Shifts tried, in label heights: in place first, then just above and below, then further. */
const SHIFTS = [0, -1, 1, -2, 2];

const overlaps = (a: LabelBox, b: LabelBox) =>
  a.x < b.x + b.width && b.x < a.x + a.width && a.y < b.y + b.height && b.y < a.y + a.height;

/**
 * Places labels in their order of importance (the first ones win): each
 * keeps its place when free, else moves up or down by whole label heights
 * (two at most) to a free place, else is hidden. `keepClear` lists boxes
 * no label may cover, such as the star.
 */
export function layoutLabels(labels: readonly LabelBox[], keepClear: readonly LabelBox[] = []): LabelPlace[] {
  const taken: LabelBox[] = [...keepClear];
  return labels.map((label) => {
    for (const shift of SHIFTS) {
      const dy = shift * label.height;
      const moved = { ...label, y: label.y + dy };
      if (!taken.some((other) => overlaps(moved, other))) {
        taken.push(moved);
        return { dy, visible: true };
      }
    }
    return { dy: 0, visible: false };
  });
}
