// One world shown in a viewer pane: its metadata, decoded layers, surface
// statistics and rivers, loaded in that order (each step can take a while
// on large maps, and the earlier ones are useful on their own).
import { t } from './i18n/index.svelte';
import { api, systemBodyId } from './api/client';
import type { System } from './api/types/System';
import type { WorldMetadata } from './api/types/WorldMetadata';
import { epochFile } from './epochs';
import { loadEpochLayers, loadLayers, type WorldLayers } from './layers';
import type { LoadProgress } from './progress';
import { RiverIndex } from './rivers';
import type { WorldStats } from './stats';

export class LoadedWorld {
  /**
   * The body shown, when it is generated: `systems/<system>/bodies/<body>`
   * (see systemBodyId). `meta` and the layers are this body's.
   */
  id = $state<string | null>(null);
  /** The planet of the body shown (itself, or a moon's planet), as an id like `id`. */
  worldId = $state<string | null>(null);
  /** The world's metadata (its star and moons), whichever body is shown. */
  worldMeta = $state.raw<WorldMetadata | null>(null);
  /** The world's biome statistics and rivers, whichever body is shown. */
  worldStats = $state.raw<WorldStats | null>(null);
  worldRivers = $state.raw<RiverIndex | null>(null);
  // Raw state: typed arrays and large JSON must not be proxied.
  meta = $state.raw<WorldMetadata | null>(null);
  layers = $state.raw<WorldLayers | null>(null);
  stats = $state.raw<WorldStats | null>(null);
  rivers = $state.raw<RiverIndex | null>(null);
  error = $state<string | null>(null);
  /** How far the download and decoding of the layers have got, while they load. */
  progress = $state.raw<LoadProgress | null>(null);
  /**
   * The epoch the layers show, in Ma before present (negative in the
   * future; see lib/epochs.ts): `layers` holds its elevation and biomes
   * once they have loaded, the present's other layers, and `stats` stay
   * the present's.
   */
  epochMa = $state(0);
  /** Whether the layers of the epoch asked for are loading. */
  epochLoading = $state(false);
  epochError = $state<string | null>(null);
  /**
   * The planetary system shown (docs/system-spec.md), when one is: its
   * file, the names given to its bodies, and the body shown (`c`, `c-1`).
   * A body not generated yet has no metadata nor layers: the view draws
   * its preview.
   */
  system = $state.raw<System | null>(null);
  systemId = $state<string | null>(null);
  systemNames = $state.raw<Record<string, string>>({});
  systemBody = $state<string | null>(null);
  /**
   * What the viewer shows: the body (`planet`), or the star and its system
   * round the body's planet. A system opens on its star unless a body is
   * asked for.
   */
  subject = $state<'planet' | 'star'>('planet');
  /**
   * The planets of the system shown whose cards are pinned in the star
   * view, by body id, in pinning order. They last for the session while
   * the same system is shown (opening a planet and coming back keeps
   * them); another system or a lone world clears them.
   */
  pinnedPlanets = $state<string[]>([]);
  /** The body of the system shown, from its file. */
  shownSystemBody = $derived(this.system?.bodies.find((b) => b.id === this.systemBody) ?? null);
  private systemController: AbortController | null = null;
  /** The layers as loaded, at the present. */
  private present: WorldLayers | null = null;
  private revision: number | null = null;
  /** Aborts the current load: its downloads and its decoding worker. */
  private controller: AbortController | null = null;
  private epochController: AbortController | null = null;

  /**
   * Loads a world, or clears the pane with null. Earlier loads are aborted.
   * Loading the id shown again reloads it (a world regenerated in place).
   */
  load(id: string | null): void {
    this.systemController?.abort();
    this.system = null;
    this.systemId = null;
    this.systemBody = null;
    this.systemNames = {};
    this.subject = 'planet';
    this.pinnedPlanets = [];
    this.worldId = id;
    this.worldMeta = null;
    this.worldStats = null;
    this.worldRivers = null;
    this.loadBody(id);
  }

  /**
   * Loads system `id` and shows its body `body`, or the body it was asked
   * around, else its first generated planet, else its first planet.
   * Loading it again (after a generation or a rename) keeps the body shown.
   */
  loadSystem(id: string, body: string | null = null): void {
    this.systemController?.abort();
    const controller = new AbortController();
    this.systemController = controller;
    const again = this.systemId === id;
    if (!again) {
      this.subject = body ? 'planet' : 'star';
      this.pinnedPlanets = [];
      // Nothing of the system left may stand under the new id: its file,
      // names, body and layers would be read as the new system's until
      // its own file arrives (moon ids requested under the wrong system).
      this.system = null;
      this.systemNames = {};
      this.systemBody = null;
      this.worldMeta = null;
      this.worldStats = null;
      this.worldRivers = null;
      this.loadBody(null);
    }
    this.systemId = id;
    this.worldId = null;
    (async () => {
      const { system, names } = await fetchSystem(id, controller.signal);
      if (controller.signal.aborted) return;
      this.system = system;
      this.systemNames = names;
      const wanted = body ?? (again ? this.systemBody : null);
      const known = (b: string | null | undefined) => (b && system.bodies.some((x) => x.id === b) ? b : null);
      const first =
        known(wanted) ??
        known(system.focus) ??
        system.bodies.find((b) => b.kind === 'planet' && b.generated)?.id ??
        system.bodies.find((b) => b.kind === 'planet')?.id ??
        null;
      const shown = this.systemBody;
      const was = shown ? system.bodies.find((b) => b.id === shown) : undefined;
      // Kept loaded when it is the same body and still as generated.
      this.systemBody = null;
      this.showSystemBody(first, again && shown === first && was?.generated === true && this.meta !== null);
    })().catch((e: unknown) => {
      if (!controller.signal.aborted) this.error = t.world.loadFailed({ id, error: String(e) });
    });
  }

