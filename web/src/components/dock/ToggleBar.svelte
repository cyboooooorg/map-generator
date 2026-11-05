<script lang="ts" module>
  import type { IconName } from '../../lib/icons';

  /** A display switch of the icon bar: its icon, its name (the tooltip), its state and what flips it. */
  export interface Toggle {
    key: string;
    label: string;
    help?: string;
    icon: IconName;
    on: boolean;
    set: (on: boolean) => void;
  }
</script>

<script lang="ts">
  // A viewer's display switches as a compact column of icon buttons, in
  // groups, each named in its tooltip and read out (aria-pressed): in place
  // of a panel of checkboxes.
  import Icon from '../Icon.svelte';

  let { groups, label }: { groups: readonly (readonly Toggle[])[]; label: string } = $props();
</script>

<div class="toggle-bar" role="toolbar" aria-label={label} aria-orientation="vertical">
  {#each groups as group, g (g)}
    {#if group.length > 0}
      <div class="group">
        {#each group as toggle (toggle.key)}
          <button
            type="button"
            class:on={toggle.on}
            aria-pressed={toggle.on}
            aria-label={toggle.label}
            title={toggle.help ? `${toggle.label}: ${toggle.help}` : toggle.label}
            onclick={() => toggle.set(!toggle.on)}><Icon name={toggle.icon} /></button
          >
        {/each}
      </div>
    {/if}
  {/each}
</div>

<style>
  .toggle-bar {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .group {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 3px;
    background: var(--frame-deco), var(--panel);
    clip-path: var(--frame-clip);
    box-shadow: var(--glow);
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }
  button {
    padding: 6px;
    background: transparent;
    border-color: transparent;
    color: var(--muted);
  }
  button.on {
    color: var(--text);
    background: var(--control-bg);
    border-color: var(--accent);
  }
</style>
