import { describe, expect, it } from 'vitest';
import { layoutLabels } from './labelLayout';

const box = (x: number, y: number, width = 100, height = 16) => ({ x, y, width, height });

describe('label layout', () => {
  it('keeps labels that do not overlap where they are', () => {
    expect(layoutLabels([box(0, 0), box(0, 40), box(200, 0)])).toEqual([
      { dy: 0, visible: true },
      { dy: 0, visible: true },
      { dy: 0, visible: true },
    ]);
  });

  it('moves a label off an earlier one, by whole label heights', () => {
    const [first, second] = layoutLabels([box(0, 0), box(30, 4)]);
    expect(first).toEqual({ dy: 0, visible: true });
    expect(second!.visible).toBe(true);
    expect(Math.abs(second!.dy) % 16).toBe(0);
    expect(second!.dy).not.toBe(0);
  });

  it('hides a label that finds no free place, earlier labels winning', () => {
    // A column of labels fills every place the last one could take.
    const column = [-2, -1, 0, 1, 2].map((k) => box(0, k * 16));
    const placed = layoutLabels([...column, box(0, 0)]);
    expect(placed.slice(0, 5).every((p) => p.visible && p.dy === 0)).toBe(true);
    expect(placed[5]!.visible).toBe(false);
  });

  it('avoids the boxes it is told to keep clear, such as the star', () => {
    const [label] = layoutLabels([box(0, 0)], [box(0, 0, 100, 20)]);
    expect(label!.visible).toBe(true);
    expect(label!.dy).not.toBe(0);
  });

  it('moves a tilted label by its step, not by its tall box', () => {
    const [, tilted] = layoutLabels([box(0, 0, 100, 20), { ...box(0, 10, 20, 60), step: 18 }]);
    expect(tilted!.visible).toBe(true);
    expect(Math.abs(tilted!.dy) % 18).toBe(0);
    expect(Math.abs(tilted!.dy)).toBeLessThanOrEqual(36);
  });
});
