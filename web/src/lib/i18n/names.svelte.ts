// Names of things that come from the generator, in the active locale.
import type { BiomeInfo } from '../api/types/BiomeInfo';
import type { ArchetypeInfo } from '../api/types/ArchetypeInfo';
import type { ArchetypeReference } from '../api/types/ArchetypeReference';
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
  references: Record<string, ArchetypeReference | undefined>;
}>({ locales: {}, descriptions: {}, references: {} });

/** Records the translations of the archetypes the server offers. */
export function setArchetypeLocales(archetypes: readonly ArchetypeInfo[]): void {
  known.locales = Object.fromEntries(archetypes.map((a) => [a.name, a.locales]));
  known.descriptions = Object.fromEntries(archetypes.map((a) => [a.name, a.description]));
  known.references = Object.fromEntries(archetypes.map((a) => [a.name, a.reference ?? undefined]));
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

/**
 * The "similar to" line of a world that resembles an archetype, and its
 * tooltip: it names the reference body of the archetype (`Earth`, `la
 * Terre`; `51 Pegasi b` with the year it was found), a theoretical type by
 * its display name, or, for an archetype file that names no reference, the
 * archetype itself.
 */
export function similarLine(archetype: string): { text: string; hint?: string | undefined } {
  const reference = known.references[archetype];
  if (reference?.theoretical) {
    return { text: t.cards.similarTheoretical({ name: archetypeName({ name: archetype }) }), hint: t.cards.theoreticalHint };
  }
  if (reference?.name) {
    const name = translation(archetype)?.reference ?? reference.name;
    const year = reference.discovered;
    return { text: t.cards.similar({ name }), hint: year === undefined ? undefined : t.cards.firstOfKind({ year }) };
  }
  return { text: t.cards.similar({ name: archetypeName({ name: archetype }) }) };
}

/** The label of a planet's class key (`rocky`, `ice-giant`, `icy-moon`…), the key itself when unknown. */
export function className(key: string, moon = false): string {
  const moons = t.viewer.moonBodyTypes as Record<string, string>;
  return (moon ? moons[key] : undefined) ?? (t.viewer.bodyTypes as Record<string, string>)[key] ?? key;
}

/**
 * The label of a world's class with its surface state: `className` and,
 * for a rocky world whose water is frozen at the surface, `frozen
 * surface` after it (`hasFrozenSurface`). The class alone without a model.
 * A `moon` reads as a moon where the planet's label says planet.
 */
export function classLabel(key: string, model?: SurfaceModel, moon = false): string {
  const name = className(key, moon);
  return hasFrozenSurface(key, model) ? `${name} · ${t.viewer.frozenSurface}` : name;
}
