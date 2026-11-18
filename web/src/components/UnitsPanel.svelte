<script lang="ts">
  // The units reference: what each unit of the interface means, with
  // familiar values. Takes the focus when it opens and gives it back when it
  // closes, so a keyboard user lands where they were.
  import { t } from '../lib/i18n/index.svelte';
  import { panelUnits } from '../lib/units';

  let { onClose }: { onClose: () => void } = $props();

  let dialog = $state<HTMLDivElement | null>(null);
  $effect(() => {
    if (!dialog) {
      return;
    }
    const previous = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    dialog.focus();
    return () => {
      if (previous?.isConnected) {
        previous.focus();
      }
    };
  });

  function onkeydown(event: KeyboardEvent): void {
    // Also in a compare pane, which has no other shortcuts.
    if (event.key === 'Escape') {
      event.preventDefault();
      event.stopPropagation();
      onClose();
    }
  }
</script>

<div class="units-panel" role="dialog" aria-label={t.units.title} tabindex="-1" bind:this={dialog} {onkeydown}>
  <header>
    <strong>{t.units.title}</strong>
    <button class="close" onclick={onClose} aria-label={t.viewer.close}>×</button>
  </header>
  <dl>
    {#each panelUnits() as unit (unit)}
      <dt><abbr>{unit}</abbr></dt>
      <dd>{t.units[unit].name}<br /><small>{t.units[unit].examples}</small></dd>
    {/each}
  </dl>
</div>

<style>
  .units-panel {
    position: absolute;
    top: 52px;
    left: 12px;
    width: min(860px, calc(100% - 24px));
    max-height: calc(100% - 140px);
    overflow-y: auto;
    /* Framed as the theme frames panels (see app.css and lib/themes.ts). */
    background: var(--frame-deco), var(--panel-solid);
    clip-path: var(--frame-clip);
    box-shadow: var(--glow);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 10px 14px;
    z-index: 2;
  }
  .units-panel:focus-visible {
    outline: 2px solid var(--accent);
  }
  header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 6px;
  }
  dl {
    display: grid;
    grid-template-columns: repeat(auto-fill, 5em minmax(13em, 1fr));
    gap: 4px 12px;
    margin: 0;
  }
  dd {
    margin: 0;
  }
  small {
    color: var(--muted);
  }
  .close {
    border: none;
    background: none;
    font-size: 1.2rem;
    padding: 0 4px;
  }
</style>
