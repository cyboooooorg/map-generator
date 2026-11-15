// Which themed cards a body's details are split into (docs/superpowers/
// specs/2026-11-14-planet-model-part5-design.md, *Card layout*). The header
// is always there; the others appear once the body has what they show.
import type { Body } from './api/types/Body';
import type { WorldMetadata } from './api/types/WorldMetadata';
import { dustFedBy } from './ringMoons';

export const CARD_IDS = ['header', 'orbit', 'interior', 'atmosphere', 'surface', 'legend', 'moons', 'map'] as const;
export type CardId = (typeof CARD_IDS)[number];

/** The dock id of a card of the info panel. */
export const cardDockId = (id: CardId): string => `info.${id}`;

/**
 * Every dock id the info panel and its cards can ever have, offered or not:
 * the dock keeps a remembered layout for them while the data they wait for
 * (the system, the maps) loads.
 */
export const infoDockIds = (): string[] => ['info', ...CARD_IDS.map(cardDockId)];

/**
 * The cards of `body`, in their order. `meta` is the body's own metadata
 * (null until its maps exist) and `hasStats` whether its biome statistics
 * are measured, `planet` the planet of a moon. Nothing without a body.
 */
export function cardsFor(body: Body | null, meta: WorldMetadata | null, hasStats: boolean, planet: Body | null = null): CardId[] {
  if (!body) return [];
  const giant = meta?.giant != null;
  const cards: CardId[] = ['header', 'orbit', 'interior'];
  // A giant's air is known from its maps; a rocky body's from its model.
  if (giant || body.model?.surface) cards.push('atmosphere');
  // Land, water and rivers are measured on the maps of a rocky world.
  if (meta && !giant && hasStats) cards.push('surface');
  if (meta) cards.push('legend');
  if (body.kind === 'planet' || body.ring_role || body.params.ridge || dustFedBy(body, planet)) cards.push('moons');
  if (meta) cards.push('map');
  return cards;
}
