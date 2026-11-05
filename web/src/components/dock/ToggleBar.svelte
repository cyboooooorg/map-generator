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
  import { t } from '../../lib/i18n/index.svelte';
  import { loadSetting, saveSetting } from '../../lib/settings';

  let { groups, label }: { groups: readonly (readonly Toggle[])[]; label: string } = $props();
  /** Whether the bar shows each switch's name beside its icon (remembered). */
  let expanded = $state(loadSetting('toggles.expanded', false));
  $effect(() => saveSetting('toggles.expanded', expanded));
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
            onclick={() => toggle.set(!toggle.on)}
            ><Icon name={toggle.icon} />{#if expanded}<span class="name">{toggle.label}</span>{/if}</button
          >
        {/each}
      </div>
    {/if}
  {/each}
  <button
    type="button"
    class="expand"
    aria-expanded={expanded}
    title={expanded ? t.toggleBar.collapse : t.toggleBar.expand}
    aria-label={expanded ? t.toggleBar.collapse : t.toggleBar.expand}
    onclick={() => (expanded = !expanded)}><Icon name={expanded ? 'unfold' : 'fold'} size={16} /></button
  >
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
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px;
    background: transparent;
    border-color: transparent;
    color: var(--muted);
    text-align: left;
  }
  .name {
    /* Short names; a long one wraps between its words. */
    max-width: 9em;
    font-size: 0.8rem;
    line-height: 1.15;
  }
  .expand {
    align-self: flex-start;
    padding: 4px 6px;
  }
  button.on {
    color: var(--text);
    background: var(--control-bg);
    border-color: var(--accent);
  }
</style>
