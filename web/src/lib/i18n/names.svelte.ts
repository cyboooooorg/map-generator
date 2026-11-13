// Names of things that come from the generator, in the active locale.
import type { BiomeInfo } from '../api/types/BiomeInfo';
import type { PresetInfo } from '../api/types/PresetInfo';
import type { Locale } from '../api/types/Locale';
import { getLocale, t } from './index.svelte';

/**
 * The name of a biome: translated from its key, or the generator's own
 * name for a key this version of the web app does not know yet.
 */
export function biomeName(biome: Pick<BiomeInfo, 'key' | 'name'>): string {
  return (t.biomes as Record<string, string>)[biome.key] ?? biome.name;
}

/**
 * Translations of the archetypes the server offers, by archetype name. A
 * world's `world.json` carries none (a world does not change when a
 * translation does): a world's card finds them here by the archetype's
 * name.
 */
const known = $state<{
  locales: Record<string, Record<string, Locale>>;
  descriptions: Record<string, string>;
}>({ locales: {}, descriptions: {} });

/** Records the translations of the archetypes the server offers. */
export function setPresetLocales(presets: readonly PresetInfo[]): void {
  known.locales = Object.fromEntries(presets.map((p) => [p.name, p.locales]));
  known.descriptions = Object.fromEntries(presets.map((p) => [p.name, p.description]));
}

function translation(name: string): Locale | undefined {
  return known.locales[name]?.[getLocale()];
}

/**
 * The display name of an archetype, translated when its file says how, else
 * worded as a body type: a planet's class key (`rocky`, `ice-giant`…).
 */
export function presetName(preset: { name: string }): string {
  return translation(preset.name)?.name ?? (t.viewer.bodyTypes as Record<string, string>)[preset.name] ?? preset.name;
}

/** The description of an archetype, translated when its file says how, else the server's. */
export function presetDescription(preset: { name: string; description?: string }): string {
  return translation(preset.name)?.description ?? preset.description ?? known.descriptions[preset.name] ?? '';
}
