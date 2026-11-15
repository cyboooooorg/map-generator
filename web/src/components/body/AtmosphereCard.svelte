<script lang="ts">
  // Atmosphere and climate: pressure and gases, surface temperature, the
  // liquid on the surface, life; for a giant, its clouds, winds and poles.
  import './facts.css';
  import { t } from '../../lib/i18n/index.svelte';
  import { atmosphereLine } from '../../lib/planetCard';
  import { formatAtmosphere, formatInt, formatSurfaceTemperature, withUnit } from '../../lib/format';
  import { unitTooltip } from '../../lib/units';
  import CauseTree from './CauseTree.svelte';
  import { treeFor } from '../../lib/causes';
  import type { Quantity } from '../../lib/api/types/Quantity';
  import type { LoadedWorld } from '../../lib/world.svelte';

  let { world }: { world: LoadedWorld } = $props();

  const explanation = $derived(world.shownExtras?.explanation ?? null);
  /** The value whose causes are unfolded: one at a time in a card. */
  let open = $state<Quantity | null>(null);
  const toggle = (q: Quantity) => () => (open = open === q ? null : q);
  // Trees start closed on every body.
  $effect(() => {
    void world.systemBody;
    open = null;
  });

  const body = $derived(world.shownSystemBody);
  const meta = $derived(world.meta);
  const giant = $derived(meta?.giant ?? null);
  const model = $derived(body?.model ?? null);
  const surface = $derived(model?.surface ?? null);
  const air = $derived(model ? atmosphereLine(model) : null);
  const temperature = $derived(model ? formatSurfaceTemperature(model) : null);
</script>

{#if body}
  <dl class="facts">
    {#if giant}
      <dt title={t.worldInfo.giantCloudsHelp}>{t.worldInfo.giantClouds}</dt>
      <dd>{t.starView.cloudClasses[giant.class] ?? ''} · {withUnit(formatInt(giant.effective_k), 'K')}</dd>
      <dt title={t.worldInfo.giantWindsHelp}>{t.worldInfo.giantWinds}</dt>
      <dd>
        {t.worldInfo.giantWindsValue({
          jets: giant.jets,
          west: withUnit(formatInt(giant.wind_west), 'm/s'),
          east: withUnit(formatInt(giant.wind_east), 'm/s'),
        })}
      </dd>
      {#if giant.great_spot}
        <dt>{t.cards.spot}</dt>
        <dd>{giant.kind === 'ice' ? t.worldInfo.darkSpot : t.worldInfo.greatSpot}</dd>
      {/if}
      <dt title={t.worldInfo.polesHelp}>{t.worldInfo.poles}</dt>
      <dd>
        {t.worldInfo.polesValue({
          north: t.worldInfo.pole({ style: giant.poles[0], cyclones: giant.polar_cyclones[0] }),
          south: t.worldInfo.pole({ style: giant.poles[1], cyclones: giant.polar_cyclones[1] }),
        })}
      </dd>
    {:else}
      {#if air || meta}
        <dt>{t.cards.pressure}</dt>
        <dd><span class="unit-help" title={unitTooltip('bar')}>{air ?? (meta ? formatAtmosphere(meta) : '')}</span><CauseTree tree={treeFor(explanation, 'pressure')} open={open === 'pressure'} ontoggle={toggle('pressure')} /></dd>
      {/if}
      {#if temperature}
        <dt>{t.card.surfaceTemperature}</dt>
        <dd>{temperature}<CauseTree tree={treeFor(explanation, 'surface_temperature')} open={open === 'surface_temperature'} ontoggle={toggle('surface_temperature')} /></dd>
      {/if}
      {#if surface}
        <dt>{t.cards.liquid}</dt>
        <dd>{t.cards.liquids[surface.climate.liquid]}<CauseTree tree={treeFor(explanation, 'liquid')} open={open === 'liquid'} ontoggle={toggle('liquid')} /></dd>
        <dt>{t.cards.life}</dt>
        <dd>{surface.life.oxygenated ? t.cards.lifeOxygenated : surface.life.arose ? t.cards.lifeArose : t.cards.lifeNone}</dd>
      {/if}
    {/if}
  </dl>
{/if}
