<script lang="ts">
  // A body's (or the star's) given name, with the button that renames it:
  // empty goes back to its designation.
  import { t } from '../../lib/i18n/index.svelte';
  import { api } from '../../lib/api/client';
  import { errorText } from '../../lib/i18n/errors';
  import { toasts } from '../../lib/toasts.svelte';
  import { MAX_NAME_CHARS } from '../../lib/names';
  import type { SystemSummary } from '../../lib/api/types/SystemSummary';
  import type { CardSource } from '../../lib/cardSource';

  interface Props {
    world: CardSource;
    /** The body's id, or `star` for the star. */
    id: string;
    designation: string;
    /** The name shown: the given one, else the designation. */
    name: string;
    /** Called once the name is saved, with the system's summary. */
    onRenamed: (summary: SystemSummary) => void;
  }

  let { world, id, designation, name, onRenamed }: Props = $props();

  let renaming = $state(false);
  let saving = $state(false);
  let field = $state('');
  // Another body or system closes the editor.
  $effect(() => {
    void [world.systemId, world.systemBody];
    renaming = false;
  });
  function start(): void {
    field = world.systemNames[id] ?? '';
    renaming = true;
  }
  async function save(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    const systemId = world.systemId;
    if (!systemId) return;
    const value = field.trim() === designation ? '' : field.trim();
    saving = true;
    try {
      // Only this name is sent: the server keeps the others, even those given since.
      const summary = await api.nameBody(systemId, id, value);
      renaming = false;
      // The system shown may have changed while the name was saved.
      if (world.systemId === systemId) world.loadSystem(systemId);
      onRenamed(summary);
      toasts.show(t.worldInfo.renamed({ name: value === '' ? designation : value }));
    } catch (e) {
      toasts.error(t.worldInfo.renameFailed({ error: errorText(e) }));
    } finally {
      saving = false;
    }
  }
</script>

{#if renaming}
  <form class="rename" onsubmit={save}>
    <input bind:value={field} placeholder={designation} maxlength={MAX_NAME_CHARS} aria-label={t.worldInfo.rename} />
    <button type="submit" disabled={saving}>{saving ? t.worldInfo.sending : t.worldInfo.renameSave}</button>
    <button type="button" onclick={() => (renaming = false)}>{t.worldInfo.renameCancel}</button>
  </form>
{:else}
  <strong>{name}</strong>
  <button class="link" onclick={start} title={t.worldInfo.renameHelp}>{t.worldInfo.rename}</button>
{/if}

<style>
  .link {
    border: none;
    background: none;
    padding: 0;
    color: var(--link);
  }
  .rename {
    display: flex;
    gap: 4px;
    flex: 1;
  }
  .rename input {
    flex: 1;
    min-width: 0;
  }
</style>
