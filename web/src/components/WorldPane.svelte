<script lang="ts">
  import { biomeName } from '../lib/i18n/names';
  import { t } from '../lib/i18n/index.svelte';
  import type { HexMap } from '../lib/api/types/HexMap';
  import type { LinkedDisplay } from '../lib/display';
  import { formatKm, formatNumber } from '../lib/format';
  import { formatLonLat, pathKm, type LonLat } from '../lib/geo';
  import type { ViewLink } from '../lib/link';
  import { samplePoint } from '../lib/sample';
  import type { CameraState } from '../lib/viewer/PlanetView';
  import type { LoadedWorld } from '../lib/world.svelte';
  import InfoPanel from './InfoPanel.svelte';
  import Viewer from './Viewer.svelte';

  interface Props {
    world: LoadedWorld;
    revision: number | null;
    /** Whether this pane answers keyboard shortcuts. */
    keyboard: boolean;
    /** Shown in a corner when two worlds are compared. */
    label?: string | null;
    onCameraChange?: (state: CameraState) => void;
    /** `[` / `]`: show the previous (-1) or next (1) world of the list. */
    onStepWorld?: (direction: 1 | -1) => void;
    /** Called with the linked display settings whenever one changes. */
    onDisplayChange?: (display: LinkedDisplay) => void;
    /** Display settings to mirror ("Link display"), or null. */
    linkedDisplay?: LinkedDisplay | null;
    /** Whether its display changes are saved as the preferences (the main pane only). */
    persistDisplay?: boolean;
  }

  let {
    world,
    revision,
    keyboard,
    label = null,
    onCameraChange,
    onStepWorld,
    onDisplayChange,
    linkedDisplay = null,
    persistDisplay = true,
  }: Props = $props();

  let viewer = $state<Viewer | null>(null);
  let picked = $state<LonLat | null>(null);
  let hover = $state<LonLat | null>(null);
  let measuring = $state(false);
  let path = $state<LonLat[]>([]);
  let hexMap = $state<HexMap | null>(null);

  // A new world: nothing picked or measured on it yet.
  $effect(() => {
    void world.id;
    picked = null;
    hover = null;
    path = [];
    measuring = false;
  });

  const hoverSample = $derived(
    hover && world.meta && world.layers ? samplePoint(hover, world.meta, world.layers) : null,
  );
  const distance = $derived(world.meta ? pathKm(path, world.meta.circumference_km) : 0);

  function pick(point: LonLat | null): void {
    if (measuring) {
      if (point) {
        path = [...path, point];
      }
      return;
    }
    picked = point;
  }

  function toggleMeasure(): void {
    measuring = !measuring;
    if (measuring) {
      picked = null;
      path = [];
    }
  }

  function escape(): void {
    if (measuring) {
      measuring = false;
      path = [];
    } else {
      picked = null;
    }
  }

  /** Shows the same place as another pane. */
  export function applyCamera(state: CameraState): void {
    viewer?.applyCamera(state);
  }

  /** The camera, layer, relief and sun shown, for a link to this view. */
  export function viewState(): ViewLink | null {
    return viewer?.viewState() ?? null;
  }

  /** Opens the view of a link (the parts it has). */
  export function applyView(link: ViewLink): void {
    viewer?.applyView(link);
  }
</script>

<div class="pane">
  <Viewer
    bind:this={viewer}
    bind:hexMap
    {world}
    {revision}
    {path}
    {measuring}
    {keyboard}
    {persistDisplay}
    onPick={pick}
    onHover={(point) => (hover = point)}
    onToggleMeasure={toggleMeasure}
    onEscape={escape}
    {...onCameraChange ? { onCameraChange } : {}}
    {...onStepWorld ? { onStepWorld } : {}}
    {...onDisplayChange ? { onDisplayChange } : {}}
    {linkedDisplay}
  />

  {#if label}
    <p class="label">{label}</p>
  {/if}

  {#if hover}
    <p class="readout" aria-live="off">
      {[
        formatLonLat(hover),
        hoverSample?.biome ? biomeName(hoverSample.biome) : null,
        hoverSample ? `${hoverSample.relief >= 0 ? '+' : ''}${formatNumber(hoverSample.relief, 3)}` : null,
      ]
        .filter(Boolean)
        .join(' · ')}
    </p>
  {/if}

  {#if measuring}
    <div class="measure" role="region" aria-label={t.measure.label}>
      <strong>{path.length < 2 ? t.measure.start : formatKm(distance)}</strong>
      {#if path.length >= 2}
        <small>{t.measure.legs({ legs: path.length - 1 })}</small>
      {/if}
      <div class="actions">
        <button onclick={() => (path = path.slice(0, -1))} disabled={path.length === 0}>{t.measure.undo}</button>
        <button onclick={() => (path = [])} disabled={path.length === 0}>{t.measure.clear}</button>
        <button onclick={toggleMeasure}>{t.measure.done}</button>
      </div>
    </div>
  {:else if picked && world.meta}
    <InfoPanel
      point={picked}
      meta={world.meta}
      layers={world.layers}
      rivers={world.rivers}
      {hexMap}
      onClose={() => (picked = null)}
    />
  {/if}
</div>

<style>
  .pane {
    position: relative;
    height: 100%;
    min-height: 0;
    min-width: 0;
    /* The overlays adapt to the pane width, not the window. */
    container-type: inline-size;
  }
  /* Bottom left: the point panel and the measure box cover it while open. */
  .label {
    position: absolute;
    left: 12px;
    bottom: 12px;
    /* Room for the 240 px legend in the other corner, but never unreadable. */
    max-width: max(120px, calc(100% - 276px));
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    margin: 0;
    padding: 2px 10px;
    border-radius: 6px;
    background: var(--panel);
    border: 1px solid var(--border);
    font-weight: 600;
    pointer-events: none;
  }
  .readout {
    position: absolute;
    left: 50%;
    bottom: 12px;
    transform: translateX(-50%);
    margin: 0;
    padding: 3px 10px;
    border-radius: 6px;
    background: var(--panel);
    border: 1px solid var(--border);
    font-size: 0.85rem;
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
    pointer-events: none;
  }
  .measure {
    position: absolute;
    left: 12px;
    bottom: 12px;
    display: grid;
    gap: 6px;
    min-width: 220px;
    background: var(--panel);
    border: 1px solid var(--accent);
    border-radius: 8px;
    padding: 10px 12px;
    z-index: 1;
  }
  .measure small {
    color: var(--muted);
  }
  .actions {
    display: flex;
    gap: 6px;
  }
  .actions button {
    padding: 2px 8px;
  }
  /* Too narrow for the readout between the info panel and the legend; the
     pointer readout is for mice anyway. */
  @container (max-width: 900px) {
    .readout {
      display: none;
    }
  }
  /* Narrow pane: the box spans it, opaque over the legend. */
  @container (max-width: 580px) {
    .measure {
      right: 12px;
      background: var(--panel-solid);
    }
  }
</style>
