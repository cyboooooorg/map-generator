<script lang="ts">
  // Moons and rings: the moons of a planet (each a link to its card), its
  // rings and the dust rings, a ring moon's part in them, and the tides.
  import './facts.css';
  import { t } from '../../lib/i18n/index.svelte';
  import { formatInt, formatUpTo, withUnit } from '../../lib/format';
  import { dustFedBy, dustOf, ringPart, ringsOf } from '../../lib/ringMoons';
  import type { CardSource } from '../../lib/cardSource';

  let { world }: { world: CardSource } = $props();

  const system = $derived(world.system);
  const body = $derived(world.shownSystemBody);
  const meta = $derived(world.meta);
  const nameOf = (id: string, designation: string) => world.systemNames[id] ?? designation;
  const planet = $derived(body ? (system?.bodies.find((b) => b.id === (body.parent ?? body.id)) ?? null) : null);
  const family = $derived(planet ? (system?.bodies.filter((b) => b.parent === planet.id) ?? []) : []);
  const rings = $derived(body ? ringsOf(body) : null);
  const part = $derived(body ? ringPart(body, planet) : null);
  const dust = $derived(body ? dustOf(body) : []);
  const fed = $derived(body ? dustFedBy(body, planet) : null);
  const tides = $derived(meta?.moons && !meta.giant ? meta.moons : null);
</script>

{#if body}
  <dl class="facts">
    {#if body.kind === 'planet'}
      <dt title={t.worldInfo.moonsHelp}>{t.card.moons}</dt>
      <dd class="moons">
        {#each family as moon (moon.id)}
          {#if world.preview}
            <span>{nameOf(moon.id, moon.designation)}</span>
          {:else}
            <button class="link" onclick={() => world.showSystemBody(moon.id)}>{nameOf(moon.id, moon.designation)}</button>
          {/if}
        {:else}
          {t.worldInfo.noMoon}
        {/each}
        {#if meta?.moons?.binary}<span>· {t.worldInfo.binary}</span>{/if}
      </dd>
    {/if}
    {#if tides}
      <dt title={t.worldInfo.tidesHelp}>{t.worldInfo.tides}</dt>
      <dd>
        {t.worldInfo.tidesValue({ tide: formatUpTo(tides.tide, tides.tide < 1 ? 2 : 1) })} ·
        <span title={t.worldInfo.tiltHelp}>{tides.tilt_steadied ? t.worldInfo.tiltSteady : t.worldInfo.tiltFree}</span>
      </dd>
    {/if}
    {#if rings}
      <dt>{t.card.rings}</dt>
      <dd>
        {t.card.ringsValue({
          kind: rings.bright ? 'bright' : 'faint',
          inner: formatUpTo(rings.inner, 2),
          outer: formatUpTo(rings.outer, 2),
          gaps: rings.gaps.length,
        })}
      </dd>
    {/if}
    {#if rings?.d_ring}
      <dt>{t.card.dRingTitle}</dt>
      <dd>{t.card.dRing({ inner: formatUpTo(rings.d_ring.inner, 2), outer: formatUpTo(rings.d_ring.outer, 2) })}</dd>
    {/if}
    {#if body.params.ridge}
      <dt>{t.card.ridge}</dt>
      <dd>{t.card.ridgeValue({ height: withUnit(formatInt(Math.round(body.params.ridge * 100)), '%') })}</dd>
    {/if}
    {#if dust.length > 0}
      <dt>{t.card.dustRings}</dt>
      <dd>
        {#each dust as ring, k (k)}
          {@const moon = family.find((m) => m.id === ring.source)}
          <span class="dust"
            >{t.card.dustRing({
              kind: ring.kind,
              moon: moon ? nameOf(moon.id, moon.designation) : ring.source,
              inner: formatUpTo(ring.inner, 1),
              outer: formatUpTo(ring.outer, 1),
            })}</span
          >
        {/each}
      </dd>
    {/if}
    {#if fed}
      <dt>{t.card.dustSource}</dt>
      <dd>{t.card.dustFed({ kind: fed.kind })}</dd>
    {/if}
    {#if part}
      <dt>{t.card.ringRole}</dt>
      <dd>{t.card.ringRoles[part.role]({ gap: withUnit(formatInt(part.gapKm ?? 0), 'km') })}</dd>
    {/if}
  </dl>
{/if}

<style>
  .dust {
    display: block;
  }
  .moons {
    display: flex;
    flex-wrap: wrap;
    gap: 2px 8px;
  }
</style>
