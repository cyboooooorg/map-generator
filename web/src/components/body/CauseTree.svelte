<script lang="ts">
  // The disclosure of a derived value: a mark that unfolds the values it
  // comes from, down to the ones the creation form sets (in the accent colour).
  import { t } from '../../lib/i18n/index.svelte';
  import type { Cause } from '../../lib/api/types/Cause';
  import Branch from './CauseBranch.svelte';

  interface Props {
    /** The value's tree; nothing is shown without one. */
    tree: Cause | null;
    open: boolean;
    ontoggle: () => void;
  }

  let { tree, open, ontoggle }: Props = $props();
</script>

{#if tree && tree.causes.length > 0}
  <button class="why" aria-expanded={open} title={open ? t.causes.whyClose : t.causes.whyHelp} onclick={ontoggle}
    ><span aria-hidden="true">{open ? '▾' : '▸'}</span> {t.causes.why}</button
  >
  {#if open}
    <ul class="causes">
      {#each tree.causes as cause}
        <Branch {cause} parent={tree.quantity} />
      {/each}
    </ul>
  {/if}
{/if}

<style>
  .why {
    border: none;
    background: none;
    padding: 0 0 0 6px;
    color: var(--muted);
    font-size: 0.85em;
    cursor: pointer;
  }
  .why:hover {
    color: var(--link);
  }
  .causes {
    margin: 2px 0 4px;
    padding: 0;
    list-style: none;
    font-size: 0.92em;
  }
</style>
