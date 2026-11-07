import { describe, expect, it } from 'vitest';
import { layoutLabels, LEAD, type LabelItem } from './labelLayout';

const box = (item: LabelItem, place: { to: { x: number; y: number }; left: boolean }) => {
  const x0 = place.left ? place.to.x - 4 - item.width : place.to.x + 4;
  return { x0, x1: x0 + item.width, y0: place.to.y - item.height / 2, y1: place.to.y + item.height / 2 };
};
const overlaps = (a: ReturnType<typeof box>, b: ReturnType<typeof box>) =>
  Math.min(a.x1, b.x1) > Math.max(a.x0, b.x0) && Math.min(a.y1, b.y1) > Math.max(a.y0, b.y0);

describe('layoutLabels', () => {
  it('hangs a lone label up and to the right of its point', () => {
    const [place] = layoutLabels([{ at: { x: 200, y: 200 }, width: 120, height: 29 }], 1000, 800);
    expect(place).toEqual({ to: { x: 200 + LEAD.x, y: 200 - LEAD.y }, left: false });
  });

  it('keeps labels of points lined up along a plane off one another', () => {
    // Moons, the host line and the arrows, all near one horizontal line.
    const items: LabelItem[] = [400, 460, 520, 580, 640, 700].map((x, k) => ({ at: { x, y: 400 + (k % 2) }, width: 190, height: 29 }));
    const places = layoutLabels(items, 1280, 820);
    const boxes = items.map((item, k) => box(item, places[k]!));
    for (let i = 0; i < boxes.length; i++) {
      for (let j = i + 1; j < boxes.length; j++) expect(overlaps(boxes[i]!, boxes[j]!)).toBe(false);
    }
  });

  it('turns to the left near the right edge, and below near the top', () => {
    const [right, top] = layoutLabels(
      [
        { at: { x: 1250, y: 400 }, width: 150, height: 29 },
        { at: { x: 300, y: 10 }, width: 150, height: 29 },
      ],
      1280,
      820,
    );
    expect(right!.left).toBe(true);
    expect(top!.to.y).toBeGreaterThan(10);
  });
});
