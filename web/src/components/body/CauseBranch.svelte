<script lang="ts">
  // One value of a cause tree and, indented under it, its own causes.
  import { t } from '../../lib/i18n/index.svelte';
  import { formatCause } from '../../lib/causes';
  import type { Cause } from '../../lib/api/types/Cause';
  import type { InputField } from '../../lib/api/types/InputField';
  import type { Quantity } from '../../lib/api/types/Quantity';
  import Branch from './CauseBranch.svelte';

  interface Props {
    cause: Cause;
    /** The quantity above, whose unit an escape takes. */
    parent: Quantity;
    /** Makes an input leaf a button that goes to its field in the form. */
    oninput?: ((field: InputField) => void) | undefined;
  }

  let { cause, parent, oninput }: Props = $props();

  const name = $derived(t.causes.quantities[cause.quantity]);
  const text = $derived(formatCause(cause, parent));
  const field = $derived(cause.input ? t.causes.inputs[cause.input] : null);
</script>

<li>
  {#if oninput && cause.input !== null}
    {@const input = cause.input}
    <button class="input" title={field ? t.causes.inputTitle({ field }) : undefined} onclick={() => oninput(input)}
      >{name ? t.causes.line({ name, value: text }) : text}</button
    >
  {:else}
    <span class:input={cause.input !== null} title={field ? t.causes.inputTitle({ field }) : undefined}
      >{name ? t.causes.line({ name, value: text }) : text}</span
    >
  {/if}
  {#if cause.causes.length > 0}
    <ul>
      {#each cause.causes as child}
        <Branch cause={child} parent={cause.quantity} {oninput} />
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
  button.input {
    border: none;
    background: none;
    padding: 0;
    font: inherit;
    text-align: left;
    cursor: pointer;
  }
  button.input:hover {
    text-decoration: underline;
  }
</style>
