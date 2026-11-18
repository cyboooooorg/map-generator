<script lang="ts">
  import Glossed from '../Glossed.svelte';
  // Interior: size and mass, density and gravity, what the body is made of,
  // its core and magnetic field, its tectonics and internal heat.
  import './facts.css';
  import { t } from '../../lib/i18n/index.svelte';
  import { bulkLine, formatPower, geysersLine, momentOfInertiaLine, oceanUnderIce, tidalHeatLine } from '../../lib/planetCard';
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
  const tidal = $derived(model ? tidalHeatLine(model, explanation) : null);
  const inertia = $derived(model ? momentOfInertiaLine(model, body?.kind === 'moon') : null);
  const geysers = $derived(model ? geysersLine(model) : null);
  const power = (w: number) => (w > 0 ? formatPower(w) : withUnit('0', 'TW'));
</script>

{#if body}
  <dl class="facts">
    <dt><Glossed text={t.cards.mass} /></dt>
    {#if giant}
      <dd>
        <Glossed text={t.worldInfo.giantMassValue({
          jupiter: withUnit(formatUpTo(giant.mass_earth / 317.8, 2), 'M♃'),
          earth: withUnit(formatInt(giant.mass_earth), 'M⊕'),
        })} />
      </dd>
    {:else}
      <dd><Glossed text={formatMassEarth(body.physical.mass_earth)} /></dd>
    {/if}
    <dt><Glossed text={t.cards.radius} /></dt>
    {#if giant}
      <dd>
        <Glossed text={t.worldInfo.giantMassValue({
          jupiter: withUnit(formatUpTo(giant.radius_earth / 11.21, 2), 'RJ'),
          earth: withUnit(formatUpTo(giant.radius_earth, 1), 'R⊕'),
        })} />
      </dd>
    {:else}
      <dd>
        {withUnit(formatInt(body.physical.radius_km), 'km')}{#if body.physical.axes_km}{' '}(<span class="axes-help" title={t.card.axesHelp}><Glossed text={t.card.axes({
            a: withUnit(formatInt(2 * body.physical.axes_km[0]), 'km'),
            b: formatInt(2 * body.physical.axes_km[1]),
            c: formatInt(2 * body.physical.axes_km[2]),
          })} /></span>){/if}
      </dd>
    {/if}
    <dt><Glossed text={t.cards.density} /></dt>
    <dd><Glossed text={withUnit(formatUpTo(body.physical.density, 2), 'g/cm³')} /></dd>
    <dt><Glossed text={t.worldInfo.gravity} /></dt>
    <dd><Glossed text={withUnit(formatUpTo(body.physical.gravity, 2), 'g')} /></dd>
    {#if meta}
      <dt><Glossed text={t.worldInfo.circumference} /></dt>
      <dd><Glossed text={withUnit(formatInt(meta.circumference_km), 'km')} /></dd>
    {/if}
    {#if layers}
      <dt><Glossed text={t.cards.layers} /></dt>
      <dd><Glossed text={layers} /><CauseTree tree={treeFor(explanation, 'ocean_depth')} open={open === 'ocean_depth'} ontoggle={toggle('ocean_depth')} {oninput} /></dd>
    {/if}
    {#if model?.surface}
      <dt><Glossed text={t.card.regime} /></dt>
      <dd><Glossed text={t.regimes[model.surface.regime]} /><CauseTree tree={treeFor(explanation, 'regime')} open={open === 'regime'} ontoggle={toggle('regime')} {oninput} /></dd>
    {/if}
    {#if model?.surface}
      <dt><Glossed text={t.cards.waterKept} /></dt>
      <dd><Glossed text={withUnit(formatUpTo(model.surface.water.oceans, 3), t.causes.units.water_kept)} /><CauseTree tree={treeFor(explanation, 'water_kept')} open={open === 'water_kept'} ontoggle={toggle('water_kept')} {oninput} /></dd>
    {/if}
    {#if core}
      <dt><Glossed text={t.cards.core} /></dt>
      <dd><Glossed text={t.cards.coreStates[core.state]} /></dd>
      <dt><Glossed text={t.cards.field} /></dt>
      <dd><Glossed text={core.dynamo ? withUnit(formatUpTo(core.field_ut, 1), 'µT') : t.cards.noField} /><CauseTree tree={treeFor(explanation, 'magnetic_field')} open={open === 'magnetic_field'} ontoggle={toggle('magnetic_field')} {oninput} /></dd>
    {/if}
    {#if model}
      <dt><Glossed text={t.cards.heat} /></dt>
      <dd><Glossed text={t.cards.heatValue({ radiogenic: power(model.heat.radiogenic_w), tidal: power(model.heat.tidal_w), secular: power(model.heat.secular_w) })} /></dd>
    {/if}
    {#if tidal && (body?.kind === 'moon' || model?.ice_shell)}
      <dt><Glossed text={t.card.tidalHeat} /></dt>
      <dd><Glossed text={tidal} /><CauseTree tree={treeFor(explanation, 'heat_flux')} open={open === 'heat_flux'} ontoggle={toggle('heat_flux')} {oninput} /></dd>
    {/if}
    {#if inertia}
      <dt><Glossed text={t.card.momentOfInertia} /></dt>
      <dd><Glossed text={inertia} /><CauseTree tree={treeFor(explanation, 'moment_of_inertia')} open={open === 'moment_of_inertia'} ontoggle={toggle('moment_of_inertia')} {oninput} /></dd>
    {/if}
    {#if geysers}
      <dt><Glossed text={t.card.geysers} /></dt>
      <dd><Glossed text={geysers} /></dd>
    {/if}
    {#if giant}
      <dt><Glossed text={t.cards.flattening} /></dt>
      <dd><Glossed text={formatUpTo(giant.flattening, 3)} /></dd>
      <dt title={t.worldInfo.giantHeatHelp}><Glossed text={t.worldInfo.giantHeat} /></dt>
      <dd><Glossed text={t.worldInfo.giantHeatValue({ ratio: formatUpTo(giant.heat_ratio, 1) })} /></dd>
    {/if}
  </dl>
{/if}
