// What the body cards read: a loaded world (every member, from its files) or
// a resolved body (the generate form's live preview, which has no map files).
import type { Body } from './api/types/Body';
import type { BodyExtras } from './api/types/BodyExtras';
import type { Quantity } from './api/types/Quantity';
import type { ResolvedSystem } from './api/types/ResolvedSystem';
import type { System } from './api/types/System';
import type { WorldMetadata } from './api/types/WorldMetadata';
import type { RiverIndex } from './rivers';
import type { WorldStats } from './stats';

/**
 * The members the six body cards (header, orbit, interior, atmosphere, surface,
 * moons) read. `LoadedWorld` satisfies it as it is. Without map files `meta`,
 * `stats` and `rivers` are null and the cards show the model's values only.
 */
export interface CardSource {
  readonly system: System | null;
  readonly systemId: string | null;
  readonly systemNames: Record<string, string>;
  readonly systemEvicted: string[];
  readonly systemBody: string | null;
  readonly shownSystemBody: Body | null;
  readonly shownExtras: BodyExtras | null;
  readonly meta: WorldMetadata | null;
  readonly stats: WorldStats | null;
  readonly rivers: RiverIndex | null;
  /** The roots of the cause trees that moved since the previous preview: their lines are highlighted. */
  readonly changed?: ReadonlySet<Quantity>;
  /** A preview (the generate form's): nothing to rename or generate, and the links go nowhere. */
  readonly preview?: boolean;
  showSystemBody(body: string | null): void;
  loadSystem(id: string): void;
}

/** The cards' source for the body a resolve asked for (`system.focus`). */
export function previewSource(result: ResolvedSystem, changed: ReadonlySet<Quantity>): CardSource {
  const focus = result.system.focus;
  return {
    system: result.system,
    systemId: null,
    systemNames: {},
    systemEvicted: [],
    systemBody: focus,
    shownSystemBody: result.system.bodies.find((b) => b.id === focus) ?? null,
    shownExtras: (focus && result.extras[focus]) || null,
    meta: null,
    stats: null,
    rivers: null,
    changed,
    preview: true,
    // A preview has one body shown and nothing to open.
    showSystemBody: () => {},
    loadSystem: () => {},
  };
}
