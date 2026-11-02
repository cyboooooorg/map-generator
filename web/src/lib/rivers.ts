// The vector rivers of a world (raw_data/rivers.json) and finding the river
// nearest to a point.
import type { RiverEntry } from './api/types/RiverEntry';
import type { RiversFile } from './api/types/RiversFile';
import { distanceKm, type LonLat } from './geo';

/** Bucket size of the spatial index, in degrees. */
const CELL_DEG = 2;
const COLUMNS = 360 / CELL_DEG;
const ROWS = 180 / CELL_DEG;

export interface RiverHit {
  river: RiverEntry;
  /** Drainage area at the nearest point of the river, in km². */
  flowKm2: number;
  /** Distance from the point to the river, in degrees of arc (approximate). */
  distanceDeg: number;
}

function wrapDeg(lon: number): number {
  const wrapped = lon % 360;
  return wrapped < 0 ? wrapped + 360 : wrapped;
}

/** Rivers with a bucket index of their segments, for nearest-river queries. */
export class RiverIndex {
  private readonly cells = new Map<number, [river: number, segment: number][]>();
  private readonly lengths = new Map<number, number>();
  readonly byId = new Map<number, RiverEntry>();

  constructor(
    readonly file: RiversFile,
    private readonly circumferenceKm: number,
  ) {
    file.rivers.forEach((river, r) => {
      this.byId.set(river.id, river);
      for (let s = 0; s + 1 < river.points.length; s += 1) {
        const [lon0, lat0] = river.points[s]!;
        const [lon1, lat1] = river.points[s + 1]!;
        // A segment is short (a fraction of a degree): its two end cells cover it.
        for (const [lon, lat] of [
          [lon0, lat0],
          [lon1, lat1],
        ] as const) {
          const key = this.key(lon, lat);
          let bucket = this.cells.get(key);
          if (!bucket) {
            bucket = [];
            this.cells.set(key, bucket);
          }
          const last = bucket[bucket.length - 1];
          if (!last || last[0] !== r || last[1] !== s) {
            bucket.push([r, s]);
          }
        }
      }
    });
  }

  private key(lonDeg: number, latDeg: number): number {
    const col = Math.floor(wrapDeg(lonDeg) / CELL_DEG) % COLUMNS;
    const row = Math.min(ROWS - 1, Math.max(0, Math.floor((latDeg + 90) / CELL_DEG)));
    return row * COLUMNS + col;
  }

  /** The river passing closest to a point, if one is within `toleranceDeg`. */
  nearest(point: LonLat, toleranceDeg: number): RiverHit | null {
    const lon = wrapDeg((point.lon * 180) / Math.PI);
    const lat = (point.lat * 180) / Math.PI;
    const cosLat = Math.max(0.05, Math.cos(point.lat));
    const reachCols = Math.ceil(toleranceDeg / cosLat / CELL_DEG);
    const reachRows = Math.ceil(toleranceDeg / CELL_DEG);
    const baseCol = Math.floor(lon / CELL_DEG);
    const baseRow = Math.floor((lat + 90) / CELL_DEG);
    let best: RiverHit | null = null;
    for (let dr = -reachRows; dr <= reachRows; dr += 1) {
      const row = baseRow + dr;
      if (row < 0 || row >= ROWS) {
        continue;
      }
      for (let dc = -reachCols; dc <= reachCols; dc += 1) {
        const col = (((baseCol + dc) % COLUMNS) + COLUMNS) % COLUMNS;
        for (const [r, s] of this.cells.get(row * COLUMNS + col) ?? []) {
          const river = this.file.rivers[r]!;
          const a = river.points[s]!;
          const b = river.points[s + 1]!;
          // Local flat projection around the point, longitudes unwrapped.
          const unwrap = (x: number) => ((((x - lon) % 360) + 540) % 360) - 180;
          const ax = unwrap(a[0]) * cosLat;
          const ay = a[1] - lat;
          const bx = unwrap(b[0]) * cosLat;
          const by = b[1] - lat;
          const dx = bx - ax;
          const dy = by - ay;
          const length2 = dx * dx + dy * dy;
          const t = length2 > 0 ? Math.min(1, Math.max(0, -(ax * dx + ay * dy) / length2)) : 0;
          const distance = Math.hypot(ax + t * dx, ay + t * dy);
          if (distance <= toleranceDeg && (!best || distance < best.distanceDeg)) {
            best = { river, flowKm2: a[2] + t * (b[2] - a[2]), distanceDeg: distance };
          }
        }
      }
    }
    return best;
  }

  /** Length of a river from its head to its mouth, in km. */
  lengthKm(river: RiverEntry): number {
    let length = this.lengths.get(river.id);
    if (length === undefined) {
      length = 0;
      for (let i = 1; i < river.points.length; i += 1) {
        const [lon0, lat0] = river.points[i - 1]!;
        const [lon1, lat1] = river.points[i]!;
        const rad = Math.PI / 180;
        length += distanceKm(
          { lon: lon0 * rad, lat: lat0 * rad },
          { lon: lon1 * rad, lat: lat1 * rad },
          this.circumferenceKm,
        );
      }
      this.lengths.set(river.id, length);
    }
    return length;
  }

  /** Drainage area at the mouth of a river, in km². */
  mouthFlowKm2(river: RiverEntry): number {
    return river.points[river.points.length - 1]?.[2] ?? 0;
  }
}
