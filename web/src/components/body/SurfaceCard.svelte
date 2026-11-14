<script lang="ts">
  // Surface (once the maps exist): sea level, volcanism, erosion and age,
  // land and water, rivers, and the giants that shape the impacts.
  import './facts.css';
  import { t } from '../../lib/i18n/index.svelte';
  import { formatArea, formatInt, formatNumber, formatShare, formatSurfaceAge, formatUpTo } from '../../lib/format';
  import { unitTooltip } from '../../lib/units';
  import type { LoadedWorld } from '../../lib/world.svelte';

  let { world }: { world: LoadedWorld } = $props();

  const body = $derived(world.shownSystemBody);
  const meta = $derived(world.meta);
  const stats = $derived(world.stats);
  const rivers = $derived(world.rivers);
</script>

{#if meta && body}
  <dl class="facts">
    <dt>{t.worldInfo.seaLevel}</dt>
    <dd>{formatNumber(meta.sea_level, 3)}</dd>
    <dt>{t.worldInfo.volcanism}</dt>
    <dd>{formatNumber(meta.volcanic_intensity, 2)}</dd>
    <dt>{t.worldInfo.erosion}</dt>
    <dd>{formatNumber(meta.erosion, 2)}</dd>
    <dt>{t.cards.surfaceAge}</dt>
    <dd class="unit-help" title={unitTooltip('Ga', 'Ma')}>{formatSurfaceAge(body.params.surface_age_ga)}</dd>
    {#if stats}
      <dt>{t.worldInfo.surface}</dt>
      <dd>{formatArea(stats.surfaceKm2)}</dd>
      <dt>{t.worldInfo.landWater}</dt>
      <dd>{formatShare(stats.land)} / {formatShare(stats.water)}</dd>
    {/if}
    {#if rivers}
      <dt>{t.worldInfo.rivers}</dt>
      <dd>{formatInt(rivers.file.rivers.length)}</dd>
    {/if}
    {#if meta.giants}
      <dt title={t.worldInfo.giantsHelp}>{t.cards.impacts}</dt>
      <dd>
        {meta.giants.giants.length === 0
          ? t.worldInfo.noGiants
          : t.worldInfo.giantsValue({ count: meta.giants.giants.length, names: meta.giants.giants.map((g) => g.name).join(', ') })} · {t.worldInfo.impactsValue({
          factor: `${formatUpTo(meta.giants.impact_factor, 2)}×`,
        })}
      </dd>
    {/if}
  </dl>
{/if}
