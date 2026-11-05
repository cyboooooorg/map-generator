<script lang="ts">
  // A system's thumbnail: a small orrery drawn from its summary (see
  // lib/orrery.ts): the star in its light's colour, the habitable zone, and
  // each planet on its orbit in its preview's colour, those not generated
  // yet outlined.
  import type { SystemSummary } from '../lib/api/types/SystemSummary';
  import { formatAu } from '../lib/format';
  import { t } from '../lib/i18n/index.svelte';
  import { cssColour, orrery } from '../lib/orrery';

  interface Props {
    system: SystemSummary;
    width: number;
    height: number;
    radius?: number;
  }

  let { system, width, height, radius = 4 }: Props = $props();

  const drawn = $derived(orrery(system, width, height));
  const starColour = $derived(cssColour(system.star_light));
  /** The tooltip: how many planets, and the habitable zone when known. */
  const summary = $derived(
    t.systemList.orreryHelp({
      planets: system.bodies.filter((b) => b.kind === 'planet').length,
      zone:
        system.hz_outer_au > 0
          ? t.systemList.orreryZone({ inner: formatAu(system.hz_inner_au), outer: formatAu(system.hz_outer_au) })
          : '',
    }),
  );
  const nameOf = (id: string) => system.names[id] ?? system.bodies.find((b) => b.id === id)?.designation ?? id;
  /** An ellipse round the centre, as a path (for the zone's ring). */
  const ellipse = (rx: number, ry: number) =>
    `M ${drawn.cx - rx} ${drawn.cy} a ${rx} ${ry} 0 1 0 ${2 * rx} 0 a ${rx} ${ry} 0 1 0 ${-2 * rx} 0 Z`;
</script>

<svg class="orrery" {width} {height} viewBox={`0 0 ${width} ${height}`} style:border-radius={`${radius}px`} aria-hidden="true">
  <title>{summary}</title>
  {#if drawn.zone}
    <path class="zone" fill-rule="evenodd" d={`${ellipse(...drawn.zone.outer)} ${ellipse(...drawn.zone.inner)}`} />
  {/if}
  {#each drawn.planets as planet (planet.id)}
    <path class="orbit" d={planet.orbit} />
  {/each}
  <circle class="glow" cx={drawn.cx} cy={drawn.cy} r={drawn.starR * 1.8} fill={starColour} />
  <circle cx={drawn.cx} cy={drawn.cy} r={drawn.starR} fill={starColour} />
  {#each drawn.planets as planet (planet.id)}
    {#if planet.generated}
      <circle cx={planet.x} cy={planet.y} r={planet.r} fill={cssColour(planet.colour)}><title>{nameOf(planet.id)}</title></circle>
    {:else}
      <circle class="stub" cx={planet.x} cy={planet.y} r={planet.r * 0.85} stroke={cssColour(planet.colour)}
        ><title>{t.systemList.orreryStub({ name: nameOf(planet.id) })}</title></circle
      >
    {/if}
  {/each}
</svg>

<style>
  .orrery {
    display: block;
    flex: none;
    background: #05070d;
  }
  .zone {
    fill: #3fbf6f;
    opacity: 0.25;
  }
  .orbit {
    fill: none;
    stroke: #c8d2e6;
    stroke-width: 0.5;
    opacity: 0.35;
  }
  .glow {
    opacity: 0.25;
  }
  .stub {
    fill: none;
    stroke-width: 0.9;
  }
</style>
