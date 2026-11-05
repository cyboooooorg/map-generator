import { describe, expect, it } from 'vitest';
import { CARD_GAP, cardBox, layoutCards, linkEnd, shownCards, zonePlace } from './planetCard';

describe('zonePlace', () => {
  it('places an orbit inside, short of or beyond the habitable zone', () => {
    expect(zonePlace(1, 0.95, 1.4)).toEqual({ inside: true });
    expect(zonePlace(0.7, 0.95, 1.4)).toEqual({ inside: false, side: 'inner', gapAu: 0.95 - 0.7 });
    expect(zonePlace(5.2, 0.95, 1.4)).toEqual({ inside: false, side: 'outer', gapAu: 5.2 - 1.4 });
  });
});

describe('cardBox', () => {
  it('puts the card right of the planet, centred on it', () => {
    expect(cardBox(100, 300, 200, 120, 1000, 800)).toEqual({ left: 100 + CARD_GAP, top: 240, side: 'right' });
  });

  it('puts the card left of a planet near the right edge', () => {
    const box = cardBox(900, 300, 200, 120, 1000, 800);
    expect(box.side).toBe('left');
    expect(box.left).toBe(900 - CARD_GAP - 200);
  });

  it('keeps the card inside the view', () => {
    const box = cardBox(50, 5, 200, 120, 1000, 800);
    expect(box.top).toBe(8);
    const low = cardBox(50, 795, 200, 120, 1000, 800);
    expect(low.top).toBe(800 - 120 - 8);
    // Too narrow for either side: right, pushed back in.
    const narrow = cardBox(150, 300, 200, 120, 300, 800);
    expect(narrow.side).toBe('right');
    expect(narrow.left).toBe(300 - 200 - 8);
  });
});

describe('linkEnd', () => {
  it('ends on the side facing the planet, at its height when it can', () => {
    expect(linkEnd({ left: 128, top: 240, side: 'right' }, 200, 120, 300)).toEqual({ x: 128, y: 300 });
    expect(linkEnd({ left: 672, top: 240, side: 'left' }, 200, 120, 300)).toEqual({ x: 872, y: 300 });
    expect(linkEnd({ left: 128, top: 8, side: 'right' }, 200, 120, 0)).toEqual({ x: 128, y: 14 });
  });
});

describe('shownCards', () => {
  const ids = ['b', 'c', 'd'];

  it('shows the pinned cards in pinning order, then the hovered one', () => {
    expect(shownCards(ids, ['d', 'b'], 1)).toEqual([
      { index: 2, pinned: true },
      { index: 0, pinned: true },
      { index: 1, pinned: false },
    ]);
  });

  it('shows a hovered pinned planet once, and skips ids the system lacks', () => {
    expect(shownCards(ids, ['c', 'x'], 1)).toEqual([{ index: 1, pinned: true }]);
    expect(shownCards(ids, [], null)).toEqual([]);
  });
});

describe('layoutCards', () => {
  const overlap = (a: { left: number; top: number }, b: { left: number; top: number }, w: number, h: number) =>
    a.left < b.left + w && b.left < a.left + w && a.top < b.top + h && b.top < a.top + h;

  it('places a lone card as cardBox does', () => {
    expect(layoutCards([{ x: 100, y: 300, width: 200, height: 120 }], 1000, 800)).toEqual([cardBox(100, 300, 200, 120, 1000, 800)]);
  });

  it('keeps the first card in place and moves the next one off it', () => {
    const [a, b] = layoutCards(
      [
        { x: 100, y: 300, width: 200, height: 120 },
        { x: 110, y: 310, width: 200, height: 120 },
      ],
      1000,
      800,
    );
    expect(a).toEqual(cardBox(100, 300, 200, 120, 1000, 800));
    expect(overlap(a!, b!, 200, 120)).toBe(false);
  });

  it('tries the other side of the planet before moving a card up or down', () => {
    const [, b] = layoutCards(
      [
        { x: 500, y: 300, width: 200, height: 120 },
        { x: 500, y: 300, width: 200, height: 120 },
      ],
      1000,
      800,
    );
    expect(b).toEqual({ left: 500 - CARD_GAP - 200, top: 240, side: 'left' });
  });
});