  /**
   * Shows body `body` of the system loaded: its maps when it is generated,
   * its preview otherwise. `keep` leaves its layers as they are.
   */
  showSystemBody(body: string | null, keep = false): void {
    const system = this.system;
    if (!system || !this.systemId || body === this.systemBody) return;
    this.systemBody = body;
    const entry = system.bodies.find((b) => b.id === body);
    const planet = entry?.parent ?? entry?.id ?? null;
    this.worldId = planet ? systemBodyId(this.systemId, planet) : null;
    if (keep) return;
    this.worldMeta = null;
    this.worldStats = null;
    this.worldRivers = null;
    this.loadBody(entry?.generated ? systemBodyId(this.systemId, entry.id) : null);
  }

  private loadBody(id: string | null): void {
    this.controller?.abort();
    this.epochController?.abort();
    this.present = null;
    this.epochLoading = false;
    this.epochError = null;
    const controller = new AbortController();
    this.controller = controller;
    const { signal } = controller;
    const current = () => !signal.aborted;
    this.id = id;
    this.meta = null;
    this.layers = null;
    this.stats = null;
    this.rivers = null;
    this.error = null;
    this.progress = null;
    if (!id) {
      return;
    }
    (async () => {
      const meta = await api.metadata(id, signal);
      if (!current()) return;
      this.meta = meta;
      if (id === this.worldId) {
        this.worldMeta = meta;
      }
      const { layers, stats } = await loadLayers(id, meta, signal, (progress) => {
        if (current()) this.progress = progress;
      });
      if (!current()) return;
      this.progress = null;
      this.present = layers;
      this.layers = layers;
      this.stats = stats;
      if (id === this.worldId) {
        this.worldStats = stats;
      }
      // An epoch asked for before the layers arrived.
      this.showEpoch();
      if (meta.rivers) {
        const file = await api.rivers(id, meta.rivers, signal);
        if (!current()) return;
        this.rivers = new RiverIndex(file, meta.circumference_km);
        if (id === this.worldId) {
          this.worldRivers = this.rivers;
        }
      }
    })().catch((e: unknown) => {
      if (current()) {
        this.progress = null;
        this.error = t.world.loadFailed({ id, error: String(e) });
      }
    });
  }

  /**
   * Shows the layers at `epochMa` (0: the present), loaded from the server
   * for world revision `revision`. Called before the layers have loaded,
   * it applies once they arrive. A newer call cancels this one.
   */
  setEpoch(epochMa: number, revision: number | null): void {
    if (epochMa === this.epochMa && revision === this.revision && !this.epochError) {
      return;
    }
    this.epochMa = epochMa;
    this.revision = revision;
    this.showEpoch();
  }

  private showEpoch(): void {
    this.epochController?.abort();
    this.epochController = null;
    this.epochError = null;
    const { id, meta, present } = this;
    const epochMa = this.epochMa;
    if (!id || !meta || !present) {
      this.epochLoading = false;
      return;
    }
    if (epochMa === 0) {
      this.epochLoading = false;
      this.layers = present;
      return;
    }
    const controller = new AbortController();
    this.epochController = controller;
    this.epochLoading = true;
    const urls = {
      elevation: epochFile(id, epochMa, 'elevation.png', this.revision),
      biomes: epochFile(id, epochMa, 'biomes.png', this.revision),
    };
    loadEpochLayers(urls, meta, present, controller.signal)
      .then((layers) => {
        if (controller.signal.aborted) return;
        this.layers = layers;
      })
      .catch((e: unknown) => {
        if (controller.signal.aborted) return;
        this.epochError = t.epochs.failed({ error: String(e) });
      })
      .finally(() => {
        if (!controller.signal.aborted) this.epochLoading = false;
      });
  }
}

/**
 * A system and the names given to its bodies (`GET /api/systems/{id}`),
 * or its file alone (`/systems/{id}/system.json`) when the route is not
 * there.
 */
async function fetchSystem(
  id: string,
  signal: AbortSignal,
): Promise<{ system: System; names: Record<string, string> }> {
  const response = await fetch(`/api/systems/${encodeURIComponent(id)}`, { signal, cache: 'no-cache' });
  if (response.ok) {
    const body = (await response.json()) as { system: System; names?: Record<string, string> };
    return { system: body.system, names: body.names ?? {} };
  }
  const file = await fetch(`/systems/${encodeURIComponent(id)}/system.json`, { signal, cache: 'no-cache' });
  if (!file.ok) throw new Error(`${file.status} ${file.statusText}`);
  return { system: (await file.json()) as System, names: {} };
}
