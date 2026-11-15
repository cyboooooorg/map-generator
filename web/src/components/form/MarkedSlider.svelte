<script lang="ts" module>
  /** A reference mark: a tick on the track and a chip under it that sets the value. */
  export interface SliderMark {
    /** Where the value sits on the slider, 0 to 1 of the thumb's travel. */
    fraction: number;
    /** The chip's text: the body and its value. */
    label: string;
    /** A second, smaller line (a mass in tonnes). */
    note?: string;
    /** Earth's mark is drawn in the accent colour. */
    earth?: boolean;
    onpick: () => void;
  }
</script>

<script lang="ts">
  // A range input with its reference marks under the track: each tick at the
  // place of its value (the thumb's centre, which stays half a thumb inside
  // the track), each chip centred under its tick and stacked on a lower row
  // when it would overlap the one before. Every slider of the form with marks
  // is this one, so their ticks and thumbs agree whatever the browser.
  import { chipWidth, placeChips, ROW_GAP_PX, THUMB_PX, tickLeft } from '../../lib/sliderMarks';

  interface Props {
    min: number;
    max: number;
    step: number;
    /** The slider's own position, between `min` and `max`. */
    value: number;
    label: string;
    oninput: (value: number) => void;
    marks?: SliderMark[];
    /** Names the group of chips, and the title of each. */
    marksLabel?: string;
  }

  let { min, max, step, value, label, oninput, marks = [], marksLabel = '' }: Props = $props();

  let track = $state(0);
  /** The chips' own sizes once drawn (px): centring and overlap use what the browser laid out, the estimate only until then. */
  let measured = $state<number[]>([]);
  let heights = $state<number[]>([]);
  const widths = $derived(marks.map((m, i) => measured[i] || Math.max(chipWidth(m.label), m.note ? chipWidth(m.note) : 0)));
  const placed = $derived(placeChips(marks.map((m, i) => ({ fraction: m.fraction, width: widths[i]! })), track));
  const rows = $derived(placed.reduce((most, p) => Math.max(most, p.row + 1), 0));
  /** Height of a chip row, px: the tallest chip and a little air (estimated until drawn). */
  const tallest = $derived(Math.max(0, ...heights.slice(0, marks.length)));
  const rowHeight = $derived(tallest > 0 ? tallest + ROW_GAP_PX : marks.some((m) => m.note) ? 34 : 20);
</script>

<div class="rail" style:--thumb={`${THUMB_PX}px`} bind:clientWidth={track}>
  <input type="range" {min} {max} {step} {value} aria-label={label} oninput={(e) => oninput(Number(e.currentTarget.value))} />
  {#if marks.length > 0}
    <div class="ticks" aria-hidden="true">
      {#each marks as mark (mark.label)}
        <span class="tick" class:earth={mark.earth} style:left={tickLeft(mark.fraction)}></span>
      {/each}
    </div>
    <div class="chips" role="group" aria-label={marksLabel} style:height={`${rows * rowHeight}px`}>
      {#each marks as mark, i (mark.label)}
        {@const at = placed[i]!}
        {#if track > 0 && at.led}
          <span class="leader" class:earth={mark.earth} style:left={`${at.centre}px`} style:height={`${at.row * rowHeight + 2}px`}></span>
        {/if}
        <button
          type="button"
          class="chip"
          class:earth={mark.earth}
          bind:clientWidth={measured[i]}
          bind:clientHeight={heights[i]}
          title={marksLabel}
          style:top={`${at.row * rowHeight}px`}
          style:left={track > 0 ? `${at.left}px` : tickLeft(mark.fraction)}
          style:transform={track > 0 ? undefined : 'translateX(-50%)'}
          onclick={mark.onpick}
          >{mark.label}{#if mark.note}<small>{mark.note}</small>{/if}</button
        >
      {/each}
    </div>
  {/if}
</div>

<style>
  .rail {
    position: relative;
    min-width: 0;
    width: 100%;
  }
  /* The thumb has a size of its own, so that the ticks can sit where it stops. */
  input {
    display: block;
    width: 100%;
    height: 28px;
    margin: 0;
    padding: 0;
    background: transparent;
    appearance: none;
    -webkit-appearance: none;
  }
  input::-webkit-slider-runnable-track {
    height: 4px;
    border-radius: 2px;
    background: var(--border);
  }
  input::-moz-range-track {
    height: 4px;
    border-radius: 2px;
    background: var(--border);
  }
  input::-webkit-slider-thumb {
    -webkit-appearance: none;
    box-sizing: border-box;
    width: var(--thumb);
    height: var(--thumb);
    margin-top: calc((4px - var(--thumb)) / 2);
    border: 2px solid var(--panel-solid, #fff);
    border-radius: 50%;
    background: var(--accent);
  }
  input::-moz-range-thumb {
    box-sizing: border-box;
    width: var(--thumb);
    height: var(--thumb);
    border: 2px solid var(--panel-solid, #fff);
    border-radius: 50%;
    background: var(--accent);
  }
  input:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }
  .ticks {
    position: relative;
    height: 6px;
  }
  .tick {
    position: absolute;
    top: 0;
    width: 2px;
    height: 6px;
    margin-left: -1px;
    background: var(--muted);
  }
  .tick.earth {
    background: var(--link);
  }
  .chips {
    position: relative;
    margin-top: 2px;
  }
  .chip {
    position: absolute;
    display: grid;
    justify-items: center;
    padding: 0 8px;
    font-size: 0.8rem;
    line-height: 1.3;
    white-space: nowrap;
    border-radius: 10px;
    color: var(--muted);
    /* Hides a leader line that runs behind it. */
    background: var(--panel-solid, transparent);
    z-index: 1;
  }
  /* Joins a chip held at an end of the slider, or on a lower row, to its tick. */
  .leader {
    position: absolute;
    top: -2px;
    width: 1px;
    margin-left: -0.5px;
    background: var(--muted);
    opacity: 0.6;
  }
  .leader.earth {
    background: var(--link);
  }
  .chip small {
    font-size: 0.7rem;
  }
  .chip.earth {
    color: var(--link);
  }
  .chip:hover {
    color: var(--link);
  }
</style>
