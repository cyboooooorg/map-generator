// Rocks of a giant's rings round a ring moon, in the planet view: the
// boulders of the ring near the moon, in the ring's plane, fixed in it.
// Each one goes round the giant at its own Keplerian pace, so in the
// moon's frame the rocks inside its orbit pass it and those outside fall
// behind (the shear that opens its gap).
//
// The rocks come from a lattice fixed in the ring: rows CELL_KM wide in
// radius, each cut along its circle into cells CELL_KM long, which turn
// with that row's pace. Only the cells within WINDOW cells of the moon,
// along and across, are drawn: as the moon moves on, cells leave the
// window on one side and enter it on the other. A cell's rocks come from
// a hash of the row and cell (see ringRockVertex), so a cell is the same
// whenever it comes back. The window is drawn round, its rocks fading out
// towards ROCK_REACH_KM from the moon. Beyond them, a coarser lattice of
// the same kind holds the ring's grain: points a pixel wide out to
// GRAIN_REACH_KM, thinning out with distance into the ring's sheet. This
// module places both windows at a time; the shaders draw them.
import type { RingSystem } from './api/types/RingSystem';
import type { SkyBody, Vec3 } from './skyBodies';

/** The drawn sizes offered, over the true size: 0 hides the rocks, more than 1 is not to scale. */
export const RING_ROCK_SCALES: readonly number[] = [0, 1, 10, 30, 100];

/** Size of a cell of the lattice, across and along the ring, km. */
export const CELL_KM = 10;
/** Cells drawn on either side of the moon, across and along the ring. */
export const WINDOW = 30;
/** Rows drawn: WINDOW on either side of the moon's own. */
export const ROWS = 2 * WINDOW + 1;
/** Rocks a cell holds at most (where the ring is densest). */
export const ROCKS_PER_CELL = 8;
/** Distance from the moon the rocks fade out at: the window drawn round. */
export const ROCK_REACH_KM = WINDOW * CELL_KM;

/** Size of a cell of the grain's lattice, km. */
export const GRAIN_CELL_KM = 30;
/** Grain cells drawn on either side of the moon, across and along the ring. */
export const GRAIN_WINDOW = 30;
/** Grain rows drawn. */
export const GRAIN_ROWS = 2 * GRAIN_WINDOW + 1;
/** Grain points a cell holds at most. */
export const GRAINS_PER_CELL = 16;
/** Distance from the moon the grain fades out at, into the ring's sheet. */
export const GRAIN_REACH_KM = GRAIN_WINDOW * GRAIN_CELL_KM;
/**
 * Diameter of the largest boulders, m: those of Saturn's A ring reach 10
 * to 20 m (Cuzzi et al. 2009); a faint ring's grains are far smaller.
 */
export const BOULDER_M = { bright: 12, faint: 3 } as const;

/**
 * A window of a lattice at a time: the lattice number of its innermost row
 * (rows are numbered from the giant's centre out), and each row drawn,
 * innermost first: its middle's offset from the moon's orbit (km), the
 * cell the moon is in along it, how far into that cell (0 to 1), and the
 * cells round its circle.
 */
export interface LatticeWindow {
  firstRow: number;
  rows: { offsetKm: number; cell: number; into: number; cells: number }[];
}

/** What the view needs to draw a ring moon's rocks at a time. */
export interface RingRocks {
  /** The point of the ring plane under the moon, at its semi-major axis, in radii of the moon. */
  foot: Vec3;
  /** Outwards from the giant through the moon, ahead along its orbit, and the ring's normal (the giant's spin axis). */
  out: Vec3;
  ahead: Vec3;
  normal: Vec3;
  /** The giant: its centre and radius, in radii of the moon (for its shadow). */
  giantCentre: Vec3;
  giantRadius: number;
  /** Radius of the giant (`physical.radius_km`, the unit of its rings) and of the moon, km. */
  giantRadiusKm: number;
  moonRadiusKm: number;
  /** The moon's semi-major axis, km. */
  aKm: number;
  rings: RingSystem;
  /** The rocks' window (cells of CELL_KM, ROWS rows). */
  firstRow: number;
  rows: LatticeWindow['rows'];
  /** The grain's window (cells of GRAIN_CELL_KM, GRAIN_ROWS rows). */
  grains: LatticeWindow;
}

