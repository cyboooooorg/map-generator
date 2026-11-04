<script lang="ts">
  // The star view of a world: its star as a globe, its system around it,
  // and the numbers of both (see StarView.ts).
  import { onMount } from 'svelte';
  import type { StarInfo } from '../lib/api/types/StarInfo';
  import { formatAu, formatEccentricity, formatInt, formatNumber, formatOrbit, formatUpTo, formatYear, withUnit } from '../lib/format';
  import { unitTooltip } from '../lib/units';
  import { meanOrbitalSpeed, orbitalSpeed } from '../lib/viewer/starSystem';
  import { t } from '../lib/i18n/index.svelte';
  import { type SystemLabels, StarView } from '../lib/viewer/StarView';

  interface Props {
    star: StarInfo;
    /** Orientation of the orbit's periapsis, radians (see periapsisAngle). */
    omega: number;
    /** Whether the star turns and boils (ten frames a second). */
    animated: boolean;
    /** Whether the live distances are shown. */
    distances: boolean;
    /** Whether the orbit line and the habitable zone are drawn. */
    orbit: boolean;
    zone: boolean;
    /** Whether the prominences (and eruptions) are drawn. */
    prominences: boolean;
  }

  let { star, omega, animated = $bindable(), distances = $bindable(), orbit = $bindable(), zone = $bindable(), prominences = $bindable() }: Props = $props();
  let labels = $state<SystemLabels | null>(null);
  /** Width of the view, to keep the labels near its right edge on screen. */
  let width = $state(0);

  /** Light the planet gets at `au`, relative to Earth's (the model Sun at 1 AU). */
  const lightAt = (au: number) => star.luminosity / (au * au) / 1.0233;
  const inZone = (au: number) => au >= star.hz_inner_au && au <= star.hz_outer_au;

  let canvas: HTMLCanvasElement;
  let view = $state<StarView | null>(null);

  onMount(() => {
    const created = new StarView(canvas);
    created.onLabels = (next) => (labels = next);
    view = created;
    return () => {
      created.dispose();
      view = null;
    };
  });

  $effect(() => view?.setStar(star, omega));
  $effect(() => view?.setAnimated(animated));
  $effect(() => view?.setDistances(distances));
  $effect(() => view?.setOrbitVisible(orbit));
  $effect(() => view?.setZoneVisible(zone));
  $effect(() => view?.setProminences(prominences));

  const kms = (v: number) => withUnit(formatUpTo(v, 1), 'km/s');
  /** Mean speed, and its range on an eccentric orbit. */
  const speedText = $derived.by(() => {
    const mean = kms(meanOrbitalSpeed(star.mass, star.orbit_au));
    if (star.eccentricity <= 0.001) {
      return t.starView.speedCircular({ mean, earth: kms(29.78) });
    }
    return t.starView.speedRange({
      mean,
      fastest: kms(orbitalSpeed(star.mass, star.orbit_au, star.periapsis_au)),
      slowest: kms(orbitalSpeed(star.mass, star.orbit_au, star.apoapsis_au)),
    });
  });

  /** Back to the starting view of the system. */
  export function resetView(): void {
    view?.resetView();
  }
</script>

