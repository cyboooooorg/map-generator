// One world shown in a viewer pane: its metadata, decoded layers, surface
// statistics and rivers, loaded in that order (each step can take a while
// on large maps, and the earlier ones are useful on their own).
import { t } from './i18n/index.svelte';
import { api } from './api/client';
import type { WorldMetadata } from './api/types/WorldMetadata';
import { epochFile } from './epochs';
import { bodyId, sameBody, WORLD_BODY, type Body } from './moons';
import { loadEpochLayers, loadLayers, type WorldLayers } from './layers';
import type { LoadProgress } from './progress';
import { RiverIndex } from './rivers';
import type { WorldStats } from './stats';

export class LoadedWorld {
  /**
   * The body shown: the world's id, or one of its moons' (`<world>/moons/<n>`,
   * see lib/moons.ts). `meta` and the layers are this body's.
   */
  id = $state<string | null>(null);
  /** The world loaded, whichever body is shown. */
  worldId = $state<string | null>(null);
  /** The body of the world's system shown (see lib/moons.ts). */
  body = $state.raw<Body>(WORLD_BODY);
  /** The moon shown (from 1, nearest first), or null for any other body. */
  moon = $derived(this.body.kind === 'moon' ? this.body.n : null);
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
    this.worldId = id;
    this.body = WORLD_BODY;
    this.worldMeta = null;
    this.worldStats = null;
    this.worldRivers = null;
    this.loadBody(id);
  }

  /**
   * Shows moon `n` (from 1) of the world loaded, or the world itself with
   * null. The world's metadata stays.
   */
  showMoon(n: number | null): void {
    this.showBody(n === null ? WORLD_BODY : { kind: 'moon', n });
  }

  /** Shows a body of the world's system (see lib/moons.ts). The world's metadata stays. */
  showBody(body: Body): void {
    if (sameBody(body, this.body) || !this.worldId) {
      return;
    }
    this.body = body;
    this.loadBody(bodyId(this.worldId, body));
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
