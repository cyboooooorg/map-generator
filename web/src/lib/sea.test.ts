import { describe, expect, it } from 'vitest';
import type { WorldMetadata } from './api/types/WorldMetadata';
import { seaIsLiquid } from './sea';

function meta(remap: WorldMetadata['preset']['remap']): WorldMetadata {
  return {
    preset: { remap },
    biomes: [
      { id: 1, key: 'Ocean', name: 'Ocean', color: '#1E46C8', water: true },
      { id: 22, key: 'CausticLake', name: 'Caustic Lake', color: '#30A870', water: true },
      { id: 25, key: 'RockyWaste', name: 'Rocky Waste', color: '#6E675A', water: false },
    ],
  } as unknown as WorldMetadata;
}

describe('seaIsLiquid', () => {
  it('keeps unmapped and liquid seas', () => {
    expect(seaIsLiquid(meta({}))).toBe(true);
    expect(seaIsLiquid(meta({ CausticLake: ['Ocean'] }))).toBe(true);
  });

  it('drops seas remapped to dry ground', () => {
    expect(seaIsLiquid(meta({ RockyWaste: ['DeepOcean', 'Ocean'] }))).toBe(false);
  });
});
