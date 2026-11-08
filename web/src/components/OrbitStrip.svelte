<script lang="ts">
  // A system as a line from its star: the planets along it by the logarithm
  // of their distance (giants larger), the habitable zone as a band, each
  // dot in its body's own colour (lib/menuCards.ts, `strip`). The "card"
  // variant is a banner with the star's glow at its left edge; the "row"
  // one is the thin strip of the list's table.
  import type { SystemSummary } from '../lib/api/types/SystemSummary';
  import { formatAu } from '../lib/format';
  import { t } from '../lib/i18n/index.svelte';
  import { strip } from '../lib/menuCards';
  import { cssColour } from '../lib/orrery';

  interface Props {
    system: SystemSummary;
    variant?: 'card' | 'row';
  }

  let { system, variant = 'card' }: Props = $props();

  const drawn = $derived(strip(system));
  const starColour = $derived(cssColour(system.star_light));
  const nameOf = (id: string) => system.names[id] ?? system.bodies.find((b) => b.id === id)?.designation ?? id;
  const zoneHelp = $derived(
    system.hz_outer_au > 0
      ? t.systemList.orreryZone({ inner: formatAu(system.hz_inner_au), outer: formatAu(system.hz_outer_au) }).replace(/^ · /, '')
      : '',
  );
</script>

<span class="strip {variant}" style:--star={starColour} role="img" aria-label={t.systemList.stripLabel}>
  {#if variant === 'card'}
    <span class="star"></span>
  {/if}
  {#if drawn.zone}
    <span class="zone" style:left={`${drawn.zone.from}%`} style:width={`${drawn.zone.width}%`} title={zoneHelp}></span>
  {/if}
  <span class="axis"></span>
  {#each drawn.planets as planet (planet.id)}
    <span
      class="planet"
      class:giant={planet.giant}
      class:stub={!planet.generated}
      style:left={`${planet.at}%`}
      style:--colour={cssColour(planet.colour)}
      title={planet.generated ? nameOf(planet.id) : t.systemList.orreryStub({ name: nameOf(planet.id) })}
    ></span>
  {/each}
</span>

<style>
  .strip {
    position: relative;
    display: block;
    width: 100%;
  }
  .card {
    height: 78px;
    background:
      radial-gradient(circle at 0% 50%, color-mix(in srgb, var(--star) 27%, transparent) 0, transparent 38%), #05070d;
    border-bottom: 1px solid var(--border);
  }
  .row {
    height: 30px;
    min-width: 200px;
  }
  .star {
    position: absolute;
    left: 6px;
    top: 27px;
    width: 24px;
    height: 24px;
    border-radius: 50%;
    background: var(--star);
    box-shadow: 0 0 14px var(--star);
  }
  .axis {
    position: absolute;
    left: 0;
    right: 0;
    height: 1px;
    background: var(--border);
  }
  .card .axis {
    left: 22px;
    right: 14px;
    top: 39px;
  }
  .row .axis {
    top: 15px;
  }
  /* The habitable zone keeps its green in every theme. */
  .zone {
    position: absolute;
    background: rgb(63 191 111 / 0.18);
    border-left: 1px solid rgb(63 191 111 / 0.5);
    border-right: 1px solid rgb(63 191 111 / 0.5);
  }
  .card .zone {
    top: 18px;
    bottom: 18px;
  }
  .row .zone {
    top: 4px;
    bottom: 4px;
    border-radius: 3px;
  }
  .planet {
    position: absolute;
    width: var(--size);
    height: var(--size);
    margin-left: calc(var(--size) / -2);
    margin-top: calc(var(--size) / -2);
    top: var(--y);
    border-radius: 50%;
    background: var(--colour);
    --size: 8px;
  }
  .card .planet {
    --y: 39px;
  }
  .row .planet {
    --y: 15px;
    --size: 9px;
  }
  .card .planet.giant {
    --size: 14px;
  }
  .row .planet.giant {
    --size: 14px;
  }
  /* Planets not generated yet are outlined, as in the orrery. */
  .planet.stub {
    background: transparent;
    box-shadow: inset 0 0 0 1.5px var(--colour);
  }
</style>
