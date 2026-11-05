<script lang="ts" module>
  import type { IconName } from '../../lib/icons';

  /** A jump of the playback bar (to the start, to a periapsis…). */
  export interface Jump {
    key: string;
    label: string;
    help?: string;
    icon: IconName;
    run: () => void;
  }
</script>

<script lang="ts">
  // A viewer's time, as a video player shows it, at the bottom of the view:
  // play / pause, the speed, the time bar with the date, and jumps.
  import Icon from '../Icon.svelte';
  import { t } from '../../lib/i18n/index.svelte';

  interface Props {
    playing: boolean;
    /** The speed slider: its value, bounds, step, and how it reads; none hides it. */
    speed?: { value: number; min: number; max: number; step?: number; text: string; help?: string; set: (v: number) => void } | null;
    /** The time bar: where the clock is, its end, and how the date reads; none hides it. */
    time?: { value: number; max: number; text: string; help?: string; set: (v: number) => void } | null;
    /** A line on the clock (how fast time runs), as the play button's tooltip. */
    clock?: string;
    jumps?: readonly Jump[];
  }

  let { playing = $bindable(), speed = null, time = null, clock = '', jumps = [] }: Props = $props();
</script>

<div class="playback" role="group" aria-label={t.playback.label}>
  <button
    type="button"
    class="play"
    aria-pressed={playing}
    aria-label={playing ? t.playback.pause : t.playback.play}
    title={clock ? `${playing ? t.playback.pause : t.playback.play} · ${clock}` : playing ? t.playback.pause : t.playback.play}
    onclick={() => (playing = !playing)}><Icon name={playing ? 'pause' : 'play'} size={20} /></button
  >
  {#if time}
    <label class="time" title={time.help}>
      <span class="visually-hidden">{t.playback.time}</span>
      <input
        type="range"
        min="0"
        max={time.max}
        step="any"
        value={Math.min(time.value, time.max)}
        oninput={(event) => time?.set(Number((event.currentTarget as HTMLInputElement).value))}
      />
    </label>
    <span class="date" aria-live="off">{time.text}</span>
  {:else}
    <span class="date grow">{clock}</span>
  {/if}
  {#each jumps as jump (jump.key)}
    <button type="button" class="jump" title={jump.help ?? jump.label} aria-label={jump.label} onclick={jump.run}
      ><Icon name={jump.icon} size={16} /></button
    >
  {/each}
  {#if speed}
    <label class="speed" title={speed.help}>
      <span class="visually-hidden">{t.playback.speed}</span>
      <input
        type="range"
        min={speed.min}
        max={speed.max}
        step={speed.step ?? 1}
        value={speed.value}
        oninput={(event) => speed?.set(Number((event.currentTarget as HTMLInputElement).value))}
      />
      <small>{speed.text}</small>
    </label>
  {/if}
</div>

<style>
  .playback {
    pointer-events: auto;
    position: absolute;
    left: 12px;
    right: 12px;
    bottom: 12px;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px 12px;
    padding: 6px 12px;
    background: var(--frame-deco), var(--panel);
    clip-path: var(--frame-clip);
    box-shadow: var(--glow);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    font-size: 0.85rem;
  }
  .play {
    padding: 6px 10px;
  }
  .time {
    flex: 1 1 200px;
    display: flex;
  }
  .date {
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }
  .date.grow {
    flex: 1;
    white-space: normal;
    color: var(--muted);
  }
  .jump {
    padding: 5px;
  }
  .speed {
    display: flex;
    align-items: center;
    gap: 6px;
    flex: 0 1 220px;
  }
  .speed small {
    white-space: nowrap;
    font-variant-numeric: tabular-nums;
  }
  .visually-hidden {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
    white-space: nowrap;
  }
</style>
