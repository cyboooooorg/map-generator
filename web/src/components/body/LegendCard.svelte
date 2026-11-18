<script lang="ts">
  import Glossed from '../Glossed.svelte';
  // The legend of the layer shown: a ramp for a quantity, or, for biomes,
  // a bar of every biome's share, then each biome's swatch, share and area. A biome's row highlights it on the
  // globe (hover) or keeps it highlighted (click).
  import './facts.css';
  import { t } from '../../lib/i18n/index.svelte';
  import { biomeName } from '../../lib/i18n/names.svelte';
  import { formatArea, formatShare } from '../../lib/format';
  import type { BiomeInfo } from '../../lib/api/types/BiomeInfo';
  import type { WorldStats } from '../../lib/stats';

  /** The ramp of a quantity layer: its name, CSS gradient and the words at both ends. */
  export interface Ramp {
    title: string;
    gradient: string;
    min: string;
    max: string;
  }

  interface Props {
    /** The ramp of the layer shown; null for biomes. */
    ramp: Ramp | null;
    /** The biome statistics when the layer is biomes, else null. */
    stats: WorldStats | null;
    /** A biome's colour, as the globe shows it. */
    colour: (biome: BiomeInfo) => string;
    hovered?: number | null;
    pinned?: number | null;
  }

  let { ramp, stats, colour, hovered = $bindable(null), pinned = $bindable(null) }: Props = $props();

  const TOP_BIOMES = 6;
  let all = $state(false);
</script>

{#if ramp}
  <strong><Glossed text={ramp.title} /></strong>
  <div class="ramp" style:background={ramp.gradient}></div>
  <div class="ends"><span><Glossed text={ramp.min} /></span><span><Glossed text={ramp.max} /></span></div>
{:else if stats}
  <div class="bar" aria-hidden="true">
    {#each stats.shares as entry (entry.biome.id)}
      <span
        style:background={colour(entry.biome)}
        style:flex-grow={entry.share}
        title={`${biomeName(entry.biome)}: ${formatShare(entry.share)}`}
      ></span>
    {/each}
  </div>
  <ul class="biomes" aria-label={t.viewer.biomeLegend} onpointerleave={() => (hovered = null)}>
    {#each all ? stats.shares : stats.shares.slice(0, TOP_BIOMES) as entry (entry.biome.id)}
      <li>
        <button type="button"
          class:pinned={pinned === entry.biome.id}
          onpointerenter={() => (hovered = entry.biome.id)}
          onfocus={() => (hovered = entry.biome.id)}
          onblur={() => (hovered = null)}
          onclick={() => (pinned = pinned === entry.biome.id ? null : entry.biome.id)}
          aria-pressed={pinned === entry.biome.id}
          title={t.viewer.highlightHelp}
        >
          <span class="swatch" style:background={colour(entry.biome)}></span>
          <span class="name"><Glossed text={biomeName(entry.biome)} /></span>
          <span class="value"><Glossed text={formatShare(entry.share)} /></span>
          <span class="value area" title={t.cards.biomeArea}><Glossed text={formatArea(entry.areaKm2)} /></span>
        </button>
      </li>
    {/each}
  </ul>
  {#if stats.shares.length > TOP_BIOMES}
    <button type="button" class="more" onclick={() => (all = !all)}>
      {all ? t.worldInfo.showFewer : t.worldInfo.showAll({ count: stats.shares.length })}
    </button>
  {/if}
{:else}
  <p class="muted"><Glossed text={t.cards.legendEmpty} /></p>
{/if}

<style>
  .ramp {
    height: 10px;
    border-radius: 3px;
    margin: 4px 0;
  }
  .ends {
    display: flex;
    justify-content: space-between;
    color: var(--muted);
  }
  .bar {
    display: flex;
    height: 10px;
    border-radius: 3px;
    overflow: hidden;
    margin-bottom: 6px;
  }
  .bar span {
    flex-basis: 0;
    min-width: 1px;
  }
  .biomes {
    list-style: none;
    margin: 0;
    padding: 0;
    overflow-y: auto;
    max-height: min(35vh, 300px);
  }
  .biomes button {
    display: grid;
    grid-template-columns: 14px minmax(0, 1fr) auto auto;
    gap: 8px;
    align-items: center;
    width: 100%;
    border: 1px solid transparent;
    background: none;
    padding: 2px 4px;
    text-align: left;
  }
  .biomes button.pinned {
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
  .value {
    font-variant-numeric: tabular-nums;
    color: var(--muted);
    text-align: right;
  }
  .area {
    min-width: 7.5em;
  }
  .more {
    border: none;
    background: none;
    padding: 0;
    color: var(--link);
  }
  .muted {
    color: var(--muted);
    margin: 0;
  }
</style>
