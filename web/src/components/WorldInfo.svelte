<script lang="ts">
  import { getLocale, t } from '../lib/i18n/index.svelte';
  import { biomeName, presetDescription, presetName } from '../lib/i18n/names.svelte';
  import { worldFile } from '../lib/api/client';
  import { downloadRawData, downloadSvg } from '../lib/download';
  import { cliCommand, copyText, formatAgo, formatArea, formatAtmosphere, formatAu, formatEccentricity, formatGeology, formatInt, formatNumber, formatOrbit, formatShare, formatStar, formatStarlight, formatUpTo, formatYear, surfaceColour, withUnit } from '../lib/format';
  import { unitTooltip } from '../lib/units';
  import { loadSetting, saveSetting } from '../lib/settings';
  import { toasts } from '../lib/toasts.svelte';
  import type { LoadedWorld } from '../lib/world.svelte';
  import { moonsOf } from '../lib/moons';
  import { resonanceRatio, resonanceTurns } from '../lib/spinResonance';

  interface Props {
    world: LoadedWorld;
    /** When the body's maps were made (ms), for the card and the download links. */
    revision?: number | null;
    onVary: () => void;
    /** Generates the same world again with a new random seed. */
    onReroll: () => Promise<void>;
    /** The link Copy link copies: the worlds shown and the view. */
    link: () => string;
    onDelete: () => void;
    /** The title instead of the world's own: a system body's name. */
    title?: string | null;
  }

  let { world, revision = null, onVary, onReroll, link, onDelete, title: forcedTitle = null }: Props = $props();

  let open = $state(loadSetting('worldInfoOpen', true));
  let allBiomes = $state(false);
  let zipping = $state(false);
  let drawingSvg = $state(false);
  let rerolling = $state(false);
  $effect(() => saveSetting('worldInfoOpen', open));

  // The world's, also while the viewer shows one of its moons.
  const meta = $derived(world.worldMeta);
  const stats = $derived(world.worldStats);
  const rivers = $derived(world.worldRivers);
  const worldId = $derived(world.worldId);
  // The world's metadata is set only while the world itself is shown: its
  // resolution is then the one shown.
  const size = $derived(meta ? world.size : null);
  const moons = $derived(moonsOf(meta));
  const TOP_BIOMES = 6;
  const title = $derived(forcedTitle ?? (meta ? presetName(meta.preset) : worldId));

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
  async function exportSvg(): Promise<void> {
    if (!worldId || !size) {
      return;
    }
    drawingSvg = true;
    try {
      await downloadSvg(worldId, size);
    } catch (e) {
      toasts.error(t.worldInfo.svgFailed({ error: String(e) }));
    } finally {
      drawingSvg = false;
    }
  }
  async function zip(): Promise<void> {
    if (!worldId || !meta || !size) {
      return;
    }
    zipping = true;
    try {
      await downloadRawData(worldId, size, meta);
    } catch (e) {
      toasts.error(t.worldInfo.archiveFailed({ error: String(e) }));
    } finally {
      zipping = false;
    }
  }
</script>

