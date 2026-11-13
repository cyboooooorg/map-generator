/**
 * The classes drawn through the giant path: the giants (hot Neptune included) and the
 * mini-Neptune, whose radius is the giant look's (issue #446). The
 * presets a request around a planet names (`terran`, `barren`, …) are
 * never giants.
 */
export const GIANT_CLASSES: ReadonlySet<string> = new Set(['gas-giant', 'ice-giant', 'hot-jupiter', 'hot-neptune', 'mini-neptune']);

/** Whether a body type (a class key or a preset name) is drawn as a giant. */
export const isGiantType = (type: string): boolean => GIANT_CLASSES.has(type);
