<script lang="ts">
  import { ApiFailure } from '../lib/api/client';
  import { formatDuration } from '../lib/format';
  import { t } from '../lib/i18n/index.svelte';
  import { jobs, type TrackedJob } from '../lib/jobs.svelte';
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
  <section aria-label={t.jobs.title}>
    <h2>{t.jobs.title}</h2>
    <ul>
      {#each jobs.list as job (job.jobId)}
        {@const status = job.status}
        <li class:failed={status.status === 'failed'}>
          <div class="head">
            <strong>{job.worldId}</strong>
            {#if status.status === 'queued' || status.status === 'running'}
              <button class="small" onclick={() => cancel(job)} disabled={job.cancelling}>
                {job.cancelling ? t.jobs.cancelling : t.jobs.cancel}
              </button>
            {:else if status.status === 'done'}
              <span class="buttons">
                <button class="small" onclick={() => onOpen(status.world_id)}>{t.jobs.open}</button>
                <button class="small" onclick={() => jobs.dismiss(job.jobId)} aria-label={t.jobs.dismiss}>×</button>
              </span>
            {:else}
              <button class="small" onclick={() => jobs.dismiss(job.jobId)} aria-label={t.jobs.dismiss}>×</button>
            {/if}
          </div>
          {#if status.status === 'queued'}
            <small>{t.jobs.waiting}</small>
          {:else if status.status === 'running'}
            {@const left = jobs.remaining(job, now)}
            <progress max={status.steps} value={status.step - 1}></progress>
            <small>
              {[
                t.jobs.stageStep({ stage: t.stages[status.stage], step: status.step, steps: status.steps }),
                job.runStart !== null ? formatDuration((now - job.runStart) / 1000) : null,
                left !== null ? t.jobs.left({ time: formatDuration(left) }) : null,
              ]
                .filter(Boolean)
                .join(' · ')}
            </small>
            {#if job.cancelling}
              <small>{t.jobs.stageFirst}</small>
            {/if}
          {:else if status.status === 'done'}
            <small>{t.jobs.ready({ seed: job.worldId.slice(job.worldId.lastIndexOf('-') + 1) })}</small>
          {:else if status.status === 'cancelled'}
            <small>{t.jobs.cancelled}</small>
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
