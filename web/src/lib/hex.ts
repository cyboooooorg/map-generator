// Locating a point in the hex grid of a world, as mapgen::hex::HexGrid::locate
// and the shader (hexLocate in viewer/shaders.ts) do: pointy-top hexes, odd-r
// offset rows, columns wrapping around the planet.
import type { HexMap } from './api/types/HexMap';
import { lonLatToUv, type LonLat } from './geo';

export interface HexCell {
  col: number;
  row: number;
  /** Index into the `HexMap.cells` arrays. */
  index: number;
}

/** Hex containing map point (px, py), in pixels from the north-west corner. */
export function locateHex(map: HexMap, px: number, py: number): HexCell {
  const size = map.hex_size_px;
  const x = px - 0.5 * map.hex_width_px;
  const y = py - size;
  const q = ((Math.sqrt(3) / 3) * x - y / 3) / size;
  const r = ((2 / 3) * y) / size;
  const cx = q;
  const cz = r;
  const cy = -cx - cz;
  let rx = Math.round(cx);
  const ry = Math.round(cy);
  let rz = Math.round(cz);
  const dx = Math.abs(rx - cx);
  const dy = Math.abs(ry - cy);
  const dz = Math.abs(rz - cz);
  if (dx > dy && dx > dz) {
    rx = -ry - rz;
  } else if (dy <= dz) {
    rz = -rx - ry;
  }
  const row = Math.min(map.rows - 1, Math.max(0, rz));
  const col = (((rx + Math.floor(rz / 2)) % map.columns) + map.columns) % map.columns;
  return { col, row, index: row * map.columns + col };
}

/** Hex containing a point of the planet. */
export function hexAt(map: HexMap, point: LonLat): HexCell {
  const [u, v] = lonLatToUv(point);
  return locateHex(map, u * map.source.width, (1 - v) * map.source.height);
}
