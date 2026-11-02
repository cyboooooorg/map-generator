// Geometry of the detail tile quadtree served by mapgen-server
// (mapgen::detail): level z has 2^(z+1) × 2^z tiles of 256 pixels, column 0
// at longitude 0, row 0 at the north pole.

export interface TileId {
  z: number;
  x: number;
  y: number;
}

/** Tile side in pixels. */
export const TILE_SIZE = 256;
/** Deepest level the server renders. */
export const MAX_LEVEL = 8;

export function tileKey({ z, x, y }: TileId): string {
  return `${z}/${x}/${y}`;
}

export function grid(z: number): { cols: number; rows: number } {
  return { cols: 2 ** (z + 1), rows: 2 ** z };
}

/**
 * Part of the planet a tile covers, in texture coordinates (u east from
 * longitude 0, v north from the south pole): [u0, v0, du, dv].
 */
export function tileRect({ z, x, y }: TileId): [number, number, number, number] {
  const { cols, rows } = grid(z);
  return [x / cols, 1 - (y + 1) / rows, 1 / cols, 1 / rows];
}

export function children({ z, x, y }: TileId): TileId[] {
  return [
    { z: z + 1, x: 2 * x, y: 2 * y },
    { z: z + 1, x: 2 * x + 1, y: 2 * y },
    { z: z + 1, x: 2 * x, y: 2 * y + 1 },
    { z: z + 1, x: 2 * x + 1, y: 2 * y + 1 },
  ];
}

/** The ancestor of a tile at a coarser level. */
export function ancestor({ z, x, y }: TileId, level: number): TileId {
  const shift = z - level;
  return { z: level, x: x >> shift, y: y >> shift };
}

/**
 * Part of an ancestor's image that covers a tile, as [u0, v0, du, dv] in the
 * image (v down from its top row), so a patch can show a coarser tile while
 * its own loads.
 */
export function cropOf(tile: TileId, parent: TileId): [number, number, number, number] {
  const scale = 2 ** (parent.z - tile.z);
  const size = 2 ** (tile.z - parent.z);
  return [(tile.x - parent.x * size) * scale, (tile.y - parent.y * size) * scale, scale, scale];
}

/** Coarsest level whose tiles are sharper than a base map `baseWidth` pixels wide. */
export function firstDetailLevel(baseWidth: number): number {
  let z = 0;
  while (grid(z).cols * TILE_SIZE <= baseWidth && z < MAX_LEVEL) {
    z += 1;
  }
  return z;
}
