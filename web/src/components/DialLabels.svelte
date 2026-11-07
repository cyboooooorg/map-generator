<script lang="ts">
  // The rotation dial's numbers (see lib/viewer/rotationDial): the ring's,
  // 0° to 330°, and the star's half-circle's, −90° to +90°, each outside
  // its circle, centred on its point, small, without leader lines. The
  // pointer's reading and the markers' names are laid out with the other
  // labels of the sky (SkyLabels). Positions come from
  // PlanetView.skyScreenPositions, null where the body hides the point.
  import { ARC_NUMBERS } from '../lib/viewer/rotationDial';

  type Point = { x: number; y: number } | null;

  interface Props {
    /** The ring's numbered ticks, 0°, 30°, … 330° in order. */
    ticks: readonly Point[];
    /** The half-circle's, in the order of ARC_NUMBERS. */
    arcTicks: readonly Point[];
  }

  let { ticks, arcTicks }: Props = $props();
  const signed = (degrees: number) => (degrees > 0 ? `+${degrees}°` : degrees < 0 ? `−${-degrees}°` : '0°');
</script>

<div class="dial-labels" aria-hidden="true">
  {#each ticks as at, k (k)}
    {#if at}
      <span class="number" style:left={`${at.x}px`} style:top={`${at.y}px`}>{k * 30}°</span>
    {/if}
  {/each}
  {#each arcTicks as at, k (k)}
    <!-- 0° on the half-circle is the ring's own bearing: its number is left out. -->
    {#if at && ARC_NUMBERS[k] !== 0}
      <span class="height" style:left={`${at.x}px`} style:top={`${at.y}px`}>{signed(ARC_NUMBERS[k]!)}</span>
    {/if}
  {/each}
</div>

<style>
  .dial-labels {
    position: absolute;
    inset: 0;
    pointer-events: none;
    overflow: hidden;
  }
  span {
    position: absolute;
    transform: translate(-50%, -50%);
    font-size: 0.68rem;
    line-height: 1.2;
    white-space: nowrap;
    font-variant-numeric: tabular-nums;
    text-shadow: 0 0 3px rgb(0 0 0 / 0.9);
  }
  .number {
    color: #9fd3ff;
  }
  .height {
    color: #c9b98a;
  }
</style>
