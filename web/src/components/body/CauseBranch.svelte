<script lang="ts">
  // One value of a cause tree and, indented under it, its own causes.
  import { t } from '../../lib/i18n/index.svelte';
  import { formatCause } from '../../lib/causes';
  import type { Cause } from '../../lib/api/types/Cause';
  import type { Quantity } from '../../lib/api/types/Quantity';
  import Branch from './CauseBranch.svelte';

  /** `parent` is the quantity above, whose unit an escape takes. */
  let { cause, parent }: { cause: Cause; parent: Quantity } = $props();

  const name = $derived(t.causes.quantities[cause.quantity]);
  const text = $derived(formatCause(cause, parent));
  const field = $derived(cause.input ? t.causes.inputs[cause.input] : null);
</script>

<li>
  <span class:input={cause.input !== null} title={field ? t.causes.inputTitle({ field }) : undefined}
    >{name ? t.causes.line({ name, value: text }) : text}</span
  >
  {#if cause.causes.length > 0}
    <ul>
      {#each cause.causes as child}
        <Branch cause={child} parent={cause.quantity} />
      {/each}
    </ul>
  {/if}
</li>

<style>
  ul {
    margin: 0;
    padding: 0 0 0 14px;
    list-style: none;
    border-left: 1px solid var(--border);
  }
  li {
    padding: 1px 0;
  }
  .input {
    color: var(--link);
  }
</style>
