// Keyboard shortcuts of the viewer: which key does what, the help table, and
// the small steps some of them take.

/** What a shortcut does. */
export type Shortcut =
  | { action: 'layer'; index: number }
  | {
      action:
        | 'globe'
        | 'map'
        | 'reset'
        | 'hexes'
        | 'sunlight'
        | 'seasons'
        | 'atmosphere'
        | 'clouds'
        | 'drift'
        | 'grid'
        | 'tropics'
        | 'depthContours'
        | 'elevationContours'
        | 'measure'
        | 'screenshot'
        | 'help'
        | 'escape'
        | 'previousWorld'
        | 'nextWorld'
        | 'reliefUp'
        | 'reliefDown';
    };

type Simple = Exclude<Shortcut, { action: 'layer' }>['action'];

/** Number of layers reachable with the digit keys. */
export const LAYER_KEYS = 8;

const KEYS: Record<string, Simple> = {
  g: 'globe',
  m: 'map',
  r: 'reset',
  h: 'hexes',
  l: 'sunlight',
  s: 'seasons',
  a: 'atmosphere',
  c: 'clouds',
  w: 'drift',
  n: 'grid',
  t: 'tropics',
  k: 'depthContours',
  j: 'elevationContours',
  d: 'measure',
  p: 'screenshot',
  '?': 'help',
  escape: 'escape',
  '[': 'previousWorld',
  ']': 'nextWorld',
  // `=` is `+` without Shift on US keyboards; `_` is Shift + `-`.
  '+': 'reliefUp',
  '=': 'reliefUp',
  '-': 'reliefDown',
  _: 'reliefDown',
};

/**
 * The shortcut of a key (`KeyboardEvent.key`), or null. Letters are matched
 * whatever their case.
 */
export function shortcutFor(key: string): Shortcut | null {
  if (key.length === 1 && key >= '1' && key <= String(LAYER_KEYS)) {
    return { action: 'layer', index: Number(key) - 1 };
  }
  const action = KEYS[key.toLowerCase()];
  return action ? { action } : null;
}

/**
 * Whether a key press with Alt held should still count: Alt (Option)
 * produces the brackets and some symbols on several layouts (French, German
 * on macOS), but Alt + a letter or digit is a browser or system command.
 */
export function altAllowed(key: string): boolean {
  return key.length === 1 && !/[\p{L}\p{N}]/u.test(key);
}

/** The help table: keys, and what they do. */
export const SHORTCUT_HELP: readonly [string, string][] = [
  ['G / M', 'Globe / flat map'],
  [`1 – ${LAYER_KEYS}`, 'Layer'],
  ['+ / −', 'More / less relief'],
  ['[ / ]', 'Previous / next world of the list'],
  ['R', 'Reset the view'],
  ['H', 'Hexes'],
  ['L', 'Sunlight'],
  ['S', 'Play / pause the seasons (turns sunlight on)'],
  ['A', 'Atmosphere'],
  ['C', 'Clouds'],
  ['W', 'Cloud drift (turns clouds on)'],
  ['N', 'Grid'],
  ['T', 'Tropics and polar circles'],
  ['K', 'Depth contours (isobaths)'],
  ['J', 'Elevation contours'],
  ['D', 'Measure distances'],
  ['P', 'Screenshot'],
  ['Esc', 'Close this help or the point panel, stop measuring'],
  ['?', 'This help'],
];

/** Relief exaggeration change of one `+` or `-` press. */
export const RELIEF_STEP = 0.1;

/** The relief after one step up (`direction` 1) or down (-1), within [0, 1]. */
export function stepRelief(relief: number, direction: 1 | -1): number {
  const next = Math.round((relief + direction * RELIEF_STEP) * 100) / 100;
  return Math.min(1, Math.max(0, next));
}

/**
 * The world before (`direction` -1) or after (1) `current` in `order`,
 * wrapping around; the first (or last) one when `current` is not in the
 * list, null when the list is empty or holds only `current`.
 */
export function stepWorld(order: readonly string[], current: string | null, direction: 1 | -1): string | null {
  if (order.length === 0) {
    return null;
  }
  const index = current === null ? -1 : order.indexOf(current);
  if (index === -1) {
    return (direction === 1 ? order[0] : order[order.length - 1]) ?? null;
  }
  const next = order[(index + direction + order.length) % order.length] ?? null;
  return next === current ? null : next;
}
