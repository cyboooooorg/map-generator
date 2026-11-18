// The parts of a surface that were renewed the same way, for the surface
// card: one row per distinct unit of a model's terrain.
import type { Renewal } from './api/types/Renewal';
import type { Terrain } from './api/types/Terrain';

/** A part of the surface: its share of the area (0 to 1), its age in Ga and what renewed it. */
export interface Unit {
  renewal: Renewal;
  age_gyr: number;
  share: number;
}

/**
 * The rows of a terrain, largest first. Two units of the same renewal and
 * age are one row (their shares add up); none without terrain (a world
 * stored before terrain units).
 */
export function unitsFor(terrain: Terrain | undefined): Unit[] {
  const rows: Unit[] = [];
  for (const unit of terrain?.units ?? []) {
    const same = rows.find((row) => row.renewal === unit.renewal && row.age_gyr === unit.age_gyr);
    if (same) same.share += unit.share;
    else rows.push({ renewal: unit.renewal, age_gyr: unit.age_gyr, share: unit.share });
  }
  return rows.sort((a, b) => b.share - a.share);
}
