<script lang="ts" module>
  import type { IconName } from '../../lib/icons';

  /**
   * A small square beside a switch, for one part of what it switches (the
   * body shown or its host planet; the local group or the whole star
   * system): its own state, named in full for the screen reader.
   */
  export interface SubToggle {
    key: string;
    label: string;
    /** Its glyph: it tells which body or scope the square is for. */
    icon: IconName;
    on: boolean;
    set: (on: boolean) => void;
    /** Nothing for the square to switch (the host has no clouds to draw, say). */
    disabled?: boolean;
  }

  /**
   * A display switch of the icon bar: its icon, its name (the tooltip), its
   * state and what flips it; and, for a switch with parts, its squares
   * (`set` is then the main button's, which sets all of them).
   */
  export interface Toggle {
    key: string;
    label: string;
    help?: string;
    icon: IconName;
    on: boolean;
    set: (on: boolean) => void;
    subs?: readonly SubToggle[];
    /** A small button after the squares, for an action rather than a state (reset the drift). */
    action?: { label: string; icon: IconName; run: () => void };
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
          <div class="row" class:collapsed={!expanded}>
            <button
              type="button"
              class="main"
              class:on={toggle.on}
              aria-pressed={toggle.on}
              aria-label={toggle.label}
              title={toggle.help ? `${toggle.label}: ${toggle.help}` : toggle.label}
              onclick={() => toggle.set(!toggle.on)}
              ><Icon name={toggle.icon} />{#if expanded}<span class="name">{toggle.label}</span>{/if}</button
            >
            {#if toggle.action}
              <button
                type="button"
                class="sub action"
                aria-label={toggle.action.label}
                title={toggle.action.label}
                onclick={toggle.action.run}><Icon name={toggle.action.icon} size={expanded ? 13 : 9} /></button
              >
            {/if}
            {#if toggle.subs && toggle.subs.length > 0}
              <!-- Icons only: the squares shrink to dots stacked beside the icon. -->
              <div class="subs" role="group" aria-label={toggle.label}>
                {#each toggle.subs as sub (sub.key)}
                  <button
                    type="button"
                    class="sub"
                    class:on={sub.on}
                    aria-pressed={sub.on}
                    aria-label={sub.label}
                    title={sub.label}
                    disabled={sub.disabled}
                    onclick={() => sub.set(!sub.on)}><Icon name={sub.icon} size={expanded ? 13 : 9} /></button
                  >
                {/each}
              </div>
            {/if}
          </div>
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
  .row {
    display: flex;
    align-items: stretch;
    gap: 2px;
  }
  .main {
    flex: 1;
  }
  .subs {
    display: flex;
    gap: 2px;
  }
  .collapsed .subs {
    flex-direction: column;
    justify-content: center;
    gap: 1px;
  }
  .sub {
    width: 24px;
    padding: 0;
    justify-content: center;
    border-radius: 3px;
    border-color: var(--border);
  }
  .collapsed .sub {
    width: 14px;
    height: 14px;
  }
  .sub:not(.on) {
    opacity: 0.55;
  }
  .sub.action {
    color: var(--text);
  }
  .sub:disabled {
    opacity: 0.25;
    cursor: default;
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
