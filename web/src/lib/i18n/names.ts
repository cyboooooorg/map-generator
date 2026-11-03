// Names of things that come from the generator, in the active locale.
import type { BiomeInfo } from '../api/types/BiomeInfo';
import type { Preset } from '../api/types/Preset';
import { t } from './index.svelte';

/**
 * The name of a biome: translated from its key, or the generator's own
 * name for a key this version of the web app does not know yet.
 */
export function biomeName(biome: Pick<BiomeInfo, 'key' | 'name'>): string {
  return (t.biomes as Record<string, string>)[biome.key] ?? biome.name;
}

/** The display name of a preset. */
export function presetName(preset: Pick<Preset, 'name'>): string {
  return preset.name;
}

/** The description of a preset. */
export function presetDescription(preset: Pick<Preset, 'description'>): string {
  return preset.description;
}
