<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { worldFile } from '../lib/api/client';
  import type { LonLat } from '../lib/geo';
  import { PlanetView, type ViewMode } from '../lib/viewer/PlanetView';

  interface Props {
    /** World to show, or null for an empty view. */
    worldId: string | null;
    onPick: (point: LonLat | null) => void;
  }

  let { worldId, onPick }: Props = $props();

  let canvas: HTMLCanvasElement;
  let view = $state<PlanetView | null>(null);
  let mode = $state<ViewMode>('globe');
  let graticule = $state(false);
  let loading = $state(false);
  let error = $state<string | null>(null);

  onMount(() => {
    try {
      view = new PlanetView(canvas);
      view.onPick = (point) => onPick(point);
    } catch (e) {
      error = `WebGL is not available: ${String(e)}`;
    }
  });

  onDestroy(() => view?.dispose());

  $effect(() => {
    const current = view;
    const id = worldId;
    if (!current || !id) {
      return;
    }
    loading = true;
    error = null;
    current
      .setTexture(worldFile(id, 'world.png'))
      .catch((e: unknown) => (error = `Could not load the map: ${String(e)}`))
      .finally(() => (loading = false));
  });

  $effect(() => view?.setMode(mode));
  $effect(() => view?.setGraticule(graticule));
</script>

<div class="viewer">
  <canvas bind:this={canvas}></canvas>

  <div class="toolbar" role="toolbar" aria-label="View">
    <div class="segmented">
      <button class:active={mode === 'globe'} onclick={() => (mode = 'globe')}>Globe</button>
      <button class:active={mode === 'map'} onclick={() => (mode = 'map')}>Map</button>
    </div>
    <label class="toggle">
      <input type="checkbox" bind:checked={graticule} />
      Grid
    </label>
  </div>

  {#if !worldId}
    <p class="overlay">Select a world, or generate a new one.</p>
  {:else if loading}
    <p class="overlay">Loading…</p>
  {/if}
  {#if error}
    <p class="overlay error">{error}</p>
  {/if}
</div>

<style>
  .viewer {
    position: relative;
    height: 100%;
    min-height: 0;
  }
  canvas {
    display: block;
    width: 100%;
    height: 100%;
    cursor: grab;
  }
  canvas:active {
    cursor: grabbing;
  }
  .toolbar {
    position: absolute;
    top: 12px;
    right: 12px;
    display: flex;
    gap: 12px;
    align-items: center;
  }
  .segmented {
    display: flex;
    border: 1px solid var(--border);
    border-radius: 6px;
    overflow: hidden;
  }
  .segmented button {
    border: none;
    border-radius: 0;
  }
  .segmented button.active {
    background: var(--accent);
    color: var(--accent-text);
  }
  .toggle {
    display: flex;
    gap: 6px;
    align-items: center;
    background: var(--panel);
    padding: 6px 10px;
    border-radius: 6px;
    border: 1px solid var(--border);
  }
  .overlay {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    margin: 0;
    pointer-events: none;
    color: var(--muted);
  }
  .overlay.error {
    color: var(--danger);
  }
</style>
