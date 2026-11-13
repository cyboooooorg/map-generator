<script lang="ts">
  // The buttons of a system in the main menu, inside its card or row: pin,
  // rename, export and delete (or, in select mode, the checkbox in delete's place).
  import { systemExportUrl } from '../lib/api/client';
  import { t } from '../lib/i18n/index.svelte';
  import type { SystemSummary } from '../lib/api/types/SystemSummary';
  import { systemName } from '../lib/systemList';
  import PinButton from './PinButton.svelte';

  interface Props {
    system: SystemSummary;
    /** Whether the pin is among them (the list gives it a column of its own). */
    showPin?: boolean;
    selecting: boolean;
    chosen: boolean;
    onChanged: (system: SystemSummary) => void;
    /** The pin was changed (defaults to `onChanged`). */
    onPinned?: ((system: SystemSummary) => void) | undefined;
    onRename: () => void;
    onDelete: () => void;
    onToggle: () => void;
  }

  let { system, showPin = true, selecting, chosen, onChanged, onPinned, onRename, onDelete, onToggle }: Props = $props();
  const name = $derived(systemName(system));
</script>

<span class="actions">
  {#if showPin && !system.outdated}
    <PinButton {system} onChanged={onPinned ?? onChanged} />
  {/if}
  {#if !system.outdated}
  <button class="rename-button" data-rename={system.id} onclick={onRename} aria-label={t.systemList.rename({ name })} title={t.systemList.rename({ name })}
    >✎</button
  >
  <a
    class="export"
    data-export={system.id}
    href={systemExportUrl(system.id)}
    download
    aria-label={t.systemList.exportSystem({ name })}
    title={t.systemList.exportSystem({ name })}>⤓</a
  >
  {/if}
  {#if selecting}
    <input class="choose" type="checkbox" checked={chosen} onchange={onToggle} aria-label={t.worldList.selectOne({ id: name })} />
  {:else}
    <button class="delete" onclick={onDelete} aria-label={t.worldList.deleteOne({ id: name })} title={t.worldList.deleteOne({ id: name })}>×</button>
  {/if}
</span>

<style>
  .actions {
    display: inline-flex;
    align-items: center;
    gap: 2px;
  }
  .actions :global(button) {
    min-width: 32px;
    height: 32px;
    padding: 0 7px;
    background: transparent;
    border-color: transparent;
    color: var(--muted);
  }
  .actions :global(button:hover:not(:disabled)),
  .actions :global(button:focus-visible) {
    border-color: var(--accent);
    color: var(--text);
  }
  .export {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    box-sizing: border-box;
    min-width: 32px;
    height: 32px;
    padding: 0 7px;
    border: 1px solid transparent;
    border-radius: var(--radius);
    color: var(--muted);
    text-decoration: none;
  }
  .export:hover,
  .export:focus-visible {
    border-color: var(--accent);
    color: var(--text);
  }
  .actions :global(button.pin.on) {
    color: var(--accent);
  }
  .delete:hover {
    color: var(--danger);
  }
  .choose {
    width: 18px;
    height: 18px;
    margin: 7px;
  }
  @media (hover: none), (pointer: coarse) {
    .actions :global(button) {
      min-width: 40px;
      height: 40px;
    }
    .export {
      min-width: 40px;
      height: 40px;
    }
    .choose {
      width: 22px;
      height: 22px;
      margin: 9px;
    }
  }
</style>
