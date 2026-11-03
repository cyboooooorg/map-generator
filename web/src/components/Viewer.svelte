<script lang="ts">
  import { onDestroy, onMount, untrack } from 'svelte';
  import { ApiFailure, api, worldFile } from '../lib/api/client';
  import type { HexMap } from '../lib/api/types/HexMap';
  import { linkedChanges, type LinkedDisplay } from '../lib/display';
  import { downloadJson } from '../lib/download';
  import { formatArea, formatShare, saveBlob, surfaceColour } from '../lib/format';
  import type { LonLat } from '../lib/geo';
  import type { LoadProgress } from '../lib/progress';
  import { coastline } from '../lib/sample';
  import { seaIsLiquid } from '../lib/sea';
  import { retryWhileBusy } from '../lib/retry';
  import { debouncedSetting, loadSetting } from '../lib/settings';
  import { altAllowed, SHORTCUT_HELP, shortcutFor, stepRelief } from '../lib/shortcuts';
  import { cloudTint, skyFromWorld } from '../lib/sky';
  import { toasts } from '../lib/toasts.svelte';
  import { cssGradient, RAMPS } from '../lib/viewer/colormaps';
  import { type CameraState, PlanetView, type ViewMode } from '../lib/viewer/PlanetView';
  import type { Layer } from '../lib/viewer/shaders';
  import type { LoadedWorld } from '../lib/world.svelte';
  import LoadingBar from './LoadingBar.svelte';

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
    /** `[` / `]`: show the previous (-1) or next (1) world of the list. */
    onStepWorld?: (direction: 1 | -1) => void;
    /** Called with the linked display settings whenever one changes (the main pane). */
    onDisplayChange?: (display: LinkedDisplay) => void;
    /** Settings to mirror ("Link display", a compare pane), or null to keep its own. */
    linkedDisplay?: LinkedDisplay | null;
    /**
     * Whether display changes are saved as the preferences. Only the main
     * viewer saves them: a compare pane starts from them but would otherwise
     * overwrite them with its own.
     */
    persistDisplay?: boolean;
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
    onStepWorld,
    onDisplayChange,
    linkedDisplay = null,
    persistDisplay = true,
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
  /** Wait before asking a server busy generating for the hex grid again, ms. */
  const HEX_RETRY_MS = 5000;

  /** Display preferences kept between visits. */
  const saved = loadSetting('display', {
    mode: 'globe' as string,
    layer: 'biomes' as string,
    relief: 0,
    graticule: false,
    referenceLines: true,
    contours: true,
    landContours: true,
    atmosphere: true,
    clouds: true,
    cloudDrift: false,
    sun: false,
    season: 0.25,
    sunLongitude: 0,
    hexColumns: 64,
    hexRiverLog: DEFAULT_RIVER_LOG,
    legendOpen: true,
    panelOpen: true,
  });

  let canvas: HTMLCanvasElement;
  let view = $state<PlanetView | null>(null);
  let mode = $state<ViewMode>(saved.mode === 'map' ? 'map' : 'globe');
  let layer = $state<Layer>(LAYERS.some((l) => l.id === saved.layer) ? (saved.layer as Layer) : 'biomes');
  let relief = $state(saved.relief);
  let graticule = $state(saved.graticule);
  let referenceLines = $state(saved.referenceLines);
  // `contours` keeps its saved name: it was the only (depth) contour toggle.
  let seaContours = $state(saved.contours);
  let landContours = $state(saved.landContours);
  let atmosphere = $state(saved.atmosphere);
  let clouds = $state(saved.clouds);
  let cloudDrift = $state(saved.cloudDrift);
  let sun = $state(saved.sun);
  let season = $state(saved.season);
  let sunLongitude = $state(saved.sunLongitude);
  let playing = $state(false);
  /** Whether the canvas has a size: hidden (narrow screens showing the list), nothing animates. */
  let canvasVisible = $state(true);
  let hexOn = $state(false);
  let hexColumns = $state(saved.hexColumns);
  let hexRiverLog = $state(saved.hexRiverLog);
  let hexBusy = $state(false);
  /** Whether the server answered that it is busy generating: the request is retried. */
  let hexServerBusy = $state(false);
  // Narrow screens fold the legend and the display panel, which would cover
  // the planet; wide ones open them as last chosen on a wide screen. Both
  // follow the window as it is resized across the limit.
  const wideQuery = window.matchMedia('(min-width: 900px)');
  let wide = $state(wideQuery.matches);
  /** The legend and panel states chosen on a wide screen, saved as the preferences. */
  let wideChoice = { legendOpen: saved.legendOpen, panelOpen: saved.panelOpen };
  let legendOpen = $state(saved.legendOpen && wideQuery.matches);
  let hovered = $state<number | null>(null);
  let pinned = $state<number | null>(null);
  let loading = $state(false);
  /** The base map's download and decoding, while it loads. */
  let mapProgress = $state.raw<LoadProgress | null>(null);
  let error = $state<string | null>(null);
  let showHelp = $state(false);
  let panelOpen = $state(saved.panelOpen && wideQuery.matches);

  $effect(() => {
    const change = (event: MediaQueryListEvent) => {
      wide = event.matches;
      legendOpen = wide && wideChoice.legendOpen;
      panelOpen = wide && wideChoice.panelOpen;
    };
    wideQuery.addEventListener('change', change);
    return () => wideQuery.removeEventListener('change', change);
  });

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

  // Debounced: the season animation and the sliders change these every frame.
  const display = debouncedSetting<typeof saved>('display', 300);
  $effect(() => {
    if (!persistDisplay) {
      return;
    }
    // Folded for lack of room is not a choice: only wide screens change it.
    if (wide) {
      wideChoice = { legendOpen, panelOpen };
    }
    display.save({
      mode,
      layer,
      relief,
      graticule,
      referenceLines,
      contours: seaContours,
      landContours,
      atmosphere,
      clouds,
      cloudDrift,
      sun,
      season,
      sunLongitude,
      hexColumns,
      hexRiverLog,
      ...wideChoice,
    });
  });

  /** The settings "Link display" mirrors, as this pane has them. */
  function currentLinked(): LinkedDisplay {
    return { layer, relief, sun, season, sunLongitude, atmosphere, clouds, graticule };
  }

  $effect(() => {
    const current = currentLinked();
    onDisplayChange?.(current);
  });

  // A compare pane with "Link display": follow the main pane's settings.
  $effect(() => {
    const source = linkedDisplay;
    if (!source) {
      return;
    }
    const changes = untrack(() => linkedChanges(currentLinked(), source));
    if (changes.layer !== undefined && LAYERS.some((l) => l.id === changes.layer)) layer = changes.layer as Layer;
    if (changes.relief !== undefined) relief = changes.relief;
    if (changes.sun !== undefined) sun = changes.sun;
    if (changes.season !== undefined) season = changes.season;
    if (changes.sunLongitude !== undefined) sunLongitude = changes.sunLongitude;
    if (changes.atmosphere !== undefined) atmosphere = changes.atmosphere;
    if (changes.clouds !== undefined) clouds = changes.clouds;
    if (changes.graticule !== undefined) graticule = changes.graticule;
  });

  onMount(() => {
    try {
      view = new PlanetView(canvas);
      view.onPick = (point) => onPick(point);
      view.onHover = (point) => onHover(point);
      view.onCameraChange = (state) => onCameraChange?.(state);
      canvasVisible = view.visible;
      view.onVisibilityChange = (visible) => (canvasVisible = visible);
    } catch (e) {
      error = `WebGL is not available: ${String(e)}`;
    }
  });

  onDestroy(() => {
    display.flush();
    view?.dispose();
  });

  /** The world whose base map is shown (or loading). */
  let textureOf: string | null = null;

  // The base map, reloaded when the world or its revision changes (a world
  // regenerated under the same id). A newer load cancels this one: its
  // result and its errors are dropped.
  $effect(() => {
    const current = view;
    const id = worldId;
    const rev = revision;
    // No revision: the world left the list, typically while it is being
    // regenerated in place. Keep its map until the new one is listed.
    if (rev === null && id !== null && id === textureOf) {
      loading = false;
      return;
    }
    pinned = null;
    hovered = null;
    if (!current || !id) {
      textureOf = null;
      loading = false;
      return;
    }
    textureOf = id;
    loading = true;
    mapProgress = null;
    error = null;
    hexOn = false;
    const controller = new AbortController();
    const progress = (p: LoadProgress) => {
      if (!controller.signal.aborted) mapProgress = p;
    };
    // The render without lines (format version 6), which the reference
    // lines and contours are drawn over; world.png, which has them baked
    // in, for older worlds.
    current
      .setTexture(worldFile(id, 'raw_data/surface.png', rev), controller.signal, progress)
      .catch((e: unknown) => {
        if (controller.signal.aborted) throw e;
        return current.setTexture(worldFile(id, 'world.png', rev), controller.signal, progress);
      })
      .catch((e: unknown) => {
        if (!controller.signal.aborted) error = `Could not load the map: ${String(e)}`;
      })
      .finally(() => {
        if (!controller.signal.aborted) {
          loading = false;
          mapProgress = null;
        }
      });
    return () => controller.abort();
  });

  $effect(() => {
    view?.setData(layers, meta ? coastline(meta) : 0, meta ? seaIsLiquid(meta) : true);
  });
  // The cloud layer of worlds that have one (format version 5 and later).
  $effect(() => {
    const current = view;
    const id = worldId;
    const file = meta?.layers.clouds?.file;
    if (!current) {
      return;
    }
    const controller = new AbortController();
    const url = id && file ? worldFile(id, `raw_data/${file}`, revision) : null;
    current
      .setClouds(url, meta ? cloudTint(meta) : [1, 1, 1], controller.signal)
      .catch((e: unknown) => {
        if (!controller.signal.aborted) console.warn('Could not load the clouds', e);
      });
    return () => controller.abort();
  });
  $effect(() => view?.setCloudsVisible(clouds));
  $effect(() => view?.setCloudsDrifting(cloudDrift));
  $effect(() => {
    view?.setDetail(
      worldId && meta && revision !== null ? { id: worldId, revision, baseWidth: meta.width } : null,
    );
  });
  $effect(() => view?.setMode(mode));
  $effect(() => view?.setLayer(shown));
  $effect(() => view?.setRelief(dataReady ? relief : 0));
  $effect(() => view?.setGraticule(graticule));
  $effect(() => view?.setReferenceTilt(meta ? meta.axial_tilt_deg : null));
  $effect(() => view?.setReferenceLines(referenceLines));
  $effect(() => view?.setSeaContours(seaContours));
  $effect(() => view?.setLandContours(landContours));
  $effect(() => view?.setAtmosphere(atmosphere));
  $effect(() => view?.setSky(meta ? skyFromWorld(meta) : null));
  $effect(() => view?.setHighlight(dataReady ? highlight : null));
  $effect(() => view?.setPath(path));
  $effect(() => {
    view?.setLighting(
      sun
        ? { kind: 'sun', season, longitudeDeg: sunLongitude, axialTiltDeg: meta?.axial_tilt_deg ?? 23.44 }
        : { kind: 'headlight' },
    );
  });

  // The season animation: one year every YEAR_SECONDS, paused while the
  // canvas is hidden (it resumes from the same day).
  $effect(() => {
    if (!playing || !sun || !canvasVisible) {
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
      hexBusy = false;
      hexServerBusy = false;
      return;
    }
    // A newer request (another slider value, another world, hexes turned
    // off) aborts this one: its result and its errors are dropped.
    const controller = new AbortController();
    const timer = setTimeout(() => {
      hexBusy = true;
      retryWhileBusy((signal) => api.hex(id, { columns, river_flow_km2: riverFlow }, signal), {
        signal: controller.signal,
        // The server answers 503 while it generates a world.
        isBusy: (e) => e instanceof ApiFailure && e.status === 503,
        delayMs: HEX_RETRY_MS,
        onBusy: () => (hexServerBusy = true),
      })
        .then((map) => {
          current.setHex(map);
          hexMap = map;
        })
        .catch((e: unknown) => {
          if (!controller.signal.aborted) {
            toasts.error(`Could not compute the hex grid: ${e instanceof ApiFailure ? e.message : String(e)}`);
          }
        })
        .finally(() => {
          if (!controller.signal.aborted) {
            hexBusy = false;
            hexServerBusy = false;
          }
        });
    }, 300);
    return () => {
      controller.abort();
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
    if (!keyboard || event.ctrlKey || event.metaKey || (event.altKey && !altAllowed(event.key))) {
      return;
    }
    const target = event.target as HTMLElement | null;
    if (target && (target.isContentEditable || ['INPUT', 'SELECT', 'TEXTAREA'].includes(target.tagName))) {
      return;
    }
    const shortcut = shortcutFor(event.key);
    if (!shortcut) {
      return;
    }
    event.preventDefault();
    switch (shortcut.action) {
      case 'layer':
        selectLayer(shortcut.index);
        break;
      case 'globe':
        mode = 'globe';
        break;
      case 'map':
        mode = 'map';
        break;
      case 'reset':
        resetView();
        break;
      case 'hexes':
        if (worldId) hexOn = !hexOn;
        break;
      case 'sunlight':
        sun = !sun;
        break;
      case 'seasons':
        // The seasons need the sun: turn it on and play.
        if (!sun) {
          sun = true;
          playing = true;
        } else {
          playing = !playing;
        }
        break;
      case 'atmosphere':
        atmosphere = !atmosphere;
        break;
      case 'clouds':
        clouds = !clouds;
        break;
      case 'drift':
        // Drifting needs the clouds: show them and drift.
        if (!clouds) {
          clouds = true;
          cloudDrift = true;
        } else {
          cloudDrift = !cloudDrift;
        }
        break;
      case 'grid':
        graticule = !graticule;
        break;
      case 'tropics':
        referenceLines = !referenceLines;
        break;
      case 'depthContours':
        seaContours = !seaContours;
        break;
      case 'elevationContours':
        landContours = !landContours;
        break;
      case 'measure':
        if (worldId) onToggleMeasure();
        break;
      case 'screenshot':
        if (worldId) void screenshot();
        break;
      case 'help':
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
      case 'previousWorld':
        onStepWorld?.(-1);
        break;
      case 'nextWorld':
        onStepWorld?.(1);
        break;
      case 'reliefUp':
        if (dataReady) relief = stepRelief(relief, 1);
        break;
      case 'reliefDown':
        if (dataReady) relief = stepRelief(relief, -1);
        break;
    }
  }

  // The help takes the focus when it opens, and gives it back when it
  // closes, so a keyboard user lands where they were.
  let helpDialog = $state<HTMLDivElement | null>(null);
  $effect(() => {
    if (!showHelp || !helpDialog) {
      return;
    }
    const previous = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    helpDialog.focus();
    return () => {
      if (previous?.isConnected) {
        previous.focus();
      }
    };
  });

  function helpKey(event: KeyboardEvent): void {
    // Also in a compare pane, which has no other shortcuts.
    if (event.key === 'Escape') {
      event.preventDefault();
      event.stopPropagation();
      showHelp = false;
    }
  }
</script>

<svelte:window onkeydown={handleKey} />

<div class="viewer">
  <canvas bind:this={canvas} class:measuring></canvas>

  <div class="toolbar" role="toolbar" aria-label="Tools">
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

  <div class="panel" role="region" aria-label="Display">
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
      <label class="check"><input type="checkbox" bind:checked={clouds} /> Clouds</label>
      {#if clouds}
        <label class="check sub"><input type="checkbox" bind:checked={cloudDrift} /> Drift</label>
      {/if}
      <label class="check"><input type="checkbox" bind:checked={graticule} /> Grid</label>
      <label class="check" title="Equator, tropics and polar circles">
        <input type="checkbox" bind:checked={referenceLines} /> Tropics
      </label>
      <label class="check" title="Depth contours (isobaths) under water, in pale blue (needs the world's data)">
        <input type="checkbox" bind:checked={seaContours} disabled={!dataReady} /> Depth contours
      </label>
      <label class="check" title="Elevation contours on land, in brown, every fifth one stronger (needs the world's data)">
        <input type="checkbox" bind:checked={landContours} disabled={!dataReady} /> Elevation contours
      </label>
      <label class="check">
        <input type="checkbox" bind:checked={hexOn} disabled={!worldId} />
        Hexes
        {#if hexServerBusy}
          <small class="muted" title="The server is generating a world: the hex grid follows once it is done"
            >server busy, retrying…</small
          >
        {:else if hexBusy}
          <small class="muted">computing…</small>
        {/if}
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
    <div class="legend" role="region" aria-label="Legend">
      <strong>{ramp.label}</strong>
      <div class="bar" style:background={cssGradient(shown)}></div>
      <div class="ends">
        <span>{ramp.min}</span>
        <span>{shown === 'coast' && layers?.extra ? `${layers.extra.coastMaxKm} km (√ scale)` : ramp.max}</span>
      </div>
    </div>
  {:else if shown === 'biomes' && world.stats}
    <div class="legend biomes" role="region" aria-label="Biome legend">
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
                <span class="swatch" style:background={hexOn ? entry.biome.color : surfaceColour(entry.biome)}></span>
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
    <div
      class="help"
      role="dialog"
      aria-label="Keyboard shortcuts"
      tabindex="-1"
      bind:this={helpDialog}
      onkeydown={helpKey}
    >
      <header>
        <strong>Keyboard shortcuts</strong>
        <button class="close" onclick={() => (showHelp = false)} aria-label="Close">×</button>
      </header>
      <dl>
        {#each SHORTCUT_HELP as [key, action] (key)}
          <dt><kbd>{key}</kbd></dt>
          <dd>{action}</dd>
        {/each}
      </dl>
    </div>
  {/if}

  {#if !worldId}
    <p class="overlay">Select a world, or generate a new one.</p>
  {:else if loading}
    <div class="overlay">
      <div class="loading">
        <p>Loading…</p>
        {#if mapProgress}<LoadingBar progress={mapProgress} />{/if}
      </div>
    </div>
  {:else if !dataReady && !world.error}
    <div class="chip">
      <p>Loading the raw data…</p>
      {#if world.progress}<LoadingBar progress={world.progress} />{/if}
    </div>
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
  .panel label.check.sub {
    padding-left: 18px;
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
  .help:focus-visible {
    outline: 2px solid var(--accent);
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
  .loading {
    display: grid;
    justify-items: center;
    gap: 8px;
  }
  .loading p,
  .chip p {
    margin: 0;
  }
  .chip:has(:global(.loading-bar)) {
    border-radius: 10px;
    padding: 6px 12px;
    text-align: center;
  }
  .chip :global(.loading-bar) {
    margin: 4px 0 2px;
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
