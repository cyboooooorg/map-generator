<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { ApiFailure, api, worldFile } from '../lib/api/client';
  import type { HexMap } from '../lib/api/types/HexMap';
  import { downloadJson } from '../lib/download';
  import { formatArea, formatShare, saveBlob } from '../lib/format';
  import type { LonLat } from '../lib/geo';
  import { coastline } from '../lib/sample';
  import { seaIsLiquid } from '../lib/sea';
  import { loadSetting, saveSetting } from '../lib/settings';
  import { toasts } from '../lib/toasts.svelte';
  import { cssGradient, RAMPS } from '../lib/viewer/colormaps';
  import { type CameraState, PlanetView, type ViewMode } from '../lib/viewer/PlanetView';
  import type { Layer } from '../lib/viewer/shaders';
  import type { LoadedWorld } from '../lib/world.svelte';

  interface Props {
    world: LoadedWorld;
    /** Its revision (modification time), which keys the cached detail tiles. */
    revision: number | null;
    /** Measured path to draw. */
    path: readonly LonLat[];
    measuring: boolean;
    /** Whether this viewer answers keyboard shortcuts (one viewer at a time). */
    keyboard: boolean;
    /** The hex grid shown, if any (for the point information). */
    hexMap?: HexMap | null;
    onPick: (point: LonLat | null) => void;
    onHover: (point: LonLat | null) => void;
    onToggleMeasure: () => void;
    /** Escape: close what the pane shows over the viewer. */
    onEscape: () => void;
    onCameraChange?: (state: CameraState) => void;
  }

  let {
    world,
    revision,
    path,
    measuring,
    keyboard,
    hexMap = $bindable(null),
    onPick,
    onHover,
    onToggleMeasure,
    onEscape,
    onCameraChange,
  }: Props = $props();

  type Needs = 'none' | 'extra' | 'floodplain';
  const LAYERS: { id: Layer; label: string; needs: Needs }[] = [
    { id: 'biomes', label: 'Biomes', needs: 'none' },
    { id: 'elevation', label: 'Elevation', needs: 'none' },
    { id: 'temperature', label: 'Temperature', needs: 'none' },
    { id: 'moisture', label: 'Moisture', needs: 'none' },
    { id: 'flow', label: 'Rivers', needs: 'none' },
    { id: 'volcanic', label: 'Volcanic zone', needs: 'extra' },
    { id: 'coast', label: 'Distance to the sea', needs: 'extra' },
    { id: 'floodplain', label: 'Floodplains', needs: 'floodplain' },
  ];
  const SEASONS = ['Spring equinox', 'Summer solstice', 'Autumn equinox', 'Winter solstice'];
  /** One year of the season animation, in seconds. */
  const YEAR_SECONDS = 12;
  /** mapgen::hex::DEFAULT_RIVER_FLOW_KM2, on the log10 slider. */
  const DEFAULT_RIVER_LOG = Math.log10(500_000);

  /** Display preferences kept between visits. */
  const saved = loadSetting('display', {
    mode: 'globe' as string,
    layer: 'biomes' as string,
    relief: 0,
    graticule: false,
    atmosphere: true,
    sun: false,
    season: 0.25,
    sunLongitude: 0,
    hexColumns: 64,
    hexRiverLog: DEFAULT_RIVER_LOG,
    legendOpen: true,
  });

  let canvas: HTMLCanvasElement;
  let view = $state<PlanetView | null>(null);
  let mode = $state<ViewMode>(saved.mode === 'map' ? 'map' : 'globe');
  let layer = $state<Layer>(LAYERS.some((l) => l.id === saved.layer) ? (saved.layer as Layer) : 'biomes');
  let relief = $state(saved.relief);
  let graticule = $state(saved.graticule);
  let atmosphere = $state(saved.atmosphere);
  let sun = $state(saved.sun);
  let season = $state(saved.season);
  let sunLongitude = $state(saved.sunLongitude);
  let playing = $state(false);
  let hexOn = $state(false);
  let hexColumns = $state(saved.hexColumns);
  let hexRiverLog = $state(saved.hexRiverLog);
  let hexBusy = $state(false);
  // Narrow screens start with the legend folded too: it would cover the planet.
  const wide = window.matchMedia('(min-width: 900px)').matches;
  let legendOpen = $state(saved.legendOpen && wide);
  let hovered = $state<number | null>(null);
  let pinned = $state<number | null>(null);
  let loading = $state(false);
  let error = $state<string | null>(null);
  let showHelp = $state(false);
  // Folded by default on narrow screens, where it would hide the planet.
  let panelOpen = $state(wide);

  const worldId = $derived(world.id);
  const meta = $derived(world.meta);
  const layers = $derived(world.layers);
  const dataReady = $derived(layers !== null);
  function available(needs: Needs): boolean {
    return (
      needs === 'none' ||
      (needs === 'extra' && layers?.extra != null) ||
      (needs === 'floodplain' && layers?.extra != null && meta?.layers.floodplain != null)
    );
  }
  /** The layer drawn: the chosen one when this world has it, the biomes otherwise. */
  const shown = $derived.by((): Layer => {
    const option = LAYERS.find((l) => l.id === layer);
    return dataReady && option && available(option.needs) ? layer : 'biomes';
  });
  const declination = $derived((meta?.axial_tilt_deg ?? 23.44) * Math.sin(2 * Math.PI * season));
  const seasonLabel = $derived.by(() => {
    const quarter = season * 4;
    const nearest = Math.round(quarter) % 4;
    return Math.abs(quarter - Math.round(quarter)) < 0.12
      ? (SEASONS[nearest] ?? '')
      : `Day ${Math.round(season * 365) % 365} of the year`;
  });
  const highlight = $derived(hovered ?? pinned);

  $effect(() => {
    saveSetting('display', {
      mode,
      layer,
      relief,
      graticule,
      atmosphere,
      sun,
      season,
      sunLongitude,
      hexColumns,
      hexRiverLog,
      // A legend folded for lack of room stays open on wide screens.
      legendOpen: wide ? legendOpen : saved.legendOpen,
    });
  });

  onMount(() => {
    try {
      view = new PlanetView(canvas);
      view.onPick = (point) => onPick(point);
      view.onHover = (point) => onHover(point);
      view.onCameraChange = (state) => onCameraChange?.(state);
    } catch (e) {
      error = `WebGL is not available: ${String(e)}`;
    }
  });

  onDestroy(() => view?.dispose());

  $effect(() => {
    const current = view;
    const id = worldId;
    pinned = null;
    hovered = null;
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
    view?.setData(layers, meta ? coastline(meta) : 0, meta ? seaIsLiquid(meta) : true);
  });
  $effect(() => {
    view?.setDetail(
      worldId && meta && revision !== null ? { id: worldId, revision, baseWidth: meta.width } : null,
    );
  });
  $effect(() => view?.setMode(mode));
  $effect(() => view?.setLayer(shown));
  $effect(() => view?.setRelief(dataReady ? relief : 0));
  $effect(() => view?.setGraticule(graticule));
  $effect(() => view?.setAtmosphere(atmosphere));
  $effect(() => view?.setHighlight(dataReady ? highlight : null));
  $effect(() => view?.setPath(path));
  $effect(() => {
    view?.setLighting(
      sun
        ? { kind: 'sun', season, longitudeDeg: sunLongitude, axialTiltDeg: meta?.axial_tilt_deg ?? 23.44 }
        : { kind: 'headlight' },
    );
  });

  // The season animation: one year every YEAR_SECONDS.
  $effect(() => {
    if (!playing || !sun) {
      return;
    }
    let last = performance.now();
    let frame = requestAnimationFrame(function step(now) {
      season = (season + (now - last) / 1000 / YEAR_SECONDS) % 1;
      last = now;
      frame = requestAnimationFrame(step);
    });
    return () => cancelAnimationFrame(frame);
  });

  // The hex grid is computed by the server on demand, for the chosen column
  // count and river threshold; slider moves are debounced.
  $effect(() => {
    const current = view;
    const id = worldId;
    const columns = hexColumns;
    const riverFlow = Math.round(10 ** hexRiverLog);
    if (!current || !id || !hexOn) {
      current?.setHex(null);
      hexMap = null;
      return;
    }
    let cancelled = false;
    const timer = setTimeout(() => {
      hexBusy = true;
      api
        .hex(id, { columns, river_flow_km2: riverFlow })
        .then((map) => {
          if (!cancelled) {
            current.setHex(map);
            hexMap = map;
          }
        })
        .catch((e: unknown) => {
          if (!cancelled) error = e instanceof ApiFailure ? e.message : String(e);
        })
        .finally(() => {
          if (!cancelled) hexBusy = false;
        });
    }, 300);
    return () => {
      cancelled = true;
      clearTimeout(timer);
    };
  });

  /** Back to the whole planet, north up. */
  export function resetView(): void {
    view?.resetView();
  }

  /** Shows the same place as another viewer. */
  export function applyCamera(state: CameraState): void {
    mode = state.mode;
    view?.setCamera(state);
  }

  /** Saves the current view as a PNG. */
  export async function screenshot(): Promise<void> {
    const blob = await view?.screenshot();
    if (blob && worldId) {
      saveBlob(blob, `${worldId}-${shown}-${mode}.png`);
    } else {
      toasts.error('The screenshot failed.');
    }
  }

  function downloadHex(): void {
    if (hexMap && worldId) {
      downloadJson(hexMap, `${worldId}-hex-${hexMap.columns}.json`);
    }
  }

  function selectLayer(index: number): void {
    const option = LAYERS[index];
    if (option && available(option.needs)) {
      layer = option.id;
    }
  }

  function handleKey(event: KeyboardEvent): void {
    if (!keyboard || event.ctrlKey || event.metaKey || event.altKey) {
      return;
    }
    const target = event.target as HTMLElement | null;
    if (target && (target.isContentEditable || ['INPUT', 'SELECT', 'TEXTAREA'].includes(target.tagName))) {
      return;
    }
    const key = event.key;
    let handled = true;
    if (key >= '1' && key <= '8' && key.length === 1) {
      selectLayer(Number(key) - 1);
    } else {
      switch (key.toLowerCase()) {
        case 'g':
          mode = 'globe';
          break;
        case 'm':
          mode = 'map';
          break;
        case 'r':
          resetView();
          break;
        case 'h':
          if (worldId) hexOn = !hexOn;
          break;
        case 'l':
          sun = !sun;
          break;
        case 's':
          if (sun) playing = !playing;
          break;
        case 'a':
          atmosphere = !atmosphere;
          break;
        case 'n':
          graticule = !graticule;
          break;
        case 'd':
          if (worldId) onToggleMeasure();
          break;
        case 'p':
          if (worldId) void screenshot();
          break;
        case '?':
          showHelp = !showHelp;
          break;
        case 'escape':
          if (showHelp) {
            showHelp = false;
          } else {
            pinned = null;
            onEscape();
          }
          break;
        default:
          handled = false;
      }
    }
    if (handled) {
      event.preventDefault();
    }
  }

  const SHORTCUTS: [string, string][] = [
    ['G / M', 'Globe / flat map'],
    ['1 – 8', 'Layer'],
    ['R', 'Reset the view'],
    ['H', 'Hexes'],
    ['L', 'Sunlight'],
    ['S', 'Play the seasons (with sunlight)'],
    ['A', 'Atmosphere'],
    ['N', 'Grid'],
    ['D', 'Measure distances'],
    ['P', 'Screenshot'],
    ['Esc', 'Close the point panel, stop measuring'],
    ['?', 'This help'],
  ];
