<script lang="ts">
  // Atmosphere and climate: pressure and gases, surface temperature, the
  // liquid on the surface, life; for a giant, its clouds, winds and poles.
  import './facts.css';
  import { t } from '../../lib/i18n/index.svelte';
  import { atmosphereLine } from '../../lib/planetCard';
  import { formatAtmosphere, formatInt, formatSurfaceTemperature, withUnit } from '../../lib/format';
  import Glossed from '../Glossed.svelte';
  import CauseTree from './CauseTree.svelte';
  import { treeFor } from '../../lib/causes';
  import type { Quantity } from '../../lib/api/types/Quantity';
  import type { CardSource } from '../../lib/cardSource';
  import type { InputField } from '../../lib/api/types/InputField';

  let { world, oninput }: { world: CardSource; oninput?: ((field: InputField) => void) | undefined } = $props();

  const explanation = $derived(world.shownExtras?.explanation ?? null);
  /** The value whose causes are unfolded: one at a time in a card. */
  let open = $state<Quantity | null>(null);
  const toggle = (q: Quantity) => () => (open = open === q ? null : q);
  // Trees start closed on every body.
  // A preview answer is a new object for the same body: only the id counts.
  const bodyId = $derived(world.systemBody);
  $effect(() => {
    void bodyId;
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
      <dt title={t.worldInfo.giantCloudsHelp}><Glossed text={t.worldInfo.giantClouds} /></dt>
      <dd>{t.starView.cloudClasses[giant.class] ?? ''} · {withUnit(formatInt(giant.effective_k), 'K')}</dd>
      <dt title={t.worldInfo.giantWindsHelp}><Glossed text={t.worldInfo.giantWinds} /></dt>
      <dd>
        <Glossed text={t.worldInfo.giantWindsValue({
          jets: giant.jets,
          west: withUnit(formatInt(giant.wind_west), 'm/s'),
          east: withUnit(formatInt(giant.wind_east), 'm/s'),
        })} />
      </dd>
      {#if giant.great_spot}
        <dt><Glossed text={t.cards.spot} /></dt>
        <dd><Glossed text={giant.kind === 'ice' ? t.worldInfo.darkSpot : t.worldInfo.greatSpot} /></dd>
      {/if}
      <dt title={t.worldInfo.polesHelp}><Glossed text={t.worldInfo.poles} /></dt>
      <dd>
        <Glossed text={t.worldInfo.polesValue({
          north: t.worldInfo.pole({ style: giant.poles[0], cyclones: giant.polar_cyclones[0] }),
          south: t.worldInfo.pole({ style: giant.poles[1], cyclones: giant.polar_cyclones[1] }),
        })} />
      </dd>
    {:else}
      {#if air || meta}
        <dt><Glossed text={t.cards.pressure} /></dt>
        <dd><Glossed text={air ?? (meta ? formatAtmosphere(meta) : '')} /><CauseTree tree={treeFor(explanation, 'pressure')} open={open === 'pressure'} ontoggle={toggle('pressure')} {oninput} /></dd>
      {/if}
      {#if temperature}
        <dt><Glossed text={t.card.surfaceTemperature} /></dt>
        <dd><Glossed text={temperature} /><CauseTree tree={treeFor(explanation, 'surface_temperature')} open={open === 'surface_temperature'} ontoggle={toggle('surface_temperature')} {oninput} /></dd>
      {/if}
      {#if surface}
        <dt><Glossed text={t.cards.liquid} /></dt>
        <dd><Glossed text={t.cards.liquids[surface.climate.liquid]} /><CauseTree tree={treeFor(explanation, 'liquid')} open={open === 'liquid'} ontoggle={toggle('liquid')} {oninput} /></dd>
        <dt><Glossed text={t.cards.life} /></dt>
        <dd><Glossed text={surface.life.oxygenated ? t.cards.lifeOxygenated : surface.life.arose ? t.cards.lifeArose : t.cards.lifeNone} /></dd>
      {/if}
    {/if}
  </dl>
{/if}
