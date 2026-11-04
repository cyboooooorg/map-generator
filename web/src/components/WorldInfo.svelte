<script lang="ts">
  import { getLocale, t } from '../lib/i18n/index.svelte';
  import { biomeName, presetDescription, presetName } from '../lib/i18n/names.svelte';
  import { worldFile } from '../lib/api/client';
  import type { WorldSummary } from '../lib/api/types/WorldSummary';
  import { downloadRawData } from '../lib/download';
  import { cliCommand, copyText, formatAgo, formatArea, formatAtmosphere, formatAu, formatEccentricity, formatGeology, formatInt, formatNumber, formatOrbit, formatShare, formatStar, formatStarlight, formatUpTo, formatYear, surfaceColour, withUnit } from '../lib/format';
  import { unitTooltip } from '../lib/units';
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
    toasts.show(ok ? t.worldInfo.copied({ what }) : t.worldInfo.copyRefused, { kind: ok ? 'info' : 'error' });
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
      toasts.error(t.worldInfo.archiveFailed({ error: String(e) }));
    } finally {
      zipping = false;
    }
  }
</script>

{#if world.id}
  <details class="card" bind:open>
    <summary>
      <span class="title">
        <strong>{meta ? presetName(meta.preset) : (summary?.planet ?? world.id)}</strong>
        <small>{world.id}</small>
      </span>
    </summary>

    {#if meta}
      <p class="description">{presetDescription(meta.preset)}</p>
      <dl>
        <dt>{t.worldInfo.seed}</dt>
        <dd>{meta.seed}</dd>
        <dt>{t.worldInfo.map}</dt>
        <dd>{meta.width} × {meta.height} px</dd>
        <dt>{t.worldInfo.seaLevel}</dt>
        <dd>{formatNumber(meta.sea_level, 3)}</dd>
        <dt>{t.worldInfo.volcanism}</dt>
        <dd>{formatNumber(meta.volcanic_intensity, 2)}</dd>
        <dt>{t.worldInfo.erosion}</dt>
        <dd>{formatNumber(meta.erosion, 2)}</dd>
        <dt>{t.worldInfo.circumference}</dt>
        <dd>{withUnit(formatInt(meta.circumference_km), 'km')}</dd>
        <dt>{t.worldInfo.axialTilt}</dt>
        <dd>{formatNumber(meta.axial_tilt_deg, 1)}°</dd>
        <dt>{t.worldInfo.gravity}</dt>
        <dd class="unit-help" title={unitTooltip('g')}>{withUnit(formatNumber(meta.gravity_modifier, 2), 'g')}</dd>
        <dt>{t.worldInfo.atmosphere}</dt>
        {#if (meta.pressure_bar ?? 1) > 0}
          <dd class="unit-help" title={unitTooltip('bar')}>{formatAtmosphere(meta)}</dd>
        {:else}
          <dd>{formatAtmosphere(meta)}</dd>
        {/if}
        <dt>{t.worldInfo.geology}</dt>
        {#if meta.surface_age_ga != null}
          <dd class="unit-help" title={unitTooltip('Ga', 'Ma')}>{formatGeology(meta)}</dd>
        {:else}
          <dd>{formatGeology(meta)}</dd>
        {/if}
        <dt>{t.worldInfo.star}</dt>
        {#if meta.star}
          {@const star = meta.star}
          <dd class="unit-help" title={unitTooltip('L☉')}>{formatStar(star)}</dd>
          <dt>{t.worldInfo.orbit}</dt>
          <dd
            class="unit-help"
            title={`${t.worldInfo.hzRange({ inner: formatAu(star.hz_inner_au), outer: formatAu(star.hz_outer_au) })}\n${unitTooltip('AU')}`}
          >
            {formatOrbit(star)}
          </dd>
          <dt>{t.worldInfo.eccentricity}</dt>
          <dd class="unit-help" title={unitTooltip('AU')}>{formatEccentricity(star)}</dd>
          <dt>{t.worldInfo.light}</dt>
          <dd>{formatStarlight(star)}</dd>
          <dt>{t.worldInfo.year}</dt>
          <dd>{formatYear(star.year_days)}</dd>
          <dt>{t.worldInfo.starSize}</dt>
          <dd>
            {t.worldInfo.starSizeValue({
              size: `${formatUpTo(star.angular_diameter_deg, 2)}°`,
              sun: `${formatUpTo(0.53, 2)}°`,
            })}{#if star.tidally_locked}
              · <span class="unit-help" title={t.worldInfo.lockedHelp}>{t.worldInfo.locked}</span>{/if}
          </dd>
        {:else}
          <dd>{t.worldInfo.theSun}</dd>
        {/if}
        {#if stats}
          <dt>{t.worldInfo.surface}</dt>
          <dd>{formatArea(stats.surfaceKm2)}</dd>
          <dt>{t.worldInfo.landWater}</dt>
          <dd>{formatShare(stats.land)} / {formatShare(stats.water)}</dd>
        {/if}
        {#if world.rivers}
          <dt>{t.worldInfo.rivers}</dt>
          <dd>{formatInt(world.rivers.file.rivers.length)}</dd>
        {/if}
        {#if summary}
          <dt>{t.worldInfo.generated}</dt>
          <dd title={new Date(summary.modified_ms).toLocaleString(getLocale())}>{formatAgo(summary.modified_ms)}</dd>
        {/if}
      </dl>

      {#if stats}
        <div class="bar" aria-hidden="true">
          {#each stats.shares as entry (entry.biome.id)}
            <span
              style:background={surfaceColour(entry.biome)}
              style:flex-grow={entry.share}
              title={`${biomeName(entry.biome)}: ${formatShare(entry.share)}`}
            ></span>
          {/each}
        </div>
        <ul class="biomes">
          {#each allBiomes ? stats.shares : stats.shares.slice(0, TOP_BIOMES) as entry (entry.biome.id)}
            <li>
              <span class="swatch" style:background={surfaceColour(entry.biome)}></span>
              <span class="name">{biomeName(entry.biome)}</span>
              <span class="value">{formatShare(entry.share)}</span>
              <span class="value area">{formatArea(entry.areaKm2)}</span>
            </li>
          {/each}
        </ul>
        {#if stats.shares.length > TOP_BIOMES}
          <button class="link" onclick={() => (allBiomes = !allBiomes)}>
            {allBiomes ? t.worldInfo.showFewer : t.worldInfo.showAll({ count: stats.shares.length })}
          </button>
        {/if}
      {:else}
        <p class="muted">{t.worldInfo.measuring}</p>
      {/if}

      <div class="actions">
        <button onclick={reroll} disabled={rerolling} title={t.worldInfo.rerollHelp}
          >{rerolling ? t.worldInfo.sending : t.worldInfo.reroll}</button
        >
        <button onclick={onVary} title={t.worldInfo.varyHelp}>{t.worldInfo.vary}</button>
        <button onclick={() => copy(cliCommand(meta), t.worldInfo.command)} title={t.worldInfo.copyCommandHelp}
          >{t.worldInfo.copyCommand}</button
        >
        <button onclick={() => copy(link(), t.worldInfo.link)} title={t.worldInfo.copyLinkHelp}>{t.worldInfo.copyLink}</button>
        <button class="danger" onclick={onDelete}>{t.worldInfo.delete}</button>
      </div>

      <label class="compare">
        <span>{t.worldInfo.compareWith}</span>
        <select value={compareId ?? ''} onchange={(e) => onCompare(e.currentTarget.value || null)}>
          <option value="">{t.worldInfo.nothing}</option>
          {#each others as other (other.id)}
            <option value={other.id}>{other.id}</option>
          {/each}
        </select>
      </label>

      <div class="downloads">
        <span>{t.worldInfo.download}</span>
        <a href={worldFile(world.id, 'world.png', summary?.modified_ms)} download={`${world.id}.png`}>{t.worldInfo.mapPng}</a>
        <a href={worldFile(world.id, 'world.svg', summary?.modified_ms)} download={`${world.id}.svg`}>{t.worldInfo.svg}</a>
        <a href={worldFile(world.id, 'legend.png', summary?.modified_ms)} download={`${world.id}-legend.png`}>{t.worldInfo.legend}</a>
        <button class="link" onclick={zip} disabled={zipping}>{zipping ? t.worldInfo.zipping : t.worldInfo.rawData}</button>
      </div>
    {:else if world.error}
      <p class="error">{world.error}</p>
    {:else}
      <p class="muted">{t.worldInfo.loading}</p>
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
