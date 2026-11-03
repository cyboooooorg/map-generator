<script lang="ts">
  import { type LoadProgress, progressLabel, progressShare, progressText } from '../lib/progress';

  interface Props {
    progress: LoadProgress;
  }

  let { progress }: Props = $props();

  const share = $derived(progressShare(progress));
  const text = $derived(progressText(progress));
  const label = $derived(progressLabel(progress));
</script>

<!-- A slim bar under a loading message: the file being worked on, and how far it has got. -->
<div class="loading-bar">
  <div class="caption">
    <span class="label">{label}</span>
    {#if text}<span class="amount">{text}</span>{/if}
  </div>
  <div
    class="track"
    class:indeterminate={share === null}
    role="progressbar"
    aria-label={label}
    aria-valuemin="0"
    aria-valuemax="100"
    aria-valuenow={share === null ? undefined : Math.round(share * 100)}
  >
    <div class="fill" style:width={share === null ? undefined : `${share * 100}%`}></div>
  </div>
</div>

<style>
  .loading-bar {
    width: min(280px, 70vw);
    font-size: 0.8rem;
    color: var(--muted);
  }
  .caption {
    display: flex;
    justify-content: space-between;
    gap: 8px;
    margin-bottom: 4px;
  }
  .label {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .amount {
    flex: none;
    font-variant-numeric: tabular-nums;
  }
  .track {
    position: relative;
    height: 3px;
    overflow: hidden;
    border-radius: 2px;
    background: var(--border);
  }
  .fill {
    height: 100%;
    background: var(--accent);
    transition: width 0.15s linear;
  }
  .indeterminate .fill {
    position: absolute;
    width: 30%;
    animation: slide 1.2s ease-in-out infinite;
  }
  @keyframes slide {
    from {
      left: -30%;
    }
    to {
      left: 100%;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .indeterminate .fill {
      animation: none;
      left: 0;
      width: 100%;
      opacity: 0.4;
    }
  }
</style>
