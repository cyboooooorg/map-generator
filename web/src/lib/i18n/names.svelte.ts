// Names of things that come from the generator, in the active locale.
import type { BiomeInfo } from '../api/types/BiomeInfo';
import type { ArchetypeInfo } from '../api/types/ArchetypeInfo';
import type { Locale } from '../api/types/Locale';
import { getLocale, t } from './index.svelte';
import { hasFrozenSurface, type SurfaceModel } from '../planetClass';

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
export function setArchetypeLocales(archetypes: readonly ArchetypeInfo[]): void {
  known.locales = Object.fromEntries(archetypes.map((a) => [a.name, a.locales]));
  known.descriptions = Object.fromEntries(archetypes.map((a) => [a.name, a.description]));
}

function translation(name: string): Locale | undefined {
  return known.locales[name]?.[getLocale()];
}

/**
 * The display name of an archetype: translated when its file says how, else
 * its own name.
 */
export function archetypeName(archetype: { name: string }): string {
  return translation(archetype.name)?.name ?? archetype.name;
}

/** The description of an archetype, translated when its file says how, else the server's. */
export function archetypeDescription(archetype: { name: string; description?: string }): string {
  return translation(archetype.name)?.description ?? archetype.description ?? known.descriptions[archetype.name] ?? '';
}

/** The label of a planet's class key (`rocky`, `ice-giant`, `icy-moon`…), the key itself when unknown. */
export function className(key: string): string {
  return (t.viewer.bodyTypes as Record<string, string>)[key] ?? key;
}

/**
 * The label of a world's class with its surface state: `className` and,
 * for a rocky world whose water is frozen at the surface, `frozen
 * surface` after it (`hasFrozenSurface`). The class alone without a model.
 */
export function classLabel(key: string, model?: SurfaceModel): string {
  return hasFrozenSurface(key, model) ? `${className(key)} · ${t.viewer.frozenSurface}` : className(key);
}
