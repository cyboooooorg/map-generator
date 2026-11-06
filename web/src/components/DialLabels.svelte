<script lang="ts">
  // The rotation dial's labels (see lib/viewer/rotationDial): its numbers
  // on the ring, the pointer's reading in a box, and the names of the sun
  // and of the host by their markers. Small text centred on each point, no
  // leader lines; positions come from PlanetView.skyScreenPositions, null
  // where the globe hides the point.

  type Point = { x: number; y: number } | null;

  interface Props {
    /** The ring's numbered ticks, 0°, 30°, … 330° in order. */
    ticks: readonly Point[];
    pointer: Point;
    sun: Point;
    host: Point;
    /** The pointer's reading, written out (`137°`, `+0.3°`). */
    reading: string;
    sunName: string;
    /** The host's name, null when the body shown has none (a planet). */
    hostName: string | null;
  }

  let { ticks, pointer, sun, host, reading, sunName, hostName }: Props = $props();

  /** A marker's label closer than this to the pointer's (CSS px) would draw over its box. */
  const CLEAR = 40;
  /** How far a moved label stands above or below the pointer's box, CSS px. */
  const STEP = 18;
  /**
   * Where the host's and the sun's labels go: by their markers, or, when
   * that falls on the pointer's box (on a locked body the pointer and the
   * host both sit near 0°), just above the box, the second one below it;
   * at the view's top edge, below the box, one under the other.
   */
  const placed = $derived.by(() => {
    let moved = 0;
    const place = (at: Point): Point => {
      if (!at || !pointer || Math.hypot(at.x - pointer.x, at.y - pointer.y) >= CLEAR) return at;
      const slots = pointer.y < STEP + 8 ? [1, 2] : [-1, 1];
      return { x: pointer.x, y: pointer.y + (slots[moved++] ?? 1) * STEP };
    };
    return { host: place(hostName ? host : null), sun: place(sun) };
  });
</script>

<div class="dial-labels" aria-hidden="true">
  {#each ticks as at, k (k)}
    {#if at}
      <span class="number" style:left={`${at.x}px`} style:top={`${at.y}px`}>{k * 30}°</span>
    {/if}
  {/each}
  {#if placed.sun}
    <span class="sun" style:left={`${placed.sun.x}px`} style:top={`${placed.sun.y}px`}>{sunName}</span>
  {/if}
  {#if placed.host && hostName}
    <span class="host" style:left={`${placed.host.x}px`} style:top={`${placed.host.y}px`}>{hostName}</span>
  {/if}
  {#if pointer}
    <span class="pointer" style:left={`${pointer.x}px`} style:top={`${pointer.y}px`}>{reading}</span>
  {/if}
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
  .sun {
    color: #ffe28a;
  }
  .host {
    color: #c89a6a;
  }
  .pointer {
    padding: 1px 5px;
    border: 1px solid #ffcc66;
    border-radius: 3px;
    background: rgb(5 7 13 / 0.85);
    color: #ffcc66;
    text-shadow: none;
  }
</style>
