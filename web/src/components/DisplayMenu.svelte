<script lang="ts">
  // The cog of the main menu and its "Display" menu: one radio item per
  // view. Escape or a click outside closes it, the arrows move between the
  // items, and the focus goes back to the cog when it closes.
  import { tick } from 'svelte';
  import { t } from '../lib/i18n/index.svelte';
  import { VIEWS, type View } from '../lib/menuCards';

  interface Props {
    view: View;
    onPick: (view: View) => void;
  }

  let { view, onPick }: Props = $props();

  let open = $state(false);
  let cog = $state<HTMLButtonElement>();
  let menu = $state<HTMLDivElement>();
  const items = () => [...(menu?.querySelectorAll<HTMLElement>('[role="menuitemradio"]') ?? [])];

  async function show(): Promise<void> {
    open = true;
    await tick();
    (items().find((i) => i.getAttribute('aria-checked') === 'true') ?? items()[0])?.focus();
  }
  function close(refocus = true): void {
    open = false;
    if (refocus) cog?.focus();
  }
  function pick(next: View): void {
    onPick(next);
    close();
  }
  function onKeydown(e: KeyboardEvent): void {
    if (!open) return;
    if (e.key === 'Escape') {
      e.preventDefault();
      close();
      return;
    }
    if (e.key === 'Tab') {
      close(false);
      return;
    }
    const all = items();
    const at = all.indexOf(document.activeElement as HTMLElement);
    // The arrows belong to the menu only while the focus is in it.
    if (at < 0) return;
    const move = (to: number) => {
      e.preventDefault();
      all[(to + all.length) % all.length]?.focus();
    };
    if (e.key === 'ArrowDown') move(at + 1);
    else if (e.key === 'ArrowUp') move(at - 1);
    else if (e.key === 'Home') move(0);
    else if (e.key === 'End') move(all.length - 1);
  }
  function onOutside(e: PointerEvent): void {
    if (open && e.target instanceof Node && !menu?.contains(e.target) && !cog?.contains(e.target)) close(false);
  }
</script>

<svelte:window onpointerdown={onOutside} onkeydown={onKeydown} />

<span class="display">
  <button
    bind:this={cog}
    class="cog"
    aria-haspopup="menu"
    aria-expanded={open}
    aria-controls={open ? 'display-menu' : undefined}
    aria-label={t.systemList.displayLabel}
    title={t.systemList.displayLabel}
    onclick={() => (open ? close() : void show())}
  >
    <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"
      ><circle cx="12" cy="12" r="3" /><path
        d="M19.4 15a1.7 1.7 0 0 0 .3 1.8l.1.1a2 2 0 1 1-2.8 2.8l-.1-.1a1.7 1.7 0 0 0-1.8-.3 1.7 1.7 0 0 0-1 1.5V21a2 2 0 1 1-4 0v-.1a1.7 1.7 0 0 0-1.1-1.5 1.7 1.7 0 0 0-1.8.3l-.1.1a2 2 0 1 1-2.8-2.8l.1-.1a1.7 1.7 0 0 0 .3-1.8 1.7 1.7 0 0 0-1.5-1H3a2 2 0 1 1 0-4h.1a1.7 1.7 0 0 0 1.5-1.1 1.7 1.7 0 0 0-.3-1.8l-.1-.1a2 2 0 1 1 2.8-2.8l.1.1a1.7 1.7 0 0 0 1.8.3H9a1.7 1.7 0 0 0 1-1.5V3a2 2 0 1 1 4 0v.1a1.7 1.7 0 0 0 1 1.5 1.7 1.7 0 0 0 1.8-.3l.1-.1a2 2 0 1 1 2.8 2.8l-.1.1a1.7 1.7 0 0 0-.3 1.8V9a1.7 1.7 0 0 0 1.5 1H21a2 2 0 1 1 0 4h-.1a1.7 1.7 0 0 0-1.5 1z"
      /></svg
    >
  </button>
  {#if open}
    <div bind:this={menu} id="display-menu" class="menu" role="menu" aria-label={t.systemList.displayLabel}>
      <span class="title" aria-hidden="true">{t.systemList.displayLabel}</span>
      {#each VIEWS as option (option)}
        <button role="menuitemradio" aria-checked={view === option} class:checked={view === option} onclick={() => pick(option)}>
          <span class="mark" aria-hidden="true">{view === option ? '✓' : ''}</span>{t.systemList.views[option]}
        </button>
      {/each}
      <span class="note">{t.systemList.displayHelp}</span>
    </div>
  {/if}
</span>

<style>
  .display {
    position: relative;
    display: inline-flex;
  }
  .cog {
    display: grid;
    place-items: center;
    width: 36px;
    padding: 0;
  }
  .menu {
    position: absolute;
    right: 0;
    top: calc(100% + 6px);
    z-index: 10;
    display: flex;
    flex-direction: column;
    gap: 2px;
    width: 240px;
    max-width: calc(100vw - 32px);
    padding: 6px;
    background: var(--panel-solid);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    box-shadow: 0 12px 30px rgb(0 0 0 / 0.45);
  }
  .title,
  .note {
    padding: 4px 10px;
    color: var(--muted);
    font-size: 0.85em;
  }
  .title {
    text-transform: uppercase;
    letter-spacing: 0.06em;
  }
  .menu button {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 10px;
    text-align: left;
    background: transparent;
    border-color: transparent;
  }
  .menu button:hover,
  .menu button:focus-visible {
    border-color: var(--accent);
  }
  .menu button.checked {
    background: color-mix(in srgb, var(--accent) 22%, transparent);
  }
  .mark {
    width: 14px;
    color: var(--link);
  }
  @media (hover: none), (pointer: coarse) {
    .menu button {
      min-height: 44px;
    }
    .cog {
      width: 44px;
    }
  }
</style>
