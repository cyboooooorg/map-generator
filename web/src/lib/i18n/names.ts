// Names of things that come from the generator, in the active locale.
import type { BiomeInfo } from '../api/types/BiomeInfo';
import { t } from './index.svelte';

/**
 * The name of a biome: translated from its key, or the generator's own
 * name for a key this version of the web app does not know yet.
 */
export function biomeName(biome: Pick<BiomeInfo, 'key' | 'name'>): string {
  return (t.biomes as Record<string, string>)[biome.key] ?? biome.name;
}
