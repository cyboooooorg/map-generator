<script lang="ts">
  // Surface (once the maps exist): sea level, volcanism, erosion and age,
  // land and water, rivers, and the giants that shape the impacts.
  import './facts.css';
  import { t } from '../../lib/i18n/index.svelte';
  import { formatArea, formatInt, formatNumber, formatShare, formatSurfaceAge, formatUpTo, withUnit } from '../../lib/format';
  import { unitTooltip } from '../../lib/units';
  import { treeFor } from '../../lib/causes';
  import { unitsFor } from '../../lib/surfaceUnits';
  import CauseTree from './CauseTree.svelte';
  import type { CardSource } from '../../lib/cardSource';
  import type { InputField } from '../../lib/api/types/InputField';

  let { world, oninput }: { world: CardSource; oninput?: ((field: InputField) => void) | undefined } = $props();

  const explanation = $derived(world.shownExtras?.explanation ?? null);
  /** Whether the causes of the surface age are unfolded. */
  let open = $state(false);
  // Trees start closed on every body; a preview answer is a new object for the same body: only the id counts.
  const bodyId = $derived(world.systemBody);
  $effect(() => {
    void bodyId;
    open = false;
  });
  const body = $derived(world.shownSystemBody);
  /** The parts of the surface and their ages: shown only for a surface of several. */
  const units = $derived(unitsFor(body?.model?.terrain));
  const meta = $derived(world.meta);
  const stats = $derived(world.stats);
  const rivers = $derived(world.rivers);
  // The share counts every liquid biome (`Biome::is_water`): name the
  // model's liquid when it is not water, so methane seas are not "water".
  const liquid = $derived(body?.model?.surface?.climate.liquid ?? null);
  const landLabel = $derived(
    liquid === 'hydrocarbons' || liquid === 'lava' ? t.worldInfo.landLiquid({ liquid: t.cards.liquids[liquid] }) : t.worldInfo.landWater,
  );
</script>

{#if body}
  <dl class="facts">
    {#if meta}
      <dt>{t.worldInfo.seaLevel}</dt>
      <dd>{formatNumber(meta.sea_level, 3)}</dd>
      <dt>{t.worldInfo.volcanism}</dt>
      <dd>{formatNumber(meta.volcanic_intensity, 2)}</dd>
      <dt>{t.worldInfo.erosion}</dt>
      <dd>{formatNumber(meta.erosion, 2)}</dd>
    {/if}
    <dt>{t.cards.surfaceAge}</dt>
    <dd>
      <span class="unit-help" title={unitTooltip('Ga', 'Ma')}>{formatSurfaceAge(body.params.surface_age_ga)}</span><CauseTree
        tree={treeFor(explanation, 'surface_age')}
        {open}
        ontoggle={() => (open = !open)}
        {oninput}
      />
    </dd>
    {#if units.length > 1}
      <dt>{t.cards.terrainUnits}</dt>
      <dd>
        {#each units as unit}
          <div>{t.causes.renewalValue({ renewal: t.causes.renewals[unit.renewal], value: `${withUnit(formatInt(Math.round(unit.share * 100)), '%')} · ${formatSurfaceAge(unit.age_gyr)}` })}</div>
        {/each}
      </dd>
    {/if}
    {#if stats}
      <dt>{t.worldInfo.surface}</dt>
      <dd>{formatArea(stats.surfaceKm2)}</dd>
      <dt>{landLabel}</dt>
      <dd>{formatShare(stats.land)} / {formatShare(stats.water)}</dd>
    {/if}
    {#if rivers}
      <dt>{t.worldInfo.rivers}</dt>
      <dd>{formatInt(rivers.file.rivers.length)}</dd>
    {/if}
    {#if meta?.giants}
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
