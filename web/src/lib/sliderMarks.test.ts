import { describe, expect, it } from 'vitest';
import { chipWidth, placeChips, THUMB_PX, tickLeft } from './sliderMarks';

describe('sliderMarks', () => {
  it('puts a tick at the centre of the thumb, which stays inside the track by half its width', () => {
    expect(tickLeft(0)).toBe(`calc(${THUMB_PX / 2}px + 0 * (100% - ${THUMB_PX}px))`);
    expect(tickLeft(1)).toBe(`calc(${THUMB_PX / 2}px + 1 * (100% - ${THUMB_PX}px))`);
    expect(tickLeft(0.25, 20)).toBe('calc(10px + 0.25 * (100% - 20px))');
  });

  it('centres a chip under its tick', () => {
    const [chip] = placeChips([{ fraction: 0.5, width: 40 }], 300);
    expect(chip).toEqual({ left: 150 - 20, centre: 150, row: 0, led: false });
  });

  it('keeps a chip inside the slider at either end', () => {
    const [first, last] = placeChips([{ fraction: 0, width: 60 }, { fraction: 1, width: 60 }], 300);
    expect(first!.left).toBe(0);
    expect(last!.left).toBe(240);
  });

  it('measures the centre from the thumb’s travel, and joins a held or lowered chip to its tick', () => {
    const [edge, mid, under] = placeChips([{ fraction: 0, width: 60 }, { fraction: 0.5, width: 60 }, { fraction: 0.52, width: 60 }], 300);
    expect(edge).toMatchObject({ left: 0, centre: 8, led: true });
    expect(mid).toMatchObject({ left: 150 - 30, centre: 150, row: 0, led: false });
    expect(under).toMatchObject({ row: 1, led: true });
  });

  it('puts chips of known widths exactly under their ticks, and uses the fewest rows', () => {
    const track = 400;
    const items = [{ fraction: 0.2, width: 50 }, { fraction: 0.5, width: 70 }, { fraction: 0.8, width: 50 }];
    const out = placeChips(items, track);
    out.forEach((p, i) => expect(p.left + items[i]!.width / 2).toBeCloseTo(p.centre, 6));
    expect(out.map((p) => p.row)).toEqual([0, 0, 0]);
    // A chip's own width decides the overlap: 70 + 70 do not fit in 60 px, 40 + 40 do.
    expect(placeChips([{ fraction: 0.5, width: 70 }, { fraction: 0.5 + 60 / 384, width: 70 }], track).map((p) => p.row)).toEqual([0, 1]);
    expect(placeChips([{ fraction: 0.5, width: 40 }, { fraction: 0.5 + 60 / 384, width: 40 }], track).map((p) => p.row)).toEqual([0, 0]);
  });

  it('stacks a chip that would overlap on the next row, and reuses a free row', () => {
    const items = [
      { fraction: 0.5, width: 80 },
      { fraction: 0.52, width: 80 },
      { fraction: 0.54, width: 80 },
      { fraction: 0.95, width: 50 },
    ];
    const rows = placeChips(items, 400).map((p) => p.row);
    expect(rows).toEqual([0, 1, 2, 0]);
  });

  it('places in the order given even when the fractions are not sorted', () => {
    const [a, b] = placeChips([{ fraction: 0.9, width: 40 }, { fraction: 0.1, width: 40 }], 400);
    expect(a!.left).toBeGreaterThan(b!.left);
    expect([a!.row, b!.row]).toEqual([0, 0]);
  });

  it('keeps one row until the slider is measured', () => {
    expect(placeChips([{ fraction: 0.5, width: 80 }, { fraction: 0.5, width: 80 }], 0).map((p) => p.row)).toEqual([0, 0]);
  });

  it('estimates a wider chip for a longer text', () => {
    expect(chipWidth('Earth 1')).toBeLessThan(chipWidth('Earth 1 (5.97×10²¹ t)'));
  });
});
