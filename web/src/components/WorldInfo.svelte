<script lang="ts">
  import { worldFile } from '../lib/api/client';
  import type { WorldSummary } from '../lib/api/types/WorldSummary';
  import { downloadRawData } from '../lib/download';
  import { cliCommand, copyText, formatAgo, formatArea, formatAtmosphere, formatInt, formatShare, surfaceColour } from '../lib/format';
  import { loadSetting, saveSetting } from '../lib/settings';
  import { toasts } from '../lib/toasts.svelte';
  import type { LoadedWorld } from '../lib/world.svelte';

  interface Props {
    world: LoadedWorld;
    summary: WorldSummary | undefined;
    /** Other worlds, to compare with. */
    worlds: WorldSummary[];
    compareId: string | null;
    onVary: () => void;
    /** Generates the same world again with a new random seed. */
    onReroll: () => Promise<void>;
    onCompare: (id: string | null) => void;
    /** The link Copy link copies: the worlds shown and the view. */
    link: () => string;
    onDelete: () => void;
  }

  let { world, summary, worlds, compareId, onVary, onReroll, onCompare, link, onDelete }: Props = $props();

  let open = $state(loadSetting('worldInfoOpen', true));
  let allBiomes = $state(false);
  let zipping = $state(false);
  let rerolling = $state(false);
  $effect(() => saveSetting('worldInfoOpen', open));

  const meta = $derived(world.meta);
  const stats = $derived(world.stats);
  const others = $derived(worlds.filter((w) => w.id !== world.id));
  const TOP_BIOMES = 6;

  async function copy(text: string, what: string): Promise<void> {
    const ok = await copyText(text);
    toasts.show(ok ? `${what} copied.` : 'The browser refused to copy.', { kind: ok ? 'info' : 'error' });
  }

  async function reroll(): Promise<void> {
    rerolling = true;
    try {
      await onReroll();
    } finally {
      rerolling = false;
    }
  }

  async function zip(): Promise<void> {
    if (!world.id || !meta) {
      return;
    }
    zipping = true;
    try {
      await downloadRawData(world.id, meta);
    } catch (e) {
      toasts.error(`Could not build the archive: ${String(e)}`);
    } finally {
      zipping = false;
    }
  }
</script>

