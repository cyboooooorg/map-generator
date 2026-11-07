// Places the sky labels (components/SkyLabels.svelte) so that they do not
// cover one another: each hangs from its point by a short leader, up and
// to the right by default, and moves to another side, or further up or
// down in a stack, when that place is taken. Seen near a ring plane, the
// moons, the host line and the arrows line up along it, and their labels
// all hung the same way on top of each other.

/** A label to place: its point (CSS px) and its box's size. */
export interface LabelItem {
  at: { x: number; y: number };
  width: number;
  height: number;
}

/** Where a label goes: the end of its leader, and whether its box extends left of it. */
export interface PlacedLabel {
  to: { x: number; y: number };
  left: boolean;
}

/** The leader's default reach from the point, CSS px. */
export const LEAD = { x: 34, y: 30 };
/** Gap between a leader's end and its box, CSS px (the box's translate in SkyLabels). */
const GAP = 4;
/** Space kept between boxes, CSS px. */
const MARGIN = 3;
/** How many boxes up or down a label may stack from its point. */
const STACK = 4;

interface Box {
  x0: number;
  y0: number;
  x1: number;
  y1: number;
}

/** The candidate places of a label, nearest the default first. */
function candidates(item: LabelItem): PlacedLabel[] {
  const out: PlacedLabel[] = [];
  const step = item.height + MARGIN;
  for (let k = 0; k <= STACK; k++) {
    for (const [dx, dy] of [
      [LEAD.x, -LEAD.y - k * step],
      [LEAD.x, LEAD.y + k * step],
      [-LEAD.x, -LEAD.y - k * step],
      [-LEAD.x, LEAD.y + k * step],
    ] as const) {
      out.push({ to: { x: item.at.x + dx, y: item.at.y + dy }, left: dx < 0 });
    }
  }
  return out;
}

function boxOf(item: LabelItem, place: PlacedLabel): Box {
  const x0 = place.left ? place.to.x - GAP - item.width : place.to.x + GAP;
  const y0 = place.to.y - item.height / 2;
  return { x0, y0, x1: x0 + item.width, y1: y0 + item.height };
}

function overlap(a: Box, b: Box): number {
  const w = Math.min(a.x1, b.x1) - Math.max(a.x0, b.x0) + MARGIN;
  const h = Math.min(a.y1, b.y1) - Math.max(a.y0, b.y0) + MARGIN;
  return w > 0 && h > 0 ? w * h : 0;
}

/**
 * Places `items` in a view `width` × `height`, in their order (the first
 * keep their default place the most): each at its first candidate place
 * inside the view that covers no label placed before it; else at the one
 * covering the least.
 */
export function layoutLabels(items: readonly LabelItem[], width: number, height: number): PlacedLabel[] {
  const taken: Box[] = [];
  return items.map((item) => {
    let best: { place: PlacedLabel; box: Box; cost: number } | null = null;
    for (const place of candidates(item)) {
      const box = boxOf(item, place);
      const outside = box.x0 < 0 || box.y0 < 0 || box.x1 > width || box.y1 > height;
      const covered = taken.reduce((sum, other) => sum + overlap(box, other), 0);
      // Outside the view counts as covering a whole box.
      const cost = covered + (outside ? item.width * item.height : 0);
      if (!best || cost < best.cost) best = { place, box, cost };
      if (cost === 0) break;
    }
    taken.push(best!.box);
    return best!.place;
  });
}
