// How much of each biome is covered by plants, for the vegetation colours
// of "Star light" (see the surface shader).
import type { Biome } from './api/types/Biome';
import type { BiomeInfo } from './api/types/BiomeInfo';

/** Share of the ground covered by plants, by biome; biomes left out have none. */
const SHARES: Partial<Record<Biome, number>> = {
  Forest: 1,
  Jungle: 1,
  Taiga: 0.9,
  Wetland: 0.8,
  Savanna: 0.6,
  Shrubland: 0.6,
  Plain: 0.6,
  Tundra: 0.35,
};

/** Share of vegetation of each biome id of a world, by id. */
export function vegetationShares(biomes: readonly Pick<BiomeInfo, 'id' | 'key'>[]): number[] {
  const shares: number[] = [];
  for (const biome of biomes) {
    shares[biome.id] = SHARES[biome.key] ?? 0;
  }
  return Array.from(shares, (share) => share ?? 0);
}
