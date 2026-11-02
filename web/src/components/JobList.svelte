<script lang="ts">
  import { ApiFailure } from '../lib/api/client';
  import { formatDuration } from '../lib/format';
  import { jobs, STAGE_LABELS, type TrackedJob } from '../lib/jobs.svelte';
  import { toasts } from '../lib/toasts.svelte';

  interface Props {
    onOpen: (worldId: string) => void;
  }

  let { onOpen }: Props = $props();

  let now = $state(Date.now());
  // A boolean, so that progress updates of the jobs do not restart the timer.
  const ticking = $derived(jobs.active.length > 0);
  $effect(() => {
    if (!ticking) {
      return;
    }
    const timer = setInterval(() => (now = Date.now()), 1000);
    return () => clearInterval(timer);
  });

  async function cancel(job: TrackedJob): Promise<void> {
    try {
      await jobs.cancel(job.jobId);
    } catch (e) {
      toasts.error(e instanceof ApiFailure ? e.message : String(e));
    }
  }
</script>

{#if jobs.list.length > 0}
  <section aria-label="Generations">
    <h2>Generations</h2>
    <ul>
      {#each jobs.list as job (job.jobId)}
        {@const status = job.status}
        <li class:failed={status.status === 'failed'}>
          <div class="head">
            <strong>{job.worldId}</strong>
            {#if status.status === 'queued' || status.status === 'running'}
              <button class="small" onclick={() => cancel(job)} disabled={job.cancelling}>
                {job.cancelling ? 'Cancelling…' : 'Cancel'}
              </button>
            {:else if status.status === 'done'}
              <span class="buttons">
                <button class="small" onclick={() => onOpen(status.world_id)}>Open</button>
                <button class="small" onclick={() => jobs.dismiss(job.jobId)} aria-label="Dismiss">×</button>
              </span>
            {:else}
              <button class="small" onclick={() => jobs.dismiss(job.jobId)} aria-label="Dismiss">×</button>
            {/if}
          </div>
          {#if status.status === 'queued'}
            <small>Waiting for the generator…</small>
          {:else if status.status === 'running'}
            {@const left = jobs.remaining(job, now)}
            <progress max={status.steps} value={status.step - 1}></progress>
            <small>
              {[
                `${STAGE_LABELS[status.stage]} (${status.step}/${status.steps})`,
                job.runStart !== null ? formatDuration((now - job.runStart) / 1000) : null,
                left !== null ? `about ${formatDuration(left)} left` : null,
              ]
                .filter(Boolean)
                .join(' · ')}
            </small>
            {#if job.cancelling}
              <small>The current stage finishes first.</small>
            {/if}
          {:else if status.status === 'done'}
            <small>Ready · seed {job.worldId.slice(job.worldId.lastIndexOf('-') + 1)}</small>
          {:else if status.status === 'cancelled'}
            <small>Cancelled</small>
          {:else}
            <small class="error">{status.error}</small>
          {/if}
        </li>
      {/each}
    </ul>
  </section>
{/if}

<style>
  h2 {
    margin: 0 0 6px;
    font-size: 0.95rem;
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 6px;
  }
  li {
    display: grid;
    gap: 4px;
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 8px 10px;
  }
  li.failed {
    border-color: var(--danger);
  }
  .head {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }
  .head strong {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .buttons {
    display: flex;
    gap: 4px;
  }
  .small {
    padding: 2px 8px;
    font-size: 0.85rem;
  }
  small {
    color: var(--muted);
  }
  .error {
    color: var(--danger);
  }
  progress {
    width: 100%;
  }
</style>
