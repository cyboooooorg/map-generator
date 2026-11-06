import { describe, expect, it } from 'vitest';
import type { RingSystem } from './api/types/RingSystem';
import { CELL_KM, ringRocks, rockPlace, ROWS, WINDOW } from './ringRocks';
import type { Vec3 } from './skyBodies';

const rings: RingSystem = { inner: 1.25, outer: 2.3, gaps: [[2.1, 0.002]], opacity: 0.8, colour: [0.86, 0.8, 0.68], bright: true, gap_moons: ['c-1'] };
const giantKm = 60_000;
const moonKm = 7;
const aKm = 2.1 * giantKm;
const moon = { orbit: { semi_major_axis_km: aKm, period_days: 0.6 }, physical: { radius_km: moonKm } };
// The giant below the moon's sky, its spin axis up, its centre aKm away.
const giant = {
  position: [0, 0, -aKm / moonKm] as Vec3,
  radius: giantKm / moonKm,
  axes: [
    [1, 0, 0],
    [0, 1, 0],
    [0, 0, 1],
  ] as [Vec3, Vec3, Vec3],
};

describe('ring rocks', () => {
  it('centres a window of rows on the moon, fixed in the ring', () => {
    const rocks = ringRocks(moon, giant, giantKm, rings, 0)!;
    expect(rocks.rows).toHaveLength(ROWS);
    // Rows are CELL_KM wide, numbered from the giant's centre, the moon's in the middle.
    expect(rocks.rows[WINDOW]!.offsetKm).toBeGreaterThan(-CELL_KM);
    expect(rocks.rows[WINDOW]!.offsetKm).toBeLessThanOrEqual(0.5 * CELL_KM);
    expect(rocks.rows[1]!.offsetKm - rocks.rows[0]!.offsetKm).toBeCloseTo(CELL_KM, 6);
    expect(rocks.firstRow).toBe(Math.floor(aKm / CELL_KM) - WINDOW);
    // The moon at the foot of its orbit, outwards away from the giant, ahead along its orbit.
    expect(rocks.foot[2]).toBeCloseTo(0, 6);
    expect(rocks.out[2]).toBeCloseTo(1, 9);
    // Prograde: anticlockwise seen from above the giant's north pole.
    expect(rocks.ahead[0]).toBeCloseTo(1, 9);
  });

  it('lets the inner rows pass the moon and the outer ones fall behind', () => {
    const at = (days: number) => ringRocks(moon, giant, giantKm, rings, days)!.rows;
    const [before, after] = [at(0), at(0.01)];
    // The moon's place along a row, in cells: it slips back along the
    // inner rows (their rocks go ahead of it), forwards along the outer.
    const place = (row: { cell: number; into: number }) => row.cell + row.into;
    const shift = (i: number) => {
      const cells = before[i]!.cells;
      let d = place(after[i]!) - place(before[i]!);
      if (d > cells / 2) d -= cells;
      if (d < -cells / 2) d += cells;
      return d;
    };
    expect(shift(0)).toBeLessThan(0);
    expect(shift(ROWS - 1)).toBeGreaterThan(0);
    // Faster the farther from the moon's orbit: the shear.
    expect(Math.abs(shift(0))).toBeGreaterThan(Math.abs(shift(WINDOW - 5)));
  });

  it('brings a cell back the same after a synodic turn of its row', () => {
    const row = 0;
    const r = (Math.floor(aKm / CELL_KM) - WINDOW + 0.5) * CELL_KM;
    const moonMotion = (2 * Math.PI) / moon.orbit.period_days;
    const synodic = (2 * Math.PI) / Math.abs(moonMotion - moonMotion * (aKm / r) ** 1.5);
    const a = ringRocks(moon, giant, giantKm, rings, 3)!.rows[row]!;
    const b = ringRocks(moon, giant, giantKm, rings, 3 + synodic)!.rows[row]!;
    expect(b.cell).toBe(a.cell);
    expect(b.into).toBeCloseTo(a.into, 3);
  });

  it('places a rock from the foot of the orbit, the distance to the giant cancelled out', () => {
    const rocks = ringRocks(moon, giant, giantKm, rings, 0)!;
    expect(rockPlace(rocks, aKm, 0, 0)).toEqual(rocks.foot);
    // 70 km out and 70 km ahead: ten moon radii each way.
    const p = rockPlace(rocks, aKm + 70, 70 / aKm, 0);
    expect(p[2] - rocks.foot[2]).toBeCloseTo(10, 1);
    expect(p[0]).toBeCloseTo(10, 1);
    // Exactly on the circle of radius r round the giant.
    const r = Math.hypot(p[0] - giant.position[0], p[1] - giant.position[1], p[2] - giant.position[2]);
    expect(r * moonKm).toBeCloseTo(aKm + 70, 6);
  });

  it('needs the giant placed', () => {
    const { radius, axes } = giant;
    expect(ringRocks(moon, { radius, axes }, giantKm, rings, 0)).toBeNull();
  });
});
