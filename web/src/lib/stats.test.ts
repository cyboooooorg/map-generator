import { describe, expect, it } from 'vitest';
import type { BiomeInfo } from './api/types/BiomeInfo';
import { worldStats } from './stats';

const table = [
  { id: 0, key: 'Ocean', name: 'Ocean', color: '#0000ff', water: true },
  { id: 1, key: 'Plain', name: 'Plain', color: '#00ff00', water: false },
] as BiomeInfo[];

describe('worldStats', () => {
  it('weights rows by latitude, so polar pixels count less', () => {
    // 1 × 4 map: two polar rows of land, two equatorial rows of ocean.
    const stats = worldStats(Uint8Array.from([1, 0, 0, 1]), 1, 4, table, 40_000);
    const ocean = stats.shares.find((s) => s.biome.id === 0)!;
    const plain = stats.shares.find((s) => s.biome.id === 1)!;
    expect(ocean.share).toBeGreaterThan(plain.share);
    expect(ocean.share + plain.share).toBeCloseTo(1, 10);
    expect(stats.water).toBeCloseTo(ocean.share, 10);
    expect(stats.land).toBeCloseTo(plain.share, 10);
    // Largest first.
    expect(stats.shares[0]?.biome.id).toBe(0);
  });

  it('measures the surface from the circumference', () => {
    const stats = worldStats(Uint8Array.from([1, 1]), 2, 1, table, 40_075);
    // Earth: about 511 million km².
    expect(stats.surfaceKm2 / 1e6).toBeCloseTo(511.2, 0);
    expect(stats.shares[0]?.areaKm2).toBeCloseTo(stats.surfaceKm2, 0);
  });
});