{#if world.id}
  <details class="card" bind:open>
    <summary>
      <span class="title">
        <strong>{meta?.preset.name ?? summary?.planet ?? world.id}</strong>
        <small>{world.id}</small>
      </span>
    </summary>

    {#if meta}
      <p class="description">{meta.preset.description}</p>
      <dl>
        <dt>Seed</dt>
        <dd>{meta.seed}</dd>
        <dt>Map</dt>
        <dd>{meta.width} × {meta.height} px</dd>
        <dt>Sea level</dt>
        <dd>{meta.sea_level.toFixed(3)}</dd>
        <dt>Volcanism</dt>
        <dd>{meta.volcanic_intensity.toFixed(2)}</dd>
        <dt>Erosion</dt>
        <dd>{meta.erosion.toFixed(2)}</dd>
        <dt>Circumference</dt>
        <dd>{formatInt(meta.circumference_km)} km</dd>
        <dt>Axial tilt</dt>
        <dd>{meta.axial_tilt_deg.toFixed(1)}°</dd>
        <dt>Gravity</dt>
        <dd>{meta.gravity_modifier.toFixed(2)} g</dd>
        <dt>Atmosphere</dt>
        <dd>{formatAtmosphere(meta)}</dd>
        {#if stats}
          <dt>Surface</dt>
          <dd>{formatArea(stats.surfaceKm2)}</dd>
          <dt>Land / water</dt>
          <dd>{formatShare(stats.land)} / {formatShare(stats.water)}</dd>
        {/if}
        {#if world.rivers}
          <dt>Rivers</dt>
          <dd>{formatInt(world.rivers.file.rivers.length)}</dd>
        {/if}
        {#if summary}
          <dt>Generated</dt>
          <dd title={new Date(summary.modified_ms).toLocaleString()}>{formatAgo(summary.modified_ms)}</dd>
        {/if}
      </dl>

      {#if stats}
        <div class="bar" aria-hidden="true">
          {#each stats.shares as entry (entry.biome.id)}
            <span
              style:background={surfaceColour(entry.biome)}
              style:flex-grow={entry.share}
              title={`${entry.biome.name}: ${formatShare(entry.share)}`}
            ></span>
          {/each}
        </div>
        <ul class="biomes">
          {#each allBiomes ? stats.shares : stats.shares.slice(0, TOP_BIOMES) as entry (entry.biome.id)}
            <li>
              <span class="swatch" style:background={surfaceColour(entry.biome)}></span>
              <span class="name">{entry.biome.name}</span>
              <span class="value">{formatShare(entry.share)}</span>
              <span class="value area">{formatArea(entry.areaKm2)}</span>
            </li>
          {/each}
        </ul>
        {#if stats.shares.length > TOP_BIOMES}
          <button class="link" onclick={() => (allBiomes = !allBiomes)}>
            {allBiomes ? 'Show fewer' : `Show all ${stats.shares.length} biomes`}
          </button>
        {/if}
      {:else}
        <p class="muted">Measuring the biomes…</p>
      {/if}

      <div class="actions">
        <button onclick={reroll} disabled={rerolling} title="Generate this world again with a new random seed"
          >{rerolling ? 'Sending…' : 'Reroll'}</button
        >
        <button onclick={onVary} title="Open the form with this world's parameters">Vary…</button>
        <button onclick={() => copy(cliCommand(meta), 'Command')} title="Command line that regenerates this world"
          >Copy command</button
        >
        <button onclick={() => copy(link(), 'Link')} title="Link that opens this world, with the camera, layer, relief and sun shown">Copy link</button>
        <button class="danger" onclick={onDelete}>Delete</button>
      </div>

      <label class="compare">
        <span>Compare with</span>
        <select value={compareId ?? ''} onchange={(e) => onCompare(e.currentTarget.value || null)}>
          <option value="">nothing</option>
          {#each others as other (other.id)}
            <option value={other.id}>{other.id}</option>
          {/each}
        </select>
      </label>

      <div class="downloads">
        <span>Download</span>
        <a href={worldFile(world.id, 'world.png', summary?.modified_ms)} download={`${world.id}.png`}>map PNG</a>
        <a href={worldFile(world.id, 'world.svg', summary?.modified_ms)} download={`${world.id}.svg`}>SVG</a>
        <a href={worldFile(world.id, 'legend.png', summary?.modified_ms)} download={`${world.id}-legend.png`}>legend</a>
        <button class="link" onclick={zip} disabled={zipping}>{zipping ? 'zipping…' : 'raw data (ZIP)'}</button>
      </div>
    {:else if world.error}
      <p class="error">{world.error}</p>
    {:else}
      <p class="muted">Loading…</p>
    {/if}
  </details>
{/if}

<style>
  .card {
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 8px 10px;
    display: grid;
    gap: 8px;
  }
  summary {
    cursor: pointer;
    list-style-position: outside;
    margin-left: 14px;
  }
  .title {
    display: inline-grid;
    vertical-align: top;
  }
  .title small {
    color: var(--muted);
  }
  .description {
    margin: 8px 0;
    color: var(--muted);
    font-size: 0.9rem;
  }
  dl {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 2px 12px;
    margin: 0 0 8px;
  }
  dt {
    color: var(--muted);
  }
  dd {
    margin: 0;
    font-variant-numeric: tabular-nums;
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
    display: grid;
    gap: 2px;
  }
  .biomes li {
    display: grid;
    grid-template-columns: 12px minmax(0, 1fr) auto auto;
    gap: 8px;
    align-items: center;
    font-size: 0.9rem;
  }
  .swatch {
    width: 12px;
    height: 12px;
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
  .link {
    border: none;
    background: none;
    padding: 0;
    color: var(--link);
    text-align: left;
  }
  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin: 10px 0 8px;
  }
  .actions button {
    padding: 4px 10px;
  }
  .danger:hover:not(:disabled) {
    border-color: var(--danger);
    color: var(--danger);
  }
  .compare {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr);
    gap: 8px;
    align-items: center;
    margin-bottom: 8px;
  }
  .compare select {
    min-width: 0;
  }
  .downloads {
    display: flex;
    flex-wrap: wrap;
    gap: 4px 12px;
    align-items: baseline;
    font-size: 0.9rem;
  }
  .downloads span {
    color: var(--muted);
  }
  .downloads a {
    color: var(--link);
  }
  .muted {
    color: var(--muted);
    margin: 4px 0;
  }
  .error {
    color: var(--danger);
  }
</style>
