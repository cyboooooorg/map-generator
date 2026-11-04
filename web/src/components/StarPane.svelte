<script lang="ts">
  // The star view of a world: its star as a globe, its system around it,
  // and the numbers of both (see StarView.ts).
  import { onMount } from 'svelte';
  import type { StarInfo } from '../lib/api/types/StarInfo';
  import { formatAu, formatNumber, formatOrbit, formatStar, formatUpTo, formatYear, withUnit } from '../lib/format';
  import { t } from '../lib/i18n/index.svelte';
  import { StarView } from '../lib/viewer/StarView';

  interface Props {
    star: StarInfo;
    /** Whether the star turns and boils (ten frames a second). */
    animated: boolean;
  }

  let { star, animated = $bindable() }: Props = $props();

  let canvas: HTMLCanvasElement;
  let view = $state<StarView | null>(null);

  onMount(() => {
    const created = new StarView(canvas);
    view = created;
    return () => {
      created.dispose();
      view = null;
    };
  });

  $effect(() => view?.setStar(star));
  $effect(() => view?.setAnimated(animated));

  /** Back to the starting view of the system. */
  export function resetView(): void {
    view?.resetView();
  }
</script>

<canvas bind:this={canvas} class="star-canvas"></canvas>
<details class="star-panel" role="region" aria-label={t.starView.title} open>
  <summary><strong>{formatStar(star)}</strong></summary>
  <dl>
    <dt>{t.starView.size}</dt>
    <dd>
      {t.starView.sizeValue({
        radius: withUnit(formatUpTo(star.radius, 2), 'R☉'),
        mass: withUnit(formatUpTo(star.mass, 2), 'M☉'),
      })}
    </dd>
    <dt>{t.starView.lifetime}</dt>
    <dd>{withUnit(formatUpTo(star.lifetime_gyr, star.lifetime_gyr < 10 ? 1 : 0), t.starView.gyr)}</dd>
    <dt>{t.starView.zone}</dt>
    <dd>
      <span class="swatch zone"></span>{t.starView.range({
        inner: formatAu(star.hz_inner_au),
        outer: formatAu(star.hz_outer_au),
      })}
    </dd>
    <dt>{t.worldInfo.orbit}</dt>
    <dd><span class="swatch orbit"></span>{formatOrbit(star)}</dd>
    <dt>{t.worldInfo.year}</dt>
    <dd>{formatYear(star.year_days)}</dd>
    <dt>{t.starView.equilibrium}</dt>
    <dd>{withUnit(formatNumber(star.equilibrium_k - 273.15), '°C')}</dd>
  </dl>
  <label class="check" title={t.starView.animateHelp}>
    <input type="checkbox" bind:checked={animated} />
    {t.starView.animate}
  </label>
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
    max-width: min(360px, calc(100% - 24px));
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
    grid-template-columns: auto 1fr;
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
