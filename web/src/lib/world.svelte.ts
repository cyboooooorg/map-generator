// One world shown in a viewer pane: its metadata, decoded layers, surface
// statistics and rivers, loaded in that order (each step can take a while
// on large maps, and the earlier ones are useful on their own).
import { api } from './api/client';
import type { WorldMetadata } from './api/types/WorldMetadata';
import { loadLayers, type WorldLayers } from './layers';
import { RiverIndex } from './rivers';
import type { WorldStats } from './stats';

export class LoadedWorld {
  id = $state<string | null>(null);
  // Raw state: typed arrays and large JSON must not be proxied.
  meta = $state.raw<WorldMetadata | null>(null);
  layers = $state.raw<WorldLayers | null>(null);
  stats = $state.raw<WorldStats | null>(null);
  rivers = $state.raw<RiverIndex | null>(null);
  error = $state<string | null>(null);
  /** Aborts the current load: its downloads and its decoding worker. */
  private controller: AbortController | null = null;

  /**
   * Loads a world, or clears the pane with null. Earlier loads are aborted.
   * Loading the id shown again reloads it (a world regenerated in place).
   */
  load(id: string | null): void {
    this.controller?.abort();
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
    if (!id) {
      return;
    }
    (async () => {
      const meta = await api.metadata(id, signal);
      if (!current()) return;
      this.meta = meta;
      const { layers, stats } = await loadLayers(id, meta, signal);
      if (!current()) return;
      this.layers = layers;
      this.stats = stats;
      if (meta.rivers) {
        const file = await api.rivers(id, meta.rivers, signal);
        if (!current()) return;
        this.rivers = new RiverIndex(file, meta.circumference_km);
      }
    })().catch((e: unknown) => {
      if (current()) {
        this.error = `Could not load ${id}: ${String(e)}`;
      }
    });
  }
}
