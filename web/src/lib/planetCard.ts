// The pure parts of a planet's card in the star view of a planetary system
// (components/PlanetCard.svelte): where it sits beside its planet, and the
// planet's place relative to the habitable zone.

/** Where a planet's mean orbit lies against the habitable zone. */
export type ZonePlace = { inside: true } | { inside: false; side: 'inner' | 'outer'; gapAu: number };

/**
 * The place of an orbit of semi-major axis `au` against the habitable zone
 * from `inner` to `outer` (AU): inside it, or how far short of its inner
 * edge or beyond its outer edge.
 */
export function zonePlace(au: number, inner: number, outer: number): ZonePlace {
  if (au < inner) return { inside: false, side: 'inner', gapAu: inner - au };
  if (au > outer) return { inside: false, side: 'outer', gapAu: au - outer };
  return { inside: true };
}

/** A card's box in its view, CSS px, and the side of the planet it is on. */
export interface CardBox {
  left: number;
  top: number;
  side: 'right' | 'left';
}

/** Room between a planet and its card, CSS px. */
export const CARD_GAP = 28;
/** Room kept between a card and the edges of the view, CSS px. */
const MARGIN = 8;
/** Room kept at the top of the view, CSS px: the viewer's toolbar is there. */
export const TOP_MARGIN = 56;

/**
 * Where to put a `width` × `height` card for a planet at (`x`, `y`) in a
 * `viewWidth` × `viewHeight` view: on side `side`, else to its right, or
 * to its left when it does not fit there; centred on it vertically, and
 * kept inside the view.
 */
export function cardBox(
  x: number,
  y: number,
  width: number,
  height: number,
  viewWidth: number,
  viewHeight: number,
  side?: 'right' | 'left',
): CardBox {
  const right = x + CARD_GAP;
  const fitsRight = right + width <= viewWidth - MARGIN;
  const fitsLeft = x - CARD_GAP - width >= MARGIN;
  side ??= fitsRight || !fitsLeft ? 'right' : 'left';
  const rawLeft = side === 'right' ? right : x - CARD_GAP - width;
  const clamp = (v: number, low: number, high: number) => Math.max(low, Math.min(v, Math.max(low, high)));
  return {
    left: clamp(rawLeft, MARGIN, viewWidth - width - MARGIN),
    top: clamp(y - height / 2, TOP_MARGIN, viewHeight - height - MARGIN),
    side,
  };
}

/**
 * The point of a card's edge its link line ends at: the middle of the side
 * facing the planet, at the planet's height when the card spans it.
 */
export function linkEnd(box: CardBox, width: number, height: number, y: number): { x: number; y: number } {
  return {
    x: box.side === 'right' ? box.left : box.left + width,
    y: Math.max(box.top + 6, Math.min(y, box.top + height - 6)),
  };
}

/**
 * The cards shown: every pinned planet (body ids, in pinning order) that
 * the system has, then the hovered one (an index into `ids`) when it is
 * not pinned; each as its index into `ids`.
 */
export function shownCards(ids: readonly string[], pinned: readonly string[], hovered: number | null): { index: number; pinned: boolean }[] {
  const cards = pinned.flatMap((id) => {
    const index = ids.indexOf(id);
    return index >= 0 ? [{ index, pinned: true }] : [];
  });
  if (hovered !== null && hovered >= 0 && hovered < ids.length && !cards.some((c) => c.index === hovered)) {
    cards.push({ index: hovered, pinned: false });
  }
  return cards;
}

/** A card to place: its planet's place on the view and its size, CSS px. */
export interface CardSpot {
  x: number;
  y: number;
  width: number;
  height: number;
}

/** Vertical step, CSS px, by which a card moves away from the cards placed before it. */
const STEP = 12;
/** Room kept between two cards, CSS px. */
const SPACING = 4;

/**
 * Places several cards in turn, each as cardBox would, moved to the other
 * side of its planet or up and down by small steps when it would cover a
 * card placed before it; when none of those places is free, where cardBox
 * puts it. Earlier cards (the pinned ones) keep their places.
 */
export function layoutCards(spots: readonly CardSpot[], viewWidth: number, viewHeight: number): CardBox[] {
  const placed: { left: number; top: number; width: number; height: number }[] = [];
  const free = (box: CardBox, width: number, height: number) =>
    !placed.some(
      (p) =>
        box.left < p.left + p.width + SPACING &&
        p.left < box.left + width + SPACING &&
        box.top < p.top + p.height + SPACING &&
        p.top < box.top + height + SPACING,
    );
  return spots.map(({ x, y, width, height }) => {
    const first = cardBox(x, y, width, height, viewWidth, viewHeight);
    const sides = first.side === 'right' ? (['right', 'left'] as const) : (['left', 'right'] as const);
    let chosen = first;
    const steps = Math.ceil(viewHeight / STEP) * 2;
    search: for (let k = 0; k <= steps; k++) {
      const dy = (k % 2 === 1 ? 1 : -1) * Math.ceil(k / 2) * STEP;
      for (const side of sides) {
        const box = cardBox(x, y + dy, width, height, viewWidth, viewHeight, side);
        if (free(box, width, height)) {
          chosen = box;
          break search;
        }
      }
    }
    placed.push({ left: chosen.left, top: chosen.top, width, height });
    return chosen;
  });
}

/** The lines of a card whose values the pinned cards compare. */
export const CARD_METRICS = ['radius', 'mass', 'orbit', 'speed', 'year', 'day', 'light', 'gravity', 'equilibrium', 'moons'] as const;
export type CardMetric = (typeof CARD_METRICS)[number];
/** The highest or lowest value of a line among the cards compared. */
export type Extreme = 'max' | 'min';

/**
 * For each card's values (`rows`), the lines where it holds the highest or
 * the lowest value of all the cards: every card holding it when several do,
 * none when a line's values are all the same, and none at all with fewer
 * than two cards.
 */
export function extremes(rows: readonly Readonly<Record<CardMetric, number>>[]): Partial<Record<CardMetric, Extreme>>[] {
  const marks: Partial<Record<CardMetric, Extreme>>[] = rows.map(() => ({}));
  if (rows.length < 2) return marks;
  for (const metric of CARD_METRICS) {
    const values = rows.map((row) => row[metric]);
    const max = Math.max(...values);
    const min = Math.min(...values);
    if (!(max > min)) continue;
    values.forEach((value, k) => {
      if (value === max) marks[k]![metric] = 'max';
      else if (value === min) marks[k]![metric] = 'min';
    });
  }
  return marks;
}
