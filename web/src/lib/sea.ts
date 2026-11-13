import type { WorldMetadata } from './api/types/WorldMetadata';

/**
 * Whether a world's sea is liquid: false when its look remaps the
 * ocean to a dry biome (barren worlds), so the viewer neither flattens its
 * basins nor makes the sun glint on them. Frozen and magma seas count as
 * liquid surfaces.
 */
export function seaIsLiquid(meta: WorldMetadata): boolean {
  const remap = meta.look.remap;
  const target = (Object.keys(remap) as (keyof typeof remap)[]).find((key) => remap[key]?.includes('Ocean'));
  if (!target) {
    return true;
  }
  return meta.biomes.find((b) => b.key === target)?.water ?? true;
}