{#if worldId}
  <details class="card" bind:open>
    <summary>
      <span class="title">
        <strong>{title}</strong>
        <small
          >{[meta ? presetName(meta.preset) : null, forcedTitle ? null : worldId]
            .filter(Boolean)
            .join(' · ')}</small
        >
      </span>
    </summary>

    {#if meta}
      <p class="description">{presetDescription(meta.preset)}</p>
      <dl>
        <dt>{t.worldInfo.seed}</dt>
        <dd>{meta.seed}</dd>
        {#if meta.giant}
          {@const g = meta.giant}
          <dt title={t.worldInfo.giantMassHelp}>{t.worldInfo.giantMass}</dt>
          <dd>
            {t.worldInfo.giantMassValue({
              jupiter: withUnit(formatUpTo(g.mass_earth / 317.8, 2), 'MJ'),
              earth: withUnit(formatInt(g.mass_earth), 'M⊕'),
            })}
          </dd>
          <dt>{t.worldInfo.giantRadius}</dt>
          <dd>
            {t.worldInfo.giantMassValue({
              jupiter: withUnit(formatUpTo(g.radius_earth / 11.21, 2), 'RJ'),
              earth: withUnit(formatUpTo(g.radius_earth, 1), 'R⊕'),
            })} · {withUnit(formatUpTo(g.density, 2), 'g/cm³')}
          </dd>
          <dt title={t.worldInfo.giantDayHelp}>{t.worldInfo.giantDay}</dt>
          <dd>
            {g.rotation_hours < 48
              ? withUnit(formatUpTo(g.rotation_hours, 1), 'h')
              : formatYear(g.rotation_hours / 24)}{#if g.hot}
              · {t.worldInfo.locked}{/if} · {t.worldInfo.flatteningValue({ value: formatUpTo(g.flattening, 3) })}
          </dd>
          <dt title={t.worldInfo.giantCloudsHelp}>{t.worldInfo.giantClouds}</dt>
          <dd>{t.starView.cloudClasses[g.class] ?? ''} · {withUnit(formatInt(g.effective_k), 'K')}</dd>
          <dt title={t.worldInfo.giantWindsHelp}>{t.worldInfo.giantWinds}</dt>
          <dd>
            {t.worldInfo.giantWindsValue({
              jets: g.jets,
              west: withUnit(formatInt(g.wind_west), 'm/s'),
              east: withUnit(formatInt(g.wind_east), 'm/s'),
            })}
          </dd>
          <dt title={t.worldInfo.giantHeatHelp}>{t.worldInfo.giantHeat}</dt>
          <dd>{t.worldInfo.giantHeatValue({ ratio: formatUpTo(g.heat_ratio, 1) })}</dd>
          <dt>{t.worldInfo.giantRings}</dt>
          <dd>
            {g.rings
              ? t.worldInfo.giantRingsValue({
                  kind: (g.rings.bright ? t.starView.rings['bright'] : t.starView.rings['faint']) ?? '',
                  inner: formatUpTo(g.rings.inner, 2),
                  outer: formatUpTo(g.rings.outer, 2),
                })
              : (t.starView.rings['none'] ?? '')}{#if g.great_spot}
              · {g.kind === 'ice' ? t.worldInfo.darkSpot : t.worldInfo.greatSpot}{/if}
          </dd>
          <dt title={t.worldInfo.polesHelp}>{t.worldInfo.poles}</dt>
          <dd>
            {t.worldInfo.polesValue({
              north: t.worldInfo.pole({ style: g.poles[0], cyclones: g.polar_cyclones[0] }),
              south: t.worldInfo.pole({ style: g.poles[1], cyclones: g.polar_cyclones[1] }),
            })}
          </dd>
        {:else}
          <dt>{t.worldInfo.seaLevel}</dt>
          <dd>{formatNumber(meta.sea_level, 3)}</dd>
          <dt>{t.worldInfo.volcanism}</dt>
          <dd>{formatNumber(meta.volcanic_intensity, 2)}</dd>
          <dt>{t.worldInfo.erosion}</dt>
          <dd>{formatNumber(meta.erosion, 2)}</dd>
        {/if}
        <dt>{t.worldInfo.circumference}</dt>
        <dd>{withUnit(formatInt(meta.circumference_km), 'km')}</dd>
        <dt>{t.worldInfo.axialTilt}</dt>
        <dd>{formatNumber(meta.axial_tilt_deg, 1)}°</dd>
        <dt>{t.worldInfo.gravity}</dt>
        <dd class="unit-help" title={unitTooltip('g')}>{withUnit(formatNumber(meta.gravity_modifier, 2), 'g')}</dd>
        {#if !meta.giant}
          <dt>{t.worldInfo.atmosphere}</dt>
          {#if meta.pressure_bar > 0}
            <dd class="unit-help" title={unitTooltip('bar')}>{formatAtmosphere(meta)}</dd>
          {:else}
            <dd>{formatAtmosphere(meta)}</dd>
          {/if}
          <dt>{t.worldInfo.geology}</dt>
          <dd class="unit-help" title={unitTooltip('Ga', 'Ma')}>{formatGeology(meta)}</dd>
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
              · <span class="unit-help" title={t.worldInfo.lockedHelp}>{t.worldInfo.locked}</span>{:else if resonanceTurns({ resonance_turns: star.spin_turns }) > 0}
              {@const ratio = resonanceRatio(resonanceTurns({ resonance_turns: star.spin_turns }))}
              · <span class="unit-help" title={t.worldInfo.resonantHelp({ ratio })}>{t.worldInfo.resonant({ ratio })}</span>{/if}
          </dd>
        {/if}
        {#if meta.moons}
          <dt title={t.worldInfo.moonsHelp}>{t.worldInfo.moons}</dt>
          <dd>
            {moons.length === 0
              ? t.worldInfo.noMoon
              : t.worldInfo.moonsValue({
                  count: moons.length,
                  names: moons.map((m) => m.name).join(', '),
                })}{#if meta.moons.binary}
              · {t.worldInfo.binary}{/if}
          </dd>
          {#if !meta.giant}
          <dt title={t.worldInfo.tidesHelp}>{t.worldInfo.tides}</dt>
          <dd>
            {t.worldInfo.tidesValue({ tide: formatUpTo(meta.moons.tide, meta.moons.tide < 1 ? 2 : 1) })} ·
            <span title={t.worldInfo.tiltHelp}>{meta.moons.tilt_steadied ? t.worldInfo.tiltSteady : t.worldInfo.tiltFree}</span>
          </dd>
          {/if}
        {/if}
        {#if meta.host}
          {@const h = meta.host}
          <dt title={t.worldInfo.hostHelp}>{t.worldInfo.host}</dt>
          <dd>
            {t.worldInfo.hostValue({
              kind: t.starView.giantKinds[h.giant.kind] ?? '',
              mass: withUnit(formatUpTo(h.giant.mass_earth / 317.8, 2), 'MJ'),
              radii: formatUpTo(h.distance_radii, 1),
              km: withUnit(formatInt(h.distance_km), 'km'),
            })}
          </dd>
          <dt title={t.worldInfo.hostDayHelp}>{t.worldInfo.hostDay}</dt>
          <dd>{formatYear(h.period_days)} · {t.worldInfo.hostSky({ size: `${formatUpTo(h.angular_diameter_deg, 1)}°` })}</dd>
          <dt title={t.worldInfo.tidalHelp}>{t.worldInfo.tidal}</dt>
          <dd>
            {withUnit(formatUpTo(h.tidal_flux, h.tidal_flux < 1 ? 2 : 0), 'W/m²')}{#if h.inside_edge}
              · <strong>{t.worldInfo.insideEdge}</strong>{/if}
          </dd>
        {/if}
        {#if meta.giants}
          <dt title={t.worldInfo.giantsHelp}>{t.worldInfo.giants}</dt>
          <dd>
            {meta.giants.giants.length === 0
              ? t.worldInfo.noGiants
              : t.worldInfo.giantsValue({
                  count: meta.giants.giants.length,
                  names: meta.giants.giants.map((g) => g.name).join(', '),
                })} · {t.worldInfo.impactsValue({ factor: `${formatUpTo(meta.giants.impact_factor, 2)}×` })}
          </dd>
        {/if}
        {#if stats && !meta.giant}
          <dt>{t.worldInfo.surface}</dt>
          <dd>{formatArea(stats.surfaceKm2)}</dd>
          <dt>{t.worldInfo.landWater}</dt>
          <dd>{formatShare(stats.land)} / {formatShare(stats.water)}</dd>
        {/if}
        {#if rivers && !meta.giant}
          <dt>{t.worldInfo.rivers}</dt>
          <dd>{formatInt(rivers.file.rivers.length)}</dd>
        {/if}
        {#if revision}
          <dt>{t.worldInfo.generated}</dt>
          <dd title={new Date(revision).toLocaleString(getLocale())}>{formatAgo(revision)}</dd>
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
      {:else if world.worldError || (world.id === worldId && world.error)}
        <!-- The statistics will not come: say why rather than measuring for ever. -->
        <p class="muted">{world.worldError ?? world.error}</p>
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



      <div class="downloads">
        <span>{t.worldInfo.download}</span>
        {#if size}
          <a href={worldFile(worldId, size, 'world.png', world.revision)} download={`${worldId}.png`}>{t.worldInfo.mapPng}</a>
          <a href={worldFile(worldId, size, 'legend.png', world.revision)} download={`${worldId}-legend.png`}>{t.worldInfo.legend}</a>
        {/if}
        {#if size}
          <button class="link" onclick={exportSvg} disabled={drawingSvg} title={t.worldInfo.exportSvgHelp}
            >{drawingSvg ? t.worldInfo.drawingSvg : t.worldInfo.exportSvg}</button
          >
        {/if}
        <button class="link" onclick={zip} disabled={zipping}>{zipping ? t.worldInfo.zipping : t.worldInfo.rawData}</button>
      </div>
    {:else if world.error || world.worldError}
      <p class="error">{world.worldError ?? world.error}</p>
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
