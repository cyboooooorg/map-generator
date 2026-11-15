<script lang="ts">
  // Interior: size and mass, density and gravity, what the body is made of,
  // its core and magnetic field, its tectonics and internal heat.
  import './facts.css';
  import { t } from '../../lib/i18n/index.svelte';
  import { bulkLine, oceanUnderIce } from '../../lib/planetCard';
  import { formatInt, formatMassEarth, formatUpTo, withUnit } from '../../lib/format';
  import CauseTree from './CauseTree.svelte';
  import { treeFor } from '../../lib/causes';
  import type { Quantity } from '../../lib/api/types/Quantity';
  import type { CardSource } from '../../lib/cardSource';
  import type { InputField } from '../../lib/api/types/InputField';

  let { world, oninput }: { world: CardSource; oninput?: ((field: InputField) => void) | undefined } = $props();

  const explanation = $derived(world.shownExtras?.explanation ?? null);
  /** The value whose causes are unfolded: one at a time in a card. */
  let open = $state<Quantity | null>(null);
  /** Whether the preview just moved this value. */
  const moved = (q: Quantity) => world.changed?.has(q) ?? false;
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
  const core = $derived(model?.surface?.core ?? null);

  const share = (x: number) => withUnit(formatInt(Math.round(x * 100)), '%');
  /** What the layers are made of, by share of the mass; the ocean under the ice after it. */
  const layers = $derived.by(() => {
    if (!model) return body?.physical.bulk ? bulkLine(body.physical.bulk) : null;
    const total = model.interior.layers.reduce((sum, l) => sum + l.mass_earth, 0);
    const made = total > 0 ? model.interior.layers.map((l) => `${t.cards.materials[l.material]} ${share(l.mass_earth / total)}`) : [];
    const ocean = oceanUnderIce(model);
    return [...made, ...(ocean ? [ocean] : [])].join(' · ') || null;
  });
  const teraWatts = (w: number) => withUnit(formatUpTo(w / 1e12, 2), 'TW');
</script>

{#if body}
  <dl class="facts">
    <dt>{t.cards.mass}</dt>
    {#if giant}
      <dd>
        {t.worldInfo.giantMassValue({
          jupiter: withUnit(formatUpTo(giant.mass_earth / 317.8, 2), 'MJ'),
          earth: withUnit(formatInt(giant.mass_earth), 'M⊕'),
        })}
      </dd>
    {:else}
      <dd>{formatMassEarth(body.physical.mass_earth)}</dd>
    {/if}
    <dt>{t.cards.radius}</dt>
    {#if giant}
      <dd>
        {t.worldInfo.giantMassValue({
          jupiter: withUnit(formatUpTo(giant.radius_earth / 11.21, 2), 'RJ'),
          earth: withUnit(formatUpTo(giant.radius_earth, 1), 'R⊕'),
        })}
      </dd>
    {:else}
      <dd>
        {withUnit(formatInt(body.physical.radius_km), 'km')}{#if body.physical.axes_km}{' '}(<span class="unit-help" title={t.card.axesHelp}>{t.card.axes({
            a: withUnit(formatInt(2 * body.physical.axes_km[0]), 'km'),
            b: formatInt(2 * body.physical.axes_km[1]),
            c: formatInt(2 * body.physical.axes_km[2]),
          })}</span>){/if}
      </dd>
    {/if}
    <dt>{t.cards.density}</dt>
    <dd>{withUnit(formatUpTo(body.physical.density, 2), 'g/cm³')}</dd>
    <dt>{t.worldInfo.gravity}</dt>
    <dd>{withUnit(formatUpTo(body.physical.gravity, 2), 'g')}</dd>
    {#if meta}
      <dt>{t.worldInfo.circumference}</dt>
      <dd>{withUnit(formatInt(meta.circumference_km), 'km')}</dd>
    {/if}
    {#if layers}
      <dt>{t.cards.layers}</dt>
      <dd>{layers}</dd>
    {/if}
    {#if model?.surface}
      <dt>{t.card.regime}</dt>
      <dd class:moved={moved('regime')}>{t.regimes[model.surface.regime]}<CauseTree tree={treeFor(explanation, 'regime')} open={open === 'regime'} ontoggle={toggle('regime')} {oninput} /></dd>
    {/if}
    {#if model?.surface}
      <dt>{t.cards.waterKept}</dt>
      <dd class:moved={moved('water_kept')}>{withUnit(formatUpTo(model.surface.water.oceans, 3), t.causes.units.water_kept)}<CauseTree tree={treeFor(explanation, 'water_kept')} open={open === 'water_kept'} ontoggle={toggle('water_kept')} {oninput} /></dd>
    {/if}
    {#if core}
      <dt>{t.cards.core}</dt>
      <dd>{t.cards.coreStates[core.state]}</dd>
      <dt>{t.cards.field}</dt>
      <dd class:moved={moved('magnetic_field')}>{core.dynamo ? withUnit(formatUpTo(core.field_ut, 1), 'µT') : t.cards.noField}<CauseTree tree={treeFor(explanation, 'magnetic_field')} open={open === 'magnetic_field'} ontoggle={toggle('magnetic_field')} {oninput} /></dd>
    {/if}
    {#if model}
      <dt>{t.cards.heat}</dt>
      <dd>{t.cards.heatValue({ radiogenic: teraWatts(model.heat.radiogenic_w), tidal: teraWatts(model.heat.tidal_w) })}</dd>
    {/if}
    {#if giant}
      <dt>{t.cards.flattening}</dt>
      <dd>{formatUpTo(giant.flattening, 3)}</dd>
      <dt title={t.worldInfo.giantHeatHelp}>{t.worldInfo.giantHeat}</dt>
      <dd>{t.worldInfo.giantHeatValue({ ratio: formatUpTo(giant.heat_ratio, 1) })}</dd>
    {/if}
  </dl>
{/if}
