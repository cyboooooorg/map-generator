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
  import { FLOAT_MIN_WIDTH, type FloatAt, INFO_PANEL_BOTTOM, infoPanelAt } from '../lib/dock';
  import { loadSetting, saveSetting } from '../lib/settings';

  interface Props {
    point: LonLat;
    meta: WorldMetadata;
    /** Decoded layers; null while they load. */
    layers: WorldLayers | null;
    /** Vector rivers; null while they load or on worlds without them. */
    rivers: RiverIndex | null;
    /** Hex grid shown in the viewer, if any. */
    hexMap: HexMap | null;
    /** The size of the view the panel floats over, CSS px. */
    viewWidth: number;
    viewHeight: number;
    onClose: () => void;
  }

  let { point, meta, layers, rivers, hexMap, viewWidth, viewHeight, onClose }: Props = $props();

  /**
   * Where the user dragged the panel, kept for every point and session
   * (null: its default place, see infoPanelAt). Narrow views have no room
   * to float it: it spans their bottom instead.
   */
  let saved = $state(loadSetting<FloatAt | null>('infoPanelAt', null));
  let height = $state(0);
  const floating = $derived(viewWidth >= FLOAT_MIN_WIDTH);
  const at = $derived(infoPanelAt(saved, height, viewWidth, viewHeight));

  /** Drags the panel by its header, as the dock's floating cards. */
  function drag(event: PointerEvent): void {
    if (!floating || event.button !== 0 || (event.target as HTMLElement).closest('button')) return;
    // No text selection while dragging.
    event.preventDefault();
    const header = event.currentTarget as HTMLElement;
    header.setPointerCapture(event.pointerId);
    const start = at;
    const from = { x: event.clientX, y: event.clientY };
    const move = (e: PointerEvent) => {
      saved = infoPanelAt({ x: start.x + e.clientX - from.x, y: start.y + e.clientY - from.y }, height, viewWidth, viewHeight);
    };
    const end = () => {
      header.removeEventListener('pointermove', move);
      header.removeEventListener('pointerup', end);
      header.removeEventListener('pointercancel', end);
      saveSetting('infoPanelAt', saved);
    };
    header.addEventListener('pointermove', move);
    header.addEventListener('pointerup', end);
    header.addEventListener('pointercancel', end);
  }

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

<aside
  class="info"
  class:floating
  aria-label={t.info.label}
  bind:clientHeight={height}
  style:left={floating ? `${at.x}px` : null}
  style:top={floating ? `${at.y}px` : null}
  style:max-height={floating ? `${Math.max(120, viewHeight - INFO_PANEL_BOTTOM - at.y - 8)}px` : null}
>
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <header onpointerdown={drag}>
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
  /* Narrow views: along the bottom, as wide as the view (below). */
  .info {
    position: absolute;
    left: 12px;
    bottom: 12px;
    width: 300px;
    /* Framed as the theme frames panels (see app.css and lib/themes.ts). */
    background: var(--frame-deco), var(--panel);
    clip-path: var(--frame-clip);
    box-shadow: var(--glow);
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
      /* Framed as the theme frames panels (see app.css and lib/themes.ts). */
      background: var(--frame-deco), var(--panel-solid);
      clip-path: var(--frame-clip);
      box-shadow: var(--glow);
    }
  }
  /* Wide views: floated at `at` (see infoPanelAt), dragged by its header. */
  .info.floating {
    bottom: auto;
    right: auto;
    z-index: 3;
  }
  .floating header {
    cursor: grab;
    user-select: none;
    touch-action: none;
  }
  .floating header:active {
    cursor: grabbing;
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
