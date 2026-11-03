<script lang="ts">
  import { t } from '../lib/i18n/index.svelte';
  import { toasts } from '../lib/toasts.svelte';
</script>

<div class="toasts" aria-live="polite">
  {#each toasts.list as toast (toast.id)}
    <div class="toast" class:error={toast.kind === 'error'} role={toast.kind === 'error' ? 'alert' : 'status'}>
      <span>{toast.message}</span>
      {#if toast.action}
        {@const action = toast.action}
        <button
          onclick={() => {
            action.run();
            toasts.dismiss(toast.id);
          }}>{action.label}</button
        >
      {/if}
      <button class="close" onclick={() => toasts.dismiss(toast.id)} aria-label={t.toasts.dismiss}>×</button>
    </div>
  {/each}
</div>

<style>
  .toasts {
    position: fixed;
    left: 50%;
    bottom: 16px;
    transform: translateX(-50%);
    display: grid;
    gap: 8px;
    z-index: 10;
    width: max-content;
    max-width: calc(100vw - 32px);
  }
  .toast {
    display: flex;
    align-items: center;
    gap: 10px;
    background: var(--panel-solid);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 8px 10px 8px 14px;
    box-shadow: 0 6px 20px #0008;
  }
  .toast.error {
    border-color: var(--danger);
  }
  .toast button {
    padding: 2px 10px;
  }
  .close {
    border: none;
    background: none;
    font-size: 1.1rem;
    padding: 0 4px;
  }
</style>
