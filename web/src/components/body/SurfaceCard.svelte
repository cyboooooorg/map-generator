<script lang="ts">
  // Surface (once the maps exist): sea level, volcanism, erosion and age,
  // land and water, rivers, and the giants that shape the impacts.
  import './facts.css';
  import { t } from '../../lib/i18n/index.svelte';
  import { formatArea, formatInt, formatNumber, formatShare, formatSurfaceAge, formatUpTo, withUnit } from '../../lib/format';
  import Glossed from '../Glossed.svelte';
  import { dominantFrost, treeFor } from '../../lib/causes';
  import { unitsFor } from '../../lib/surfaceUnits';
  import CauseTree from './CauseTree.svelte';
  import type { CardSource } from '../../lib/cardSource';
  import type { InputField } from '../../lib/api/types/InputField';

  let { world, oninput }: { world: CardSource; oninput?: ((field: InputField) => void) | undefined } = $props();

  const explanation = $derived(world.shownExtras?.explanation ?? null);
  /** Whether the causes of the surface age are unfolded. */
  let open = $state(false);
  /** Whether the causes of the Bond albedo are unfolded. */
  let albedoOpen = $state(false);
  // Trees start closed on every body; a preview answer is a new object for the same body: only the id counts.
  const bodyId = $derived(world.systemBody);
  $effect(() => {
    void bodyId;
    open = false;
    albedoOpen = false;
  });
  const body = $derived(world.shownSystemBody);
  /** The parts of the surface and their ages: shown only for a surface of several. */
  const units = $derived(unitsFor(body?.model?.terrain));
  const albedo = $derived(body?.model?.surface?.climate.bond_albedo ?? null);
  const albedoTree = $derived(treeFor(explanation, 'bond_albedo'));
  /** The frost that makes the albedo, named on the value line (none over a surface liquid). */
  const albedoFrost = $derived(dominantFrost(albedoTree, body?.model?.surface?.climate.liquid ?? null));
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
      <dt><Glossed text={t.worldInfo.seaLevel} /></dt>
      <dd><Glossed text={formatNumber(meta.sea_level, 3)} /></dd>
      <dt><Glossed text={t.worldInfo.volcanism} /></dt>
      <dd><Glossed text={formatNumber(meta.volcanic_intensity, 2)} /></dd>
      <dt><Glossed text={t.worldInfo.erosion} /></dt>
      <dd><Glossed text={formatNumber(meta.erosion, 2)} /></dd>
    {/if}
    <dt><Glossed text={t.cards.surfaceAge} /></dt>
    <dd>
      <Glossed text={formatSurfaceAge(body.params.surface_age_ga)} /><CauseTree
        tree={treeFor(explanation, 'surface_age')}
        {open}
        ontoggle={() => (open = !open)}
        {oninput}
      />
    </dd>
    {#if albedo !== null}
      <dt><Glossed text={t.cards.bondAlbedo} /></dt>
      <dd>
        <Glossed text={`${formatNumber(albedo, 2)}${albedoFrost ? ` · ${albedoFrost}` : ''}`} /><CauseTree tree={albedoTree} open={albedoOpen} ontoggle={() => (albedoOpen = !albedoOpen)} {oninput} />
      </dd>
    {/if}
    {#if units.length > 1}
      <dt><Glossed text={t.cards.terrainUnits} /></dt>
      <dd>
        {#each units as unit}
          <div><Glossed text={t.causes.renewalValue({ renewal: t.causes.renewals[unit.renewal], value: `${withUnit(formatInt(Math.round(unit.share * 100)), '%')} · ${formatSurfaceAge(unit.age_gyr)}` })} /></div>
        {/each}
      </dd>
    {/if}
    {#if stats}
      <dt><Glossed text={t.worldInfo.surface} /></dt>
      <dd><Glossed text={formatArea(stats.surfaceKm2)} /></dd>
      <dt><Glossed text={landLabel} /></dt>
      <dd>{formatShare(stats.land)} / {formatShare(stats.water)}</dd>
    {/if}
    {#if rivers}
      <dt><Glossed text={t.worldInfo.rivers} /></dt>
      <dd>{formatInt(rivers.file.rivers.length)}</dd>
    {/if}
    {#if meta?.giants}
      <dt title={t.worldInfo.giantsHelp}><Glossed text={t.cards.impacts} /></dt>
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