<canvas bind:this={canvas} class="star-canvas"></canvas>
{#if distances && labels}
  <div class="labels" aria-hidden="true" bind:clientWidth={width}>
    {#if labels.planet}
      <span class="label planet" class:left={labels.planet.x > width * 0.6} style:left={`${labels.planet.x}px`} style:top={`${labels.planet.y}px`}
        >{t.starView.now({
          distance: formatAu(labels.planet.au),
          speed: kms(orbitalSpeed(star.mass, star.orbit_au, labels.planet.au)),
          light: `${formatUpTo(lightAt(labels.planet.au), 2)}×`,
          zone: inZone(labels.planet.au) ? t.starView.inZone : t.starView.outOfZone,
        })}</span
      >
    {/if}
    {#if labels.periapsis && star.eccentricity > 0.001}
      <span class="label apsis" class:left={labels.periapsis.x > width * 0.6} style:left={`${labels.periapsis.x}px`} style:top={`${labels.periapsis.y}px`}
        >{t.starView.periapsis({ distance: formatAu(star.periapsis_au) })}</span
      >
    {/if}
    {#if labels.apoapsis && star.eccentricity > 0.001}
      <span class="label apsis" class:left={labels.apoapsis.x > width * 0.6} style:left={`${labels.apoapsis.x}px`} style:top={`${labels.apoapsis.y}px`}
        >{t.starView.apoapsis({ distance: formatAu(star.apoapsis_au) })}</span
      >
    {/if}
    {#if labels.inner}
      <span class="label zone" class:left={labels.inner.x > width * 0.6} style:left={`${labels.inner.x}px`} style:top={`${labels.inner.y}px`}
        >{formatAu(star.hz_inner_au)}</span
      >
    {/if}
    {#if labels.outer}
      <span class="label zone" class:left={labels.outer.x > width * 0.6} style:left={`${labels.outer.x}px`} style:top={`${labels.outer.y}px`}
        >{formatAu(star.hz_outer_au)}</span
      >
    {/if}
  </div>
{/if}
<details class="star-panel" role="region" aria-label={t.starView.title} open>
  <summary><strong>{t.starView.heading({ type: star.type, kind: t.starView.classNames[star.type[0] ?? 'G'] ?? '' })}</strong></summary>
  <h4>{t.starView.starSection}</h4>
  <dl>
    <dt title={t.starView.temperatureHelp}>{t.starView.temperature}</dt>
    <dd>{withUnit(formatInt(Math.round(star.teff_k / 10) * 10), 'K')}</dd>
    <dt title={t.starView.luminosityHelp}>{t.starView.luminosity}</dt>
    <dd class="unit-help" title={unitTooltip('L☉')}>{withUnit(formatUpTo(star.luminosity, star.luminosity < 0.01 ? 4 : 2), 'L☉')}</dd>
    <dt title={t.starView.sizeHelp}>{t.starView.size}</dt>
    <dd>
      {t.starView.sizeValue({
        radius: withUnit(formatUpTo(star.radius, 2), 'R☉'),
        mass: withUnit(formatUpTo(star.mass, 2), 'M☉'),
      })}
    </dd>
    <dt title={t.starView.lifetimeHelp}>{t.starView.lifetime}</dt>
    <dd>{withUnit(formatUpTo(star.lifetime_gyr, star.lifetime_gyr < 10 ? 1 : 0), t.starView.gyr)}</dd>
    <dt title={t.starView.zoneHelp}><span class="swatch zone"></span>{t.starView.zone}</dt>
    <dd>
      {t.starView.range({
        inner: formatAu(star.hz_inner_au),
        outer: formatAu(star.hz_outer_au),
      })}
    </dd>
  </dl>
  <h4>{t.starView.planetSection}</h4>
  <dl>
    <dt title={t.starView.orbitHelp}><span class="swatch orbit"></span>{t.worldInfo.orbit}</dt>
    <dd class="unit-help" title={unitTooltip('AU')}>{formatOrbit(star)}</dd>
    <dt title={t.starView.eccentricityHelp}>{t.worldInfo.eccentricity}</dt>
    <dd>{formatEccentricity(star)}</dd>
    <dt title={t.starView.speedHelp}>{t.starView.speed}</dt>
    <dd>{speedText}</dd>
    <dt>{t.worldInfo.year}</dt>
    <dd>{formatYear(star.year_days)}</dd>
    <dt title={t.starView.equilibriumHelp}>{t.starView.equilibrium}</dt>
    <dd>{withUnit(formatNumber(star.equilibrium_k - 273.15), '°C')}</dd>
  </dl>
  <h4>{t.starView.displaySection}</h4>
  <div class="checks">
    <label class="check">
      <input type="checkbox" bind:checked={orbit} />
      {t.starView.showOrbit}
    </label>
    <label class="check">
      <input type="checkbox" bind:checked={zone} />
      {t.starView.showZone}
    </label>
    <label class="check" title={t.starView.prominencesHelp}>
      <input type="checkbox" bind:checked={prominences} />
      {t.starView.prominences}
    </label>
    <label class="check" title={t.starView.distancesHelp}>
      <input type="checkbox" bind:checked={distances} />
      {t.starView.distances}
    </label>
    <label class="check" title={t.starView.animateHelp}>
      <input type="checkbox" bind:checked={animated} />
      {t.starView.animate}
    </label>
  </div>
  {#if distances && star.eccentricity <= 0.001}
    <!-- A circle has no closest or farthest point: say why none is marked. -->
    <small class="muted">{t.starView.circular({ distance: formatAu(star.orbit_au) })}</small>
  {/if}
  <small class="muted">{t.starView.schematic}</small>
</details>

<style>
  .star-canvas {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    display: block;
    cursor: grab;
  }
  .star-canvas:active {
    cursor: grabbing;
  }
  .labels {
    position: absolute;
    inset: 0;
    pointer-events: none;
    overflow: hidden;
  }
  .label {
    position: absolute;
    transform: translate(8px, -50%);
    padding: 1px 5px;
    border-radius: 4px;
    background: rgb(5 7 13 / 0.7);
    font-size: 0.75rem;
    white-space: nowrap;
  }
  .label.left {
    transform: translate(calc(-100% - 8px), -50%);
  }
  .label.planet {
    color: #ffd27a;
    white-space: normal;
    width: max-content;
    max-width: 170px;
  }
  .label.apsis {
    color: #ffe3a8;
  }
  .label.zone {
    color: #8fe0aa;
  }
  h4 {
    margin: 4px 0 0;
    font-size: 0.75rem;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--muted);
  }
  dt[title] {
    cursor: help;
  }
  .checks {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 2px 10px;
  }
  .star-panel summary {
    cursor: pointer;
  }
  .star-panel[open] summary {
    margin-bottom: 4px;
  }
  .star-panel {
    position: absolute;
    left: 12px;
    bottom: 12px;
    max-width: min(380px, calc(100% - 24px));
    /* Leave room for the toolbar above; scroll rather than cover it. */
    max-height: calc(100% - 72px);
    overflow-y: auto;
    display: grid;
    gap: 6px;
    padding: 10px 12px;
    background: var(--panel);
    border: 1px solid var(--border);
    border-radius: 8px;
    font-size: 0.85rem;
  }
  dl {
    display: grid;
    /* A fixed first column lines the sections up with each other. */
    grid-template-columns: 9.5em 1fr;
    gap: 2px 10px;
    margin: 0;
  }
  dt {
    color: var(--muted);
  }
  dd {
    margin: 0;
  }
  .swatch {
    display: inline-block;
    width: 10px;
    height: 10px;
    margin-right: 6px;
    border-radius: 2px;
    vertical-align: -1px;
  }
  .swatch.zone {
    background: #3fbf6f;
    opacity: 0.6;
  }
  .swatch.orbit {
    background: #c8d2e6;
  }
  label.check {
    display: flex;
    gap: 6px;
  }
  .muted {
    color: var(--muted);
  }
</style>
