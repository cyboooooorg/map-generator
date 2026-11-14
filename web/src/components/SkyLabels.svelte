<script lang="ts">
  import { layoutLabels } from '../lib/labelLayout';
  // Labels for the bodies of the sky (the moons, the planet seen from a
  // moon, a giant's other moons, the host star): a name and a line of
  // facts each, tied to the body by a leader line, so that bodies too small
  // to see can be found. Positions come from PlanetView.skyScreenPositions.

  export interface SkyLabel {
    name: string;
    detail: string;
  }

  interface Props {
    /** Screen positions, CSS px, in the order of `labels` (null: hidden). */
    positions: readonly ({ x: number; y: number } | null)[];
    labels: readonly (SkyLabel | null)[];
    star: { x: number; y: number } | null;
    starLabel: SkyLabel | null;
  }

  let { positions, labels, star, starLabel }: Props = $props();
  let width = $state(0);
  let height = $state(0);
  /** Each label's box, as last measured (keyed by its text): laid out with the others. */
  let sizes = $state<Record<string, { width: number; height: number }>>({});
  let elements = $state<HTMLDivElement[]>([]);
  const keyOf = (label: SkyLabel) => `${label.name}\n${label.detail}`;
  /** Before it is measured: about what a label of this text takes. */
  const estimate = (label: SkyLabel) => ({ width: Math.max(label.name.length * 7, label.detail.length * 6) + 12, height: 29 });
  const placed = $derived.by(() => {
    const items: { at: { x: number; y: number }; label: SkyLabel; star: boolean }[] = [];
    positions.forEach((at, k) => {
      const label = labels[k];
      if (at && label) items.push({ at, label, star: false });
    });
    if (star && starLabel) items.push({ at: star, label: starLabel, star: true });
    // Laid out together, so that none covers another (lib/labelLayout.ts).
    const places = layoutLabels(
      items.map((item) => ({ at: item.at, ...(sizes[keyOf(item.label)] ?? estimate(item.label)) })),
      width,
      height,
    );
    return items.map((item, k) => ({ ...item, ...places[k]! }));
  });
  // Measures the labels drawn, for the next layout (only when a size changes).
  $effect(() => {
    const next: Record<string, { width: number; height: number }> = {};
    let changed = false;
    placed.forEach((item, k) => {
      const element = elements[k];
      if (!element) return;
      const key = keyOf(item.label);
      const size = { width: element.offsetWidth, height: element.offsetHeight };
      next[key] = size;
      const old = sizes[key];
      if (!old || Math.abs(old.width - size.width) > 1 || Math.abs(old.height - size.height) > 1) changed = true;
    });
    // Only the labels shown: their texts change as the bodies move.
    if (changed) sizes = next;
  });
</script>

<div class="sky-labels" aria-hidden="true" bind:clientWidth={width} bind:clientHeight={height}>
  <svg {width} {height}>
    {#each placed as item, i (i)}
      <line x1={item.at.x} y1={item.at.y} x2={item.to.x} y2={item.to.y} class:star={item.star} />
      <circle cx={item.at.x} cy={item.at.y} r="3" class:star={item.star} />
    {/each}
  </svg>
  {#each placed as item, i (i)}
    <div bind:this={elements[i]} class="label" class:left={item.left} class:star={item.star} style:left={`${item.to.x}px`} style:top={`${item.to.y}px`}>
      <strong>{item.label.name}</strong>
      {#if item.label.detail}<span>{item.label.detail}</span>{/if}
    </div>
  {/each}
</div>

<style>
  .sky-labels {
    position: absolute;
    inset: 0;
    pointer-events: none;
    overflow: hidden;
  }
  svg {
    position: absolute;
    inset: 0;
  }
  line {
    stroke: var(--overlay-orbit, #c8d2e6);
    stroke-opacity: 0.75;
    stroke-width: 1;
  }
  circle {
    fill: none;
    stroke: var(--overlay-orbit, #c8d2e6);
    stroke-opacity: 0.9;
    stroke-width: 1;
  }
  line.star,
  circle.star {
    stroke: var(--overlay-line, #ffe3a8);
    stroke-opacity: 0.85;
  }
  .label {
    position: absolute;
    transform: translate(4px, -50%);
    display: grid;
    padding: 2px 6px;
    border-radius: 4px;
    background: var(--label-bg, rgb(5 7 13 / 0.72));
    font-size: 0.72rem;
    line-height: 1.25;
    white-space: nowrap;
    color: var(--label-text, #dfe6f2);
  }
  .label.left {
    transform: translate(calc(-100% - 4px), -50%);
  }
  .label.star {
    color: var(--overlay-line, #ffd68c);
  }
  .label span {
    color: var(--muted, #9aa4b8);
  }
</style>