/**
 * The window of rocks round ring moon `moon` (a body of its system: its
 * orbit and radius) at `days`, from its giant as the sky of the moon
 * shows it (`giant`: its place, radius and axes in radii of the moon), the
 * giant's radius in km and its rings. Null when the giant is not placed.
 */
export function ringRocks(
  moon: { orbit: { semi_major_axis_km: number; period_days: number }; physical: { radius_km: number } },
  giant: Pick<SkyBody, 'position' | 'radius' | 'axes'>,
  giantRadiusKm: number,
  rings: RingSystem,
  days: number,
): RingRocks | null {
  if (!giant.position || giant.radius === undefined || !giant.axes) return null;
  const normal = unit(giant.axes[1]);
  const centre = giant.position;
  // From the giant to the moon (at the origin), in the ring's plane.
  const toMoon = scale(centre, -1);
  const out = unit(sub(toMoon, scale(normal, dot(toMoon, normal))));
  const ahead = cross(normal, out);
  const moonRadiusKm = Math.max(moon.physical.radius_km, 1e-6);
  const aKm = moon.orbit.semi_major_axis_km;
  const foot = add(centre, scale(out, aKm / moonRadiusKm));
  const moonDays = moon.orbit.period_days;
  const { firstRow, rows } = latticeWindow(aKm, moonDays, days, CELL_KM, WINDOW);
  return {
    foot,
    out,
    ahead,
    normal,
    giantCentre: [...centre] as Vec3,
    giantRadius: giant.radius,
    giantRadiusKm,
    moonRadiusKm,
    aKm,
    rings,
    firstRow,
    rows,
    grains: latticeWindow(aKm, moonDays, days, GRAIN_CELL_KM, GRAIN_WINDOW),
  };
}

/**
 * The window of a lattice of cells `cellKm` wide, `window` cells on either
 * side of a moon `aKm` from the giant that goes round in `periodDays`, at
 * `days`: each row turning at its own Keplerian pace.
 */
export function latticeWindow(aKm: number, periodDays: number, days: number, cellKm: number, window: number): LatticeWindow {
  // Mean motions, radians a day: the moon's, and a row's by Kepler's third law.
  const moonMotion = (2 * Math.PI) / Math.max(periodDays, 1e-9);
  const firstRow = Math.floor(aKm / cellKm) - window;
  const rows = Array.from({ length: 2 * window + 1 }, (_, i) => {
    const r = (firstRow + i + 0.5) * cellKm;
    const cells = Math.max(1, Math.round((2 * Math.PI * r) / cellKm));
    const motion = moonMotion * (aKm / r) ** 1.5;
    // The moon's angle in the row's frame, which turns with the row.
    const turns = (((moonMotion - motion) * days) / (2 * Math.PI)) % 1;
    const at = (turns < 0 ? turns + 1 : turns) * cells;
    const cell = Math.floor(at);
    return { offsetKm: r - aKm, cell: cell % cells, into: at - cell, cells };
  });
  return { firstRow, rows };
}

/**
 * Where a rock of row `row` stands, `along` radians ahead of the moon at
 * `rKm` from the giant's centre and `heightKm` off the plane, in radii of
 * the moon: as the vertex shader places it, from the foot of the moon's
 * orbit so that the giant's distance cancels out before it reaches 32-bit
 * floats.
 */
export function rockPlace(rocks: RingRocks, rKm: number, along: number, heightKm: number): Vec3 {
  const half = Math.sin(along / 2);
  const outKm = (rKm - rocks.aKm) * Math.cos(along) - 2 * rocks.aKm * half * half;
  const aheadKm = rKm * Math.sin(along);
  return add(
    rocks.foot,
    add(scale(rocks.out, outKm / rocks.moonRadiusKm), add(scale(rocks.ahead, aheadKm / rocks.moonRadiusKm), scale(rocks.normal, heightKm / rocks.moonRadiusKm))),
  );
}

function dot(a: Vec3, b: Vec3): number {
  return a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
}
function add(a: Vec3, b: Vec3): Vec3 {
  return [a[0] + b[0], a[1] + b[1], a[2] + b[2]];
}
function sub(a: Vec3, b: Vec3): Vec3 {
  return [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
}
function scale(a: Vec3, k: number): Vec3 {
  return [a[0] * k, a[1] * k, a[2] * k];
}
function cross(a: Vec3, b: Vec3): Vec3 {
  return [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
}
function unit(a: Vec3): Vec3 {
  const l = Math.hypot(...a) || 1;
  return scale(a, 1 / l);
}
