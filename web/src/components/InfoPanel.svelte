<script lang="ts">
  import type { HexMap } from '../lib/api/types/HexMap';
  import type { WorldMetadata } from '../lib/api/types/WorldMetadata';
  import { copyText, formatArea, formatKm, surfaceColour } from '../lib/format';
  import { formatLonLat, lonLatDegrees, type LonLat } from '../lib/geo';
  import type { WorldLayers } from '../lib/layers';
  import type { RiverIndex } from '../lib/rivers';
  import { coastline, riverAt, sampleHex, samplePoint } from '../lib/sample';
  import { toasts } from '../lib/toasts.svelte';

  interface Props {
    point: LonLat;
    meta: WorldMetadata;
    /** Decoded layers; null while they load. */
    layers: WorldLayers | null;
    /** Vector rivers; null while they load or on worlds without them. */
    rivers: RiverIndex | null;
    /** Hex grid shown in the viewer, if any. */
    hexMap: HexMap | null;
    onClose: () => void;
  }

  let { point, meta, layers, rivers, hexMap, onClose }: Props = $props();

  const sample = $derived(layers ? samplePoint(point, meta, layers) : null);
  const river = $derived(rivers && layers ? riverAt(point, rivers, layers.width) : null);
  const hex = $derived(hexMap ? sampleHex(point, hexMap) : null);

  const MOUTHS = { sea: 'the sea', lake: 'a lake', river: 'another river' } as const;

  async function copyCoordinates(): Promise<void> {
    const [lat, lon] = lonLatDegrees(point);
    const ok = await copyText(`${lat.toFixed(4)}, ${lon.toFixed(4)}`);
    toasts.show(ok ? 'Coordinates copied.' : 'The browser refused to copy.', { kind: ok ? 'info' : 'error' });
  }

  function signed(value: number, digits: number): string {
    return `${value >= 0 ? '+' : ''}${value.toFixed(digits)}`;
  }
</script>

<aside class="info" aria-label="Point information">
  <header>
    <strong>{formatLonLat(point)}</strong>
    <span class="actions">
      <button class="small" onclick={copyCoordinates} title="Copy latitude, longitude in degrees">Copy</button>
      <button class="close" onclick={onClose} aria-label="Close">×</button>
    </span>
  </header>
  {#if sample}
    {#if sample.biome}
      <p class="biome">
        <span class="swatch" style:background={surfaceColour(sample.biome)}></span>
        {sample.biome.name}
      </p>
    {/if}
    <dl>
      <dt>Elevation</dt>
      <dd>{signed(sample.relief, 3)} <small>above the coastline</small></dd>
      <dt>Temperature</dt>
      <dd>{sample.temperature.toFixed(1)} °C <small>annual mean</small></dd>
      <dt>Moisture</dt>
      <dd>{sample.moisture.toFixed(2)} <small>-1 arid · 1 wet</small></dd>
      {#if sample.flowKm2 > 0}
        <dt>Drainage</dt>
        <dd>{formatArea(sample.flowKm2)} <small>upstream</small></dd>
      {/if}
      {#if sample.coastKm !== null}
        <dt>To the sea</dt>
        <dd>{sample.coastCapped ? 'over ' : ''}{formatKm(sample.coastKm)}</dd>
      {/if}
      {#if sample.volcanic !== null}
        <dt>Volcanic zone</dt>
        <dd>{sample.volcanic.toFixed(2)} <small>0 none · 1 volcanic</small></dd>
      {/if}
      {#if sample.floodplain !== null && sample.floodplain > 0.01}
        <dt>Floodplain</dt>
        <dd>{sample.floodplain.toFixed(2)} <small>valley floor weight</small></dd>
      {/if}
      {#if sample.highlandOffset !== null}
        <dt>Highland shift</dt>
        <dd>{signed(sample.highlandOffset, 3)} <small>of the mountain line</small></dd>
      {/if}
      <dt>Pixel</dt>
      <dd>{sample.x}, {sample.y}</dd>
    </dl>

    {#if river}
      {@const parent = river.river.parent !== null ? rivers?.byId.get(river.river.parent) : undefined}
      <h3>River #{river.river.id}</h3>
      <dl>
        <dt>Flow here</dt>
        <dd>{formatArea(river.flowKm2)} <small>drained</small></dd>
        <dt>At the mouth</dt>
        <dd>{formatArea(rivers?.mouthFlowKm2(river.river) ?? 0)}</dd>
        <dt>Length</dt>
        <dd>{formatKm(rivers?.lengthKm(river.river) ?? 0)}</dd>
        <dt>Ends in</dt>
        <dd>
          {MOUTHS[river.river.mouth]}{#if parent}&nbsp;(#{parent.id}){/if}{#if river.river.estuary}, through an estuary{/if}
        </dd>
        {#if river.river.source !== null}
          <dt>Delta</dt>
          <dd>branch of river #{river.river.source}</dd>
        {/if}
      </dl>
    {/if}

    {#if hex}
      <h3>Hex {hex.cell.col}, {hex.cell.row}</h3>
      <dl>
        <dt>Main biome</dt>
        <dd>
          {#if hex.biome}<span class="swatch inline" style:background={hex.biome.color}></span>{hex.biome.name}{/if}
        </dd>
        <dt>Mean elevation</dt>
        <dd>{signed(hex.elevation - coastline(meta), 3)} <small>above the coastline</small></dd>
        <dt>Mean temperature</dt>
        <dd>{hex.temperature.toFixed(1)} °C</dd>
        <dt>Mean moisture</dt>
        <dd>{hex.moisture.toFixed(2)}</dd>
        <dt>Largest river</dt>
        <dd>{hex.maxFlowKm2 > 0 ? formatArea(hex.maxFlowKm2) : 'none'}{hex.river ? ' (river hex)' : ''}</dd>
      </dl>
    {/if}
  {:else}
    <p class="muted">Loading the raw data…</p>
  {/if}
</aside>

<style>
  .info {
    position: absolute;
    left: 12px;
    bottom: 12px;
    width: 300px;
    background: var(--panel);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 12px 14px;
    max-height: calc(100% - 64px);
    overflow-y: auto;
    z-index: 1;
  }
  /* Narrow viewer: 300 px plus the 240 px legend in the other bottom corner
     do not fit side by side, so span the viewer (covering the legend while
     open). Queried on the viewer width: just above the 760 px breakpoint the
     sidebar leaves the viewer only ~420 px. */
  @container (max-width: 580px) {
    .info {
      right: 12px;
      width: auto;
      /* Opaque: the legend underneath would show through var(--panel). */
      background: var(--panel-solid);
    }
  }
  header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 8px;
  }
  .actions {
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .small {
    padding: 2px 8px;
    font-size: 0.85rem;
  }
  .close {
    border: none;
    background: none;
    font-size: 1.2rem;
    padding: 0 4px;
  }
  .biome {
    display: flex;
    gap: 8px;
    align-items: center;
    margin: 10px 0;
    font-size: 1.05rem;
  }
  h3 {
    margin: 12px 0 4px;
    font-size: 0.95rem;
  }
  .swatch {
    width: 16px;
    height: 16px;
    border-radius: 3px;
    border: 1px solid var(--border);
  }
  .swatch.inline {
    display: inline-block;
    width: 12px;
    height: 12px;
    margin-right: 6px;
    vertical-align: -1px;
  }
  dl {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 4px 12px;
    margin: 0;
  }
  dt {
    color: var(--muted);
  }
  dd {
    margin: 0;
    font-variant-numeric: tabular-nums;
  }
  small,
  .muted {
    color: var(--muted);
  }
</style>
