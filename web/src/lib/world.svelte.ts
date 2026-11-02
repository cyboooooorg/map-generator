// One world shown in a viewer pane: its metadata, decoded layers, surface
// statistics and rivers, loaded in that order (each step can take a while
// on large maps, and the earlier ones are useful on their own).
import { api } from './api/client';
import type { WorldMetadata } from './api/types/WorldMetadata';
import { loadLayers, type WorldLayers } from './layers';
import { RiverIndex } from './rivers';
import { worldStats, type WorldStats } from './stats';

export class LoadedWorld {
  id = $state<string | null>(null);
  // Raw state: typed arrays and large JSON must not be proxied.
  meta = $state.raw<WorldMetadata | null>(null);
  layers = $state.raw<WorldLayers | null>(null);
  stats = $state.raw<WorldStats | null>(null);
  rivers = $state.raw<RiverIndex | null>(null);
  error = $state<string | null>(null);
  private generation = 0;

  /** Loads a world, or clears the pane with null. Earlier loads are abandoned. */
  load(id: string | null): void {
    this.generation += 1;
    const generation = this.generation;
    const current = () => generation === this.generation;
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
      const meta = await api.metadata(id);
      if (!current()) return;
      this.meta = meta;
      const layers = await loadLayers(id, meta);
      if (!current()) return;
      this.layers = layers;
      this.stats = worldStats(layers.biomes, layers.width, layers.height, meta.biomes, meta.circumference_km);
      if (meta.rivers) {
        const file = await api.rivers(id, meta.rivers);
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
