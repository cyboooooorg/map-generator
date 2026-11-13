// Names of things that come from the generator, in the active locale.
import type { BiomeInfo } from '../api/types/BiomeInfo';
import type { Preset } from '../api/types/Preset';
import type { PresetInfo } from '../api/types/PresetInfo';
import type { PresetLocale } from '../api/types/PresetLocale';
import { getLocale, t } from './index.svelte';

/**
 * The name of a biome: translated from its key, or the generator's own
 * name for a key this version of the web app does not know yet.
 */
export function biomeName(biome: Pick<BiomeInfo, 'key' | 'name'>): string {
  return (t.biomes as Record<string, string>)[biome.key] ?? biome.name;
}

/**
 * Translations of the presets the server offers, by preset name. The
 * presets of `world.json` carry none (a world does not change when a
 * translation does): a world's card finds them here by the preset's name.
 */
const known = $state<{ locales: Record<string, Record<string, PresetLocale>> }>({ locales: {} });

/** Records the translations of the presets the server offers. */
export function setPresetLocales(presets: readonly PresetInfo[]): void {
  known.locales = Object.fromEntries(presets.map((p) => [p.name, p.locales]));
}

function translation(name: string): PresetLocale | undefined {
  return known.locales[name]?.[getLocale()];
}

/**
 * The display name of a preset, translated when its file says how, else
 * worded as a body type: a drawn planet's `meta.preset.name` is its class key.
 */
export function presetName(preset: Pick<Preset, 'name'>): string {
  return translation(preset.name)?.name ?? (t.viewer.bodyTypes as Record<string, string>)[preset.name] ?? preset.name;
}

/** The description of a preset, translated when its file says how. */
export function presetDescription(preset: Pick<Preset, 'name' | 'description'>): string {
  return translation(preset.name)?.description ?? preset.description;
}