</script>

<svelte:window onkeydown={handleKey} />

<div class="viewer">
  <canvas bind:this={canvas} class:measuring></canvas>

  <div class="toolbar" aria-label="Tools">
    <button onclick={resetView} disabled={!worldId} title="Reset the view (R)">Reset view</button>
    <button
      onclick={onToggleMeasure}
      disabled={!worldId}
      class:active={measuring}
      aria-pressed={measuring}
      title="Measure distances (D)">Measure</button
    >
    <button onclick={screenshot} disabled={!worldId} title="Save the view as a PNG (P)">Screenshot</button>
    <button onclick={() => (showHelp = !showHelp)} aria-expanded={showHelp} title="Keyboard shortcuts (?)">?</button>
  </div>

  <div class="panel" aria-label="Display">
    <div class="row">
      <div class="segmented">
        <button class:active={mode === 'globe'} onclick={() => (mode = 'globe')} title="Globe (G)">Globe</button>
        <button class:active={mode === 'map'} onclick={() => (mode = 'map')} title="Flat map (M)">Map</button>
      </div>
      <button class="fold" onclick={() => (panelOpen = !panelOpen)} aria-expanded={panelOpen}>
        {panelOpen ? 'Hide' : 'Display'}
      </button>
    </div>

    {#if panelOpen}
      <label>
        <span>Layer</span>
        <select bind:value={layer} disabled={!dataReady}>
          {#each LAYERS as option, i (option.id)}
            <option value={option.id} disabled={dataReady && !available(option.needs)}
              >{i + 1}. {option.label}{!dataReady || available(option.needs) ? '' : ' (not in this world)'}</option
            >
          {/each}
        </select>
      </label>
      <label>
        <span>Relief</span>
        <input type="range" min="0" max="1" step="0.05" bind:value={relief} disabled={!dataReady} />
      </label>

      <label class="check"><input type="checkbox" bind:checked={sun} /> Sunlight</label>
      {#if sun}
        <label>
          <span title="Sun declination follows the axial tilt">Season</span>
          <input type="range" min="0" max="1" step="0.005" bind:value={season} />
          <button class="small" onclick={() => (playing = !playing)} aria-pressed={playing} title="Play the seasons (S)"
            >{playing ? 'Pause' : 'Play'}</button
          >
        </label>
        <p class="muted">
          {seasonLabel} · sun over {Math.abs(declination).toFixed(1)}° {declination >= 0 ? 'N' : 'S'}
        </p>
        <label>
          <span>Sun longitude</span>
          <input type="range" min="0" max="360" step="1" bind:value={sunLongitude} />
          <small class="value">{sunLongitude}°</small>
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
        <label>
          <span title="Land hexes crossed by a river draining more than this get a river dot">River from</span>
          <input type="range" min="3" max="7" step="0.1" bind:value={hexRiverLog} />
          <small class="value">{formatArea(10 ** hexRiverLog)}</small>
        </label>
        <button class="small" onclick={downloadHex} disabled={!hexMap}>Download hex JSON</button>
      {/if}
    {/if}
  </div>

  {#if dataReady && shown !== 'biomes'}
    {@const ramp = RAMPS[shown]}
    <div class="legend" aria-label="Legend">
      <strong>{ramp.label}</strong>
      <div class="bar" style:background={cssGradient(shown)}></div>
      <div class="ends">
        <span>{ramp.min}</span>
        <span>{shown === 'coast' && layers?.extra ? `${layers.extra.coastMaxKm} km (√ scale)` : ramp.max}</span>
      </div>
    </div>
  {:else if shown === 'biomes' && world.stats}
    <div class="legend biomes" aria-label="Biome legend">
      <button class="legend-title" onclick={() => (legendOpen = !legendOpen)} aria-expanded={legendOpen}>
        <strong>Biomes</strong>
        <small class="muted">{legendOpen ? 'hide' : `${world.stats.shares.length}, show`}</small>
      </button>
      {#if legendOpen}
        <ul onpointerleave={() => (hovered = null)}>
          {#each world.stats.shares as entry (entry.biome.id)}
            <li>
              <button
                class:pinned={pinned === entry.biome.id}
                onpointerenter={() => (hovered = entry.biome.id)}
                onfocus={() => (hovered = entry.biome.id)}
                onblur={() => (hovered = null)}
                onclick={() => (pinned = pinned === entry.biome.id ? null : entry.biome.id)}
                aria-pressed={pinned === entry.biome.id}
                title="Hover to highlight, click to keep it highlighted"
              >
                <span class="swatch" style:background={entry.biome.color}></span>
                <span class="name">{entry.biome.name}</span>
                <span class="value">{formatShare(entry.share)}</span>
              </button>
            </li>
          {/each}
        </ul>
      {/if}
    </div>
  {/if}

  {#if showHelp}
    <div class="help" role="dialog" aria-label="Keyboard shortcuts">
      <header>
        <strong>Keyboard shortcuts</strong>
        <button class="close" onclick={() => (showHelp = false)} aria-label="Close">×</button>
      </header>
      <dl>
        {#each SHORTCUTS as [key, action] (key)}
          <dt><kbd>{key}</kbd></dt>
          <dd>{action}</dd>
        {/each}
      </dl>
    </div>
  {/if}

  {#if !worldId}
    <p class="overlay">Select a world, or generate a new one.</p>
  {:else if loading}
    <p class="overlay">Loading…</p>
  {:else if !dataReady && !world.error}
    <p class="chip">Loading the raw data…</p>
  {/if}
  {#if error || world.error}
    <p class="overlay error">{error ?? world.error}</p>
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
  canvas.measuring {
    cursor: crosshair;
  }
  .toolbar {
    position: absolute;
    top: 12px;
    left: 12px;
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    /* Leave room for the display panel on the right. */
    max-width: calc(100% - 280px);
  }
  .toolbar button {
    background: var(--panel);
    padding: 4px 10px;
  }
  .toolbar button.active {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--accent-text);
  }
  .panel {
    position: absolute;
    top: 12px;
    right: 12px;
    width: 240px;
    /* Scroll inside the panel rather than overflow a short viewer. */
    max-height: calc(100% - 24px);
    overflow-y: auto;
    display: grid;
    align-content: start;
    gap: 8px;
    background: var(--panel);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 10px;
    z-index: 1;
  }
  .row {
    display: flex;
    justify-content: space-between;
    gap: 8px;
  }
  .panel label {
    display: grid;
    grid-template-columns: 92px minmax(0, 1fr) auto;
    align-items: center;
    gap: 6px;
  }
  .panel label > select {
    min-width: 0;
  }
  .panel label.check {
    display: flex;
    gap: 6px;
  }
  .small {
    padding: 2px 8px;
    font-size: 0.85rem;
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
  .legend-title {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    border: none;
    background: none;
    padding: 0;
  }
  .legend ul {
    list-style: none;
    margin: 0;
    padding: 0;
    overflow-y: auto;
    max-height: min(40vh, 360px);
  }
  .legend li button {
    display: grid;
    grid-template-columns: 14px minmax(0, 1fr) auto;
    gap: 8px;
    align-items: center;
    width: 100%;
    border: 1px solid transparent;
    background: none;
    padding: 2px 4px;
    text-align: left;
  }
  .legend li button.pinned {
    border-color: var(--accent);
  }
  .swatch {
    width: 14px;
    height: 14px;
    border-radius: 3px;
    border: 1px solid var(--border);
  }
  .name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
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
  .help {
    position: absolute;
    top: 52px;
    left: 12px;
    width: min(320px, calc(100% - 24px));
    background: var(--panel-solid);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 10px 14px;
    z-index: 2;
  }
  .help header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 6px;
  }
  .help dl {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 4px 12px;
    margin: 0;
  }
  .help dd {
    margin: 0;
  }
  kbd {
    font-family: ui-monospace, monospace;
    font-size: 0.85rem;
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 0 5px;
  }
  .close {
    border: none;
    background: none;
    font-size: 1.2rem;
    padding: 0 4px;
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
    padding: 0 24px;
    text-align: center;
  }
  .chip {
    position: absolute;
    left: 50%;
    bottom: 12px;
    transform: translateX(-50%);
    margin: 0;
    padding: 4px 12px;
    border-radius: 999px;
    background: var(--panel);
    border: 1px solid var(--border);
    color: var(--muted);
    font-size: 0.85rem;
    pointer-events: none;
  }
  /* Narrow viewer: the tools go in a row under the display panel (which
     covers them when unfolded). */
  @container (max-width: 580px) {
    .toolbar {
      top: 64px;
      right: 12px;
      max-width: none;
    }
    .toolbar button {
      padding: 2px 8px;
      font-size: 0.85rem;
    }
  }
</style>
