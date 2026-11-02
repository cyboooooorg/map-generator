<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { ApiFailure, api, worldFile } from '../lib/api/client';
  import type { WorldMetadata } from '../lib/api/types/WorldMetadata';
  import type { LonLat } from '../lib/geo';
  import type { WorldLayers } from '../lib/layers';
  import { cssGradient, RAMPS } from '../lib/viewer/colormaps';
  import { PlanetView, type ViewMode } from '../lib/viewer/PlanetView';
  import type { Layer } from '../lib/viewer/shaders';

  interface Props {
    /** World to show, or null for an empty view. */
    worldId: string | null;
    /** Its revision (modification time), which keys the cached detail tiles. */
    revision: number | null;
    /** Its metadata, once loaded. */
    meta: WorldMetadata | null;
    /** Its decoded layers, once loaded: needed by every display except biomes. */
    layers: WorldLayers | null;
    onPick: (point: LonLat | null) => void;
  }

  let { worldId, revision, meta, layers, onPick }: Props = $props();

  const LAYERS: { id: Layer; label: string }[] = [
    { id: 'biomes', label: 'Biomes' },
    { id: 'elevation', label: 'Elevation' },
    { id: 'temperature', label: 'Temperature' },
    { id: 'moisture', label: 'Moisture' },
    { id: 'flow', label: 'Rivers' },
  ];
  const SEASONS = ['Spring equinox', 'Summer solstice', 'Autumn equinox', 'Winter solstice'];

  let canvas: HTMLCanvasElement;
  let view = $state<PlanetView | null>(null);
  let mode = $state<ViewMode>('globe');
  let layer = $state<Layer>('biomes');
  let relief = $state(0);
  let graticule = $state(false);
  let atmosphere = $state(true);
  let sun = $state(false);
  let season = $state(0.25);
  let sunLongitude = $state(0);
  let hexOn = $state(false);
  let hexColumns = $state(64);
  let hexBusy = $state(false);
  let loading = $state(false);
  let error = $state<string | null>(null);
  // Folded by default on narrow screens, where it would hide the planet.
  let panelOpen = $state(window.matchMedia('(min-width: 900px)').matches);

  const dataReady = $derived(layers !== null);
  const seasonLabel = $derived(SEASONS[Math.round(season * 4) % 4] ?? '');

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
    hexOn = false;
    current
      .setTexture(worldFile(id, 'world.png'))
      .catch((e: unknown) => (error = `Could not load the map: ${String(e)}`))
      .finally(() => (loading = false));
  });

  $effect(() => {
    view?.setData(layers, meta ? meta.sea_level + meta.preset.rules.waterline : 0);
  });
  $effect(() => {
    view?.setDetail(
      worldId && meta && revision !== null ? { id: worldId, revision, baseWidth: meta.width } : null,
    );
  });
  $effect(() => view?.setMode(mode));
  $effect(() => view?.setLayer(dataReady ? layer : 'biomes'));
  $effect(() => view?.setRelief(dataReady ? relief : 0));
  $effect(() => view?.setGraticule(graticule));
  $effect(() => view?.setAtmosphere(atmosphere));
  $effect(() => {
    view?.setLighting(
      sun
        ? { kind: 'sun', season, longitudeDeg: sunLongitude, axialTiltDeg: meta?.axial_tilt_deg ?? 23.44 }
        : { kind: 'headlight' },
    );
  });

  // The hex grid is computed by the server on demand, for the chosen column count.
  $effect(() => {
    const current = view;
    const id = worldId;
    const columns = hexColumns;
    if (!current || !id || !hexOn) {
      current?.setHex(null);
      return;
    }
    let cancelled = false;
    hexBusy = true;
    api
      .hex(id, { columns })
      .then((map) => {
        if (!cancelled) current.setHex(map);
      })
      .catch((e: unknown) => {
        if (!cancelled) error = e instanceof ApiFailure ? e.message : String(e);
      })
      .finally(() => {
        if (!cancelled) hexBusy = false;
      });
    return () => {
      cancelled = true;
    };
  });
</script>

<div class="viewer">
  <canvas bind:this={canvas}></canvas>

  <div class="panel" aria-label="Display">
    <div class="row">
      <div class="segmented">
        <button class:active={mode === 'globe'} onclick={() => (mode = 'globe')}>Globe</button>
        <button class:active={mode === 'map'} onclick={() => (mode = 'map')}>Map</button>
      </div>
      <button class="fold" onclick={() => (panelOpen = !panelOpen)} aria-expanded={panelOpen}>
        {panelOpen ? 'Hide' : 'Display'}
      </button>
    </div>

    {#if panelOpen}
      <label>
        <span>Layer</span>
        <select bind:value={layer} disabled={!dataReady}>
          {#each LAYERS as option (option.id)}
            <option value={option.id}>{option.label}</option>
          {/each}
        </select>
      </label>
      <label>
        <span>Relief</span>
        <input type="range" min="0" max="1" step="0.05" bind:value={relief} disabled={!dataReady} />
      </label>
      {#if !dataReady && worldId}
        <p class="muted">Loading the raw data…</p>
      {/if}

      <label class="check"><input type="checkbox" bind:checked={sun} /> Sunlight</label>
      {#if sun}
        <label>
          <span title="Sun declination follows the axial tilt">Season</span>
          <input type="range" min="0" max="1" step="0.01" bind:value={season} />
        </label>
        <p class="muted">{seasonLabel}</p>
        <label>
          <span>Sun longitude</span>
          <input type="range" min="0" max="360" step="1" bind:value={sunLongitude} />
        </label>
      {/if}
      <label class="check"><input type="checkbox" bind:checked={atmosphere} /> Atmosphere</label>
      <label class="check"><input type="checkbox" bind:checked={graticule} /> Grid</label>
      <label class="check">
        <input type="checkbox" bind:checked={hexOn} disabled={!worldId} />
        Hexes
        {#if hexBusy}<small class="muted">computing…</small>{/if}
      </label>
      {#if hexOn}
        <label>
          <span>Columns</span>
          <input type="range" min="16" max="256" step="4" bind:value={hexColumns} />
          <small class="value">{hexColumns}</small>
        </label>
      {/if}
    {/if}
  </div>

  {#if dataReady && layer !== 'biomes'}
    {@const ramp = RAMPS[layer]}
    <div class="legend" aria-label="Legend">
      <strong>{ramp.label}</strong>
      <div class="bar" style:background={cssGradient(layer)}></div>
      <div class="ends"><span>{ramp.min}</span><span>{ramp.max}</span></div>
    </div>
  {/if}

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
  .panel {
    position: absolute;
    top: 12px;
    right: 12px;
    width: 240px;
    display: grid;
    gap: 8px;
    background: var(--panel);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 10px;
  }
  .row {
    display: flex;
    justify-content: space-between;
    gap: 8px;
  }
  .panel label {
    display: grid;
    grid-template-columns: 92px 1fr auto;
    align-items: center;
    gap: 6px;
  }
  .panel label.check {
    display: flex;
    gap: 6px;
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
  .muted {
    margin: 0;
    color: var(--muted);
    font-size: 0.85rem;
  }
  .value {
    font-variant-numeric: tabular-nums;
    color: var(--muted);
  }
  .legend {
    position: absolute;
    right: 12px;
    bottom: 12px;
    width: 240px;
    background: var(--panel);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 8px 10px;
    display: grid;
    gap: 4px;
  }
  .bar {
    height: 10px;
    border-radius: 3px;
  }
  .ends {
    display: flex;
    justify-content: space-between;
    color: var(--muted);
    font-size: 0.8rem;
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
