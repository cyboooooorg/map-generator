// Web Worker: fetches the biome map of a world and computes its biome
// statistics, without the other layers. A moon's card shows its planet's
// statistics without loading the planet (see `loadPeekedStats`).
import type { WorldMetadata } from '../api/types/WorldMetadata';
import { worldStats, type WorldStats } from '../stats';
import { decodeBiomes } from './decode';

export interface StatsRequest {
  /** URL of the biome map. */
  biomes: string;
  meta: WorldMetadata;
}

export type StatsResponse = { ok: true; stats: WorldStats } | { ok: false; error: string };

self.onmessage = async (event: MessageEvent<StatsRequest>) => {
  const { biomes, meta } = event.data;
  try {
    const response = await fetch(biomes);
    if (!response.ok) {
      throw new Error(`${biomes}: ${response.status} ${response.statusText}`);
    }
    const ids = decodeBiomes(meta, await response.arrayBuffer());
    const stats = worldStats(ids, meta.width, meta.height, meta.biomes, meta.circumference_km);
    const message: StatsResponse = { ok: true, stats };
    self.postMessage(message);
  } catch (error) {
    const message: StatsResponse = { ok: false, error: String(error) };
    self.postMessage(message);
  }
};
