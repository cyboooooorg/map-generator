<script lang="ts">
  // The star that pins a system to the top of the main menu
  // (`PUT`/`DELETE /api/systems/{id}/pin`), the same one in the menu's cards
  // and in the viewer's bar.
  import { api } from '../lib/api/client';
  import type { SystemSummary } from '../lib/api/types/SystemSummary';
  import { errorText } from '../lib/i18n/errors';
  import { t } from '../lib/i18n/index.svelte';
  import { systemName } from '../lib/systemList';
  import { toasts } from '../lib/toasts.svelte';

  interface Props {
    system: SystemSummary;
    /** The system's new summary. */
    onChanged: (system: SystemSummary) => void;
  }

  let { system, onChanged }: Props = $props();

  // A fixed label with aria-pressed; the tooltip says what a click does.
  const label = $derived(t.systemList.pinLabel({ name: systemName(system) }));
  const action = $derived(
    system.pinned ? t.systemList.unpin({ name: systemName(system) }) : t.systemList.pin({ name: systemName(system) }),
  );

  async function toggle(): Promise<void> {
    try {
      onChanged(await api.pinSystem(system.id, !system.pinned));
    } catch (e) {
      toasts.error(t.systemList.pinFailed({ error: errorText(e) }));
    }
  }
</script>

<button class="pin" class:on={system.pinned} aria-pressed={system.pinned} aria-label={label} title={action} onclick={toggle}
  >{system.pinned ? '★' : '☆'}</button
>

<style>
  .pin {
    padding: 0 7px;
    line-height: 1.5;
  }
  .pin.on {
    color: var(--accent);
  }
</style>
