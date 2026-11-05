<script lang="ts">
  // The classic axes gizmo, in a corner of a view: x red, y green, z blue,
  // turning with the camera (see lib/viewer/axes.ts); faded when pointing away.
  import type { ScreenAxis } from '../lib/viewer/axes';
  import { t } from '../lib/i18n/index.svelte';

  let { axes }: { axes: readonly ScreenAxis[] } = $props();
  const COLOURS = { x: '#ff5a5a', y: '#5ad65a', z: '#5a9cff' } as const;
  const R = 22;
</script>

<svg class="gizmo" viewBox="-32 -32 64 64" width="64" height="64" role="img" aria-label={t.viewer.axesGizmo}>
  <circle r="30" class="ring" />
  {#each axes as a (a.axis)}
    <g opacity={a.front ? 1 : 0.45}>
      <line x1="0" y1="0" x2={a.dx * R} y2={a.dy * R} stroke={COLOURS[a.axis]} stroke-width="2.5" stroke-linecap="round" />
      <circle cx={a.dx * R} cy={a.dy * R} r="6" fill={COLOURS[a.axis]} />
      <text x={a.dx * R} y={a.dy * R + 3.5} text-anchor="middle">{a.axis.toUpperCase()}</text>
    </g>
  {/each}
</svg>

<style>
  .gizmo {
    pointer-events: none;
    display: block;
  }
  .ring {
    fill: var(--panel);
    stroke: var(--border);
  }
  text {
    font-size: 8px;
    font-weight: 700;
    fill: #0b0e14;
    font-family: var(--font-ui);
  }
</style>
