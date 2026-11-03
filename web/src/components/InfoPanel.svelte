<script lang="ts">
  import { t } from '../lib/i18n/index.svelte';
  import { biomeName } from '../lib/i18n/names.svelte';
  import type { HexMap } from '../lib/api/types/HexMap';
  import type { WorldMetadata } from '../lib/api/types/WorldMetadata';
  import { copyText, formatArea, formatKm, formatNumber, surfaceColour, withUnit } from '../lib/format';
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


  async function copyCoordinates(): Promise<void> {
    const [lat, lon] = lonLatDegrees(point);
    const ok = await copyText(`${lat.toFixed(4)}, ${lon.toFixed(4)}`);
    toasts.show(ok ? t.info.copied : t.worldInfo.copyRefused, { kind: ok ? 'info' : 'error' });
  }

  function signed(value: number, digits: number): string {
    return `${value >= 0 ? '+' : ''}${formatNumber(value, digits)}`;
  }
</script>

<aside class="info" aria-label={t.info.label}>
  <header>
    <strong>{formatLonLat(point)}</strong>
    <span class="actions">
      <button class="small" onclick={copyCoordinates} title={t.info.copyHelp}>{t.info.copy}</button>
      <button class="close" onclick={onClose} aria-label={t.info.close}>×</button>
    </span>
  </header>
  {#if sample}
    {#if sample.biome}
      <p class="biome">
        <span class="swatch" style:background={hexMap ? sample.biome.color : surfaceColour(sample.biome)}></span>
        {biomeName(sample.biome)}
      </p>
    {/if}
    <dl>
      <dt>{t.info.elevation}</dt>
      <dd>{signed(sample.relief, 3)} <small>{t.info.aboveCoastline}</small></dd>
      <dt>{t.info.temperature}</dt>
      <dd>{withUnit(formatNumber(sample.temperature, 1), '°C')} <small>{t.info.annualMean}</small></dd>
      <dt>{t.info.moisture}</dt>
      <dd>{formatNumber(sample.moisture, 2)} <small>{t.info.moistureScale}</small></dd>
      {#if sample.flowKm2 > 0}
        <dt>{t.info.drainage}</dt>
        <dd>{formatArea(sample.flowKm2)} <small>{t.info.upstream}</small></dd>
      {/if}
      {#if sample.coastKm !== null}
        <dt>{t.info.toSea}</dt>
        <dd>{sample.coastCapped ? t.info.over({ distance: formatKm(sample.coastKm) }) : formatKm(sample.coastKm)}</dd>
      {/if}
      {#if sample.volcanic !== null}
        <dt>{t.info.volcanic}</dt>
        <dd>{formatNumber(sample.volcanic, 2)} <small>{t.info.volcanicScale}</small></dd>
      {/if}
      {#if sample.floodplain !== null && sample.floodplain > 0.01}
        <dt>{t.info.floodplain}</dt>
        <dd>{formatNumber(sample.floodplain, 2)} <small>{t.info.floodplainScale}</small></dd>
      {/if}
      {#if sample.highlandOffset !== null}
        <dt>{t.info.highland}</dt>
        <dd>{signed(sample.highlandOffset, 3)} <small>{t.info.highlandScale}</small></dd>
      {/if}
      <dt>{t.info.pixel}</dt>
      <dd>{sample.x}, {sample.y}</dd>
    </dl>

    {#if river}
      {@const parent = river.river.parent !== null ? rivers?.byId.get(river.river.parent) : undefined}
      <h3>{t.info.river({ id: river.river.id })}</h3>
      <dl>
        <dt>{t.info.flowHere}</dt>
        <dd>{formatArea(river.flowKm2)} <small>{t.info.drained}</small></dd>
        <dt>{t.info.atMouth}</dt>
        <dd>{formatArea(rivers?.mouthFlowKm2(river.river) ?? 0)}</dd>
        <dt>{t.info.length}</dt>
        <dd>{formatKm(rivers?.lengthKm(river.river) ?? 0)}</dd>
        <dt>{t.info.endsIn}</dt>
        <dd>
          {t.info.mouths[river.river.mouth]}{#if parent}&nbsp;(#{parent.id}){/if}{#if river.river.estuary}{t.info.estuary}{/if}
        </dd>
        {#if river.river.source !== null}
          <dt>{t.info.delta}</dt>
          <dd>{t.info.branchOf({ id: river.river.source })}</dd>
        {/if}
      </dl>
    {/if}

    {#if hex}
      <h3>{t.info.hex({ col: hex.cell.col, row: hex.cell.row })}</h3>
      <dl>
        <dt>{t.info.mainBiome}</dt>
        <dd>
          {#if hex.biome}<span class="swatch inline" style:background={hex.biome.color}></span>{biomeName(hex.biome)}{/if}
        </dd>
        <dt>{t.info.meanElevation}</dt>
        <dd>{signed(hex.elevation - coastline(meta), 3)} <small>{t.info.aboveCoastline}</small></dd>
        <dt>{t.info.meanTemperature}</dt>
        <dd>{withUnit(formatNumber(hex.temperature, 1), '°C')}</dd>
        <dt>{t.info.meanMoisture}</dt>
        <dd>{formatNumber(hex.moisture, 2)}</dd>
        <dt>{t.info.largestRiver}</dt>
        <dd>{hex.maxFlowKm2 > 0 ? formatArea(hex.maxFlowKm2) : t.info.noRiver}{hex.river ? t.info.riverHex : ''}</dd>
      </dl>
    {/if}
  {:else}
    <p class="muted">{t.info.loading}</p>
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
