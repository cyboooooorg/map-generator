<script lang="ts">
  import type { WorldMetadata } from '../lib/api/types/WorldMetadata';
  import { formatLonLat, lonLatToPixel, type LonLat } from '../lib/geo';
  import type { WorldLayers } from '../lib/layers';

  interface Props {
    point: LonLat;
    meta: WorldMetadata;
    /** Decoded layers; null while they load. */
    layers: WorldLayers | null;
    onClose: () => void;
  }

  let { point, meta, layers, onClose }: Props = $props();

  const sample = $derived.by(() => {
    if (!layers) {
      return null;
    }
    const [x, y] = lonLatToPixel(point, layers.width, layers.height);
    const i = y * layers.width + x;
    const biome = meta.biomes[layers.biomes[i] ?? 0];
    return {
      x,
      y,
      biome,
      // The coastline sits at the preset's waterline, relative to the shifted
      // sea level: positive is dry land.
      relief: (layers.elevation[i] ?? 0) - meta.sea_level - meta.preset.rules.waterline,
      temperature: layers.temperature[i] ?? 0,
      moisture: layers.moisture[i] ?? 0,
      flowKm2: layers.flowKm2[i] ?? 0,
    };
  });

  const number = new Intl.NumberFormat('en', { maximumFractionDigits: 0 });
</script>

<aside class="info" aria-label="Point information">
  <header>
    <strong>{formatLonLat(point)}</strong>
    <button class="close" onclick={onClose} aria-label="Close">×</button>
  </header>
  {#if sample}
    {#if sample.biome}
      <p class="biome">
        <span class="swatch" style:background={sample.biome.color}></span>
        {sample.biome.name}
      </p>
    {/if}
    <dl>
      <dt>Elevation</dt>
      <dd>{sample.relief >= 0 ? '+' : ''}{sample.relief.toFixed(3)} <small>above the coastline</small></dd>
      <dt>Temperature</dt>
      <dd>{sample.temperature.toFixed(2)} <small>0 polar · 1 equatorial</small></dd>
      <dt>Moisture</dt>
      <dd>{sample.moisture.toFixed(2)} <small>-1 arid · 1 wet</small></dd>
      {#if sample.flowKm2 > 0}
        <dt>River flow</dt>
        <dd>{number.format(sample.flowKm2)} km² <small>drained</small></dd>
      {/if}
      <dt>Pixel</dt>
      <dd>{sample.x}, {sample.y}</dd>
    </dl>
  {:else}
    <p class="muted">Loading the raw data…</p>
  {/if}
</aside>

<style>
  .info {
    position: absolute;
    left: 12px;
    bottom: 12px;
    width: 280px;
    background: var(--panel);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 12px 14px;
    max-height: calc(100% - 24px);
    overflow-y: auto;
  }
  /* Narrow screens: 280 px plus the legend in the other bottom corner do not
     fit side by side, so span the viewer (covering the legend while open). */
  @media (max-width: 760px) {
    .info {
      right: 12px;
      width: auto;
      /* Opaque: the legend underneath would show through var(--panel). */
      background: #141924;
    }
  }
  header {
    display: flex;
    justify-content: space-between;
    align-items: center;
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
  .swatch {
    width: 16px;
    height: 16px;
    border-radius: 3px;
    border: 1px solid var(--border);
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
