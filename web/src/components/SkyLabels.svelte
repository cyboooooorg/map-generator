<script lang="ts">
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

  /** Offset of a label from its body: up and to the right, mirrored near the right and top edges. */
  const LEAD = { x: 34, y: -30 };
  const placed = $derived.by(() => {
    const items: { at: { x: number; y: number }; label: SkyLabel; star: boolean }[] = [];
    positions.forEach((at, k) => {
      const label = labels[k];
      if (at && label) items.push({ at, label, star: false });
    });
    if (star && starLabel) items.push({ at: star, label: starLabel, star: true });
    return items.map((item) => {
      const left = item.at.x > width - 200;
      const below = item.at.y < 60;
      return {
        ...item,
        to: { x: item.at.x + (left ? -LEAD.x : LEAD.x), y: item.at.y + (below ? -LEAD.y : LEAD.y) },
        left,
      };
    });
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
    <div class="label" class:left={item.left} class:star={item.star} style:left={`${item.to.x}px`} style:top={`${item.to.y}px`}>
      <strong>{item.label.name}</strong>
      <span>{item.label.detail}</span>
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
    stroke: rgb(200 210 230 / 0.75);
    stroke-width: 1;
  }
  circle {
    fill: none;
    stroke: rgb(200 210 230 / 0.9);
    stroke-width: 1;
  }
  line.star,
  circle.star {
    stroke: rgb(255 214 140 / 0.85);
  }
  .label {
    position: absolute;
    transform: translate(4px, -50%);
    display: grid;
    padding: 2px 6px;
    border-radius: 4px;
    background: rgb(5 7 13 / 0.72);
    font-size: 0.72rem;
    line-height: 1.25;
    white-space: nowrap;
    color: #dfe6f2;
  }
  .label.left {
    transform: translate(calc(-100% - 4px), -50%);
  }
  .label.star {
    color: #ffd68c;
  }
  .label span {
    color: var(--muted, #9aa4b8);
  }
</style>
