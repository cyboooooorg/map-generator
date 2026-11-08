<script lang="ts">
  import { untrack } from 'svelte';
  // The inline form that names a system (`PUT /api/systems/{id}/name`): Enter
  // or Save keeps the name, Escape or Cancel leaves it; empty brings the
  // designation back. Shared by the main menu's cards and the viewer's bar.
  import { api } from '../lib/api/client';
  import type { SystemSummary } from '../lib/api/types/SystemSummary';
  import { errorText } from '../lib/i18n/errors';
  import { t } from '../lib/i18n/index.svelte';
  import { MAX_NAME_CHARS } from '../lib/names';
  import { systemName } from '../lib/systemList';
  import { toasts } from '../lib/toasts.svelte';

  interface Props {
    system: SystemSummary;
    /** The renamed system's summary. */
    onChanged: (system: SystemSummary) => void;
    /** The form is done, saved or not. */
    onClose: () => void;
    /** Text was typed that is not the name given. */
    dirty?: boolean;
  }

  let { system, onChanged, onClose, dirty = $bindable(false) }: Props = $props();

  // The name when the form opened: what typing is compared with.
  const original = untrack(() => system.names['star'] ?? '');
  let field = $state(original);
  let busy = $state(false);
  $effect(() => {
    dirty = field !== original;
  });

  async function save(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    busy = true;
    try {
      // Only the system's own name is sent: the server keeps the names of the bodies.
      const summary = await api.nameSystem(system.id, field.trim() === system.designation ? '' : field);
      onChanged(summary);
      toasts.show(t.systemList.renamed({ name: systemName(summary) }));
      onClose();
    } catch (e) {
      toasts.error(t.systemList.renameFailed({ error: errorText(e) }));
    } finally {
      busy = false;
    }
  }
</script>

<form class="rename" onsubmit={save}>
  <!-- svelte-ignore a11y_autofocus -->
  <input
    bind:value={field}
    placeholder={system.designation}
    maxlength={MAX_NAME_CHARS}
    aria-label={t.systemList.renameLabel}
    onkeydown={(e) => e.key === 'Escape' && onClose()}
    autofocus
  />
  <span class="actions">
    <button type="submit" disabled={busy}>{t.systemList.renameSave}</button>
    <button type="button" onclick={onClose}>{t.systemList.renameCancel}</button>
  </span>
</form>

<style>
  .rename {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
    min-width: 0;
  }
  .rename input {
    flex: 1 1 10em;
    min-width: 0;
  }
  .actions {
    display: flex;
    gap: 6px;
  }
</style>
