<script lang="ts">
  // The generations under way, everywhere in the app (the menu and the
  // viewer): a small button counting them, which opens the job list with
  // its cancel buttons.
  import { t } from '../lib/i18n/index.svelte';
  import { jobs } from '../lib/jobs.svelte';
  import JobList from './JobList.svelte';

  interface Props {
    /** Opens a finished job's system. */
    onOpen: (systemId: string) => void;
  }

  let { onOpen }: Props = $props();

  let open = $state(false);
  const active = $derived(jobs.active.length);
  // Nothing to show: the list closes.
  $effect(() => {
    if (jobs.list.length === 0) open = false;
  });

  function onKey(event: KeyboardEvent): void {
    if (event.key === 'Escape' && open) {
      open = false;
      event.stopPropagation();
    }
  }
</script>

<svelte:window onkeydown={onKey} />

{#if jobs.list.length > 0}
  <div class="jobs-indicator">
    <button
      type="button"
      class="toggle"
      class:busy={active > 0}
      aria-expanded={open}
      title={t.jobs.indicatorHelp}
      onclick={() => (open = !open)}
    >
      <span class="dot" aria-hidden="true"></span>
      {active > 0 ? t.jobs.running({ count: active }) : t.jobs.finished({ count: jobs.list.length })}
    </button>
    {#if open}
      <div class="popover">
        <JobList
          onOpen={(id) => {
            open = false;
            onOpen(id);
          }}
        />
      </div>
    {/if}
  </div>
{/if}

<style>
  .jobs-indicator {
    position: relative;
  }
  .toggle {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    white-space: nowrap;
  }
  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--muted);
  }
  .busy .dot {
    background: var(--accent);
    animation: pulse 1.2s ease-in-out infinite;
  }
  @keyframes pulse {
    50% {
      opacity: 0.3;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .busy .dot {
      animation: none;
    }
  }
  .popover {
    position: absolute;
    right: 0;
    top: calc(100% + 6px);
    z-index: 30;
    width: min(360px, calc(100vw - 24px));
    max-height: 60vh;
    overflow-y: auto;
    padding: 10px 12px;
    background: var(--frame-deco), var(--panel-solid);
    clip-path: var(--frame-clip);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    box-shadow: 0 8px 24px color-mix(in srgb, var(--bg) 70%, transparent), var(--glow);
  }
</style>
