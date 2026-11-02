// Surface statistics of a world: how much of the planet each biome covers.
import type { BiomeInfo } from './api/types/BiomeInfo';

export interface BiomeShare {
  biome: BiomeInfo;
  /** Fraction of the planet surface, in [0, 1]. */
  share: number;
  areaKm2: number;
}

export interface WorldStats {
  /** Biomes present on the planet, largest first. */
  shares: BiomeShare[];
  /** Fractions of the surface covered by land and by water. */
  land: number;
  water: number;
  /** Whole planet surface, in km². */
  surfaceKm2: number;
}

/**
 * Surface covered by each biome of an equirectangular biome map (row 0 =
 * north). Pixels near the poles cover less ground: each row is weighted by
 * the cosine of its latitude, so shares are true surface fractions.
 */
export function worldStats(
  biomes: Uint8Array,
  width: number,
  height: number,
  table: readonly BiomeInfo[],
  circumferenceKm: number,
): WorldStats {
  const weights = new Float64Array(256);
  let total = 0;
  for (let y = 0; y < height; y += 1) {
    const lat = (0.5 - (y + 0.5) / height) * Math.PI;
    const w = Math.cos(lat);
    const row = y * width;
    for (let x = 0; x < width; x += 1) {
      weights[biomes[row + x] ?? 0]! += w;
    }
    total += w * width;
  }
  const radius = circumferenceKm / (2 * Math.PI);
  const surfaceKm2 = 4 * Math.PI * radius * radius;
  const shares: BiomeShare[] = [];
  let water = 0;
  weights.forEach((weight, id) => {
    const biome = table[id];
    if (weight <= 0 || !biome || total <= 0) {
      return;
    }
    const share = weight / total;
    if (biome.water) {
      water += share;
    }
    shares.push({ biome, share, areaKm2: share * surfaceKm2 });
  });
  shares.sort((a, b) => b.share - a.share);
  return { shares, land: 1 - water, water, surfaceKm2 };
}
