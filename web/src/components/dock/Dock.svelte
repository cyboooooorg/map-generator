<script lang="ts" module>
  import type { Snippet } from 'svelte';
  import type { IconName } from '../../lib/icons';

  /** A panel of a viewer: its id (remembered), its title and icon, and what it shows. */
  export interface DockPanel {
    id: string;
    title: string;
    icon: IconName;
    content: Snippet;
  }
</script>

<script lang="ts">
  // A viewer's panels by theme (see docs/superpowers/specs/2026-11-05-
  // interface-redesign-design.md): a dock on the right with a tab per panel,
  // one panel open in it at a time, foldable to its icons; any panel can be
  // taken out into a floating card over the view, dragged by its header and
  // kept on screen, and docked again. The layout is remembered per kind of
  // viewer (lib/dock.ts). Narrow views have no floating cards.
  import { untrack } from 'svelte';
  import Icon from '../Icon.svelte';
  import { t } from '../../lib/i18n/index.svelte';
  import { clampToView, type DockKind, FLOAT_MIN_WIDTH, loadLayout, saveLayout } from '../../lib/dock';

  interface Props {
    kind: DockKind;
    panels: readonly DockPanel[];
    /** The panel open when nothing is remembered. */
    defaultActive?: string | null;
  }

  let { kind, panels, defaultActive = null }: Props = $props();

  const ids = $derived(panels.map((p) => p.id));
  let layout = $state(untrack(() => loadLayout(kind, ids, defaultActive ?? ids[0] ?? null)));
  $effect(() => saveLayout(kind, $state.snapshot(layout)));

  let viewWidth = $state(0);
  let viewHeight = $state(0);
  const narrow = $derived(viewWidth > 0 && viewWidth < FLOAT_MIN_WIDTH);
  /** The floating panels, front last; none in a narrow view. */
  let order = $state<string[]>(untrack(() => Object.keys(layout.floating)));
  const floating = $derived(narrow ? [] : order.filter((id) => layout.floating[id] && ids.includes(id)));
  const docked = $derived(layout.active && !floating.includes(layout.active) ? panels.find((p) => p.id === layout.active) : undefined);
  /** Each floating card's size once drawn, to keep it on screen. */
  let sizes = $state<Record<string, { width: number; height: number }>>({});

  const front = (id: string) => (order = [...order.filter((o) => o !== id), id]);

  function pick(id: string): void {
    if (floating.includes(id)) {
      front(id);
      return;
    }
    layout.folded = false;
    layout.active = layout.active === id ? null : id;
  }

  function detach(id: string): void {
    const size = sizes[id] ?? { width: 320, height: 240 };
    // On the left, beside the icon bar, each new card a step lower than the last.
    const step = 28 * Object.keys(layout.floating).length;
    layout.floating[id] = clampToView({ x: 64 + step, y: 72 + step }, size.width, size.height, viewWidth, viewHeight);
    if (layout.active === id) layout.active = null;
    front(id);
  }

  function dockBack(id: string): void {
    delete layout.floating[id];
    order = order.filter((o) => o !== id);
    layout.active = id;
    layout.folded = false;
  }

  /** Drags a floating card by its header. */
  function drag(id: string, event: PointerEvent): void {
    if (event.button !== 0 || (event.target as HTMLElement).closest('button')) return;
    const start = layout.floating[id];
    if (!start) return;
    front(id);
    const header = event.currentTarget as HTMLElement;
    header.setPointerCapture(event.pointerId);
    const from = { x: event.clientX, y: event.clientY };
    const move = (e: PointerEvent) => {
      const size = sizes[id] ?? { width: 320, height: 240 };
      layout.floating[id] = clampToView({ x: start.x + e.clientX - from.x, y: start.y + e.clientY - from.y }, size.width, size.height, viewWidth, viewHeight);
    };
    const end = () => {
      header.removeEventListener('pointermove', move);
      header.removeEventListener('pointerup', end);
      header.removeEventListener('pointercancel', end);
    };
    header.addEventListener('pointermove', move);
    header.addEventListener('pointerup', end);
    header.addEventListener('pointercancel', end);
  }

  // A resized view keeps every floating card inside it.
  $effect(() => {
    if (viewWidth === 0) return;
    untrack(() => {
      for (const id of Object.keys(layout.floating)) {
        const size = sizes[id] ?? { width: 320, height: 240 };
        const at = clampToView(layout.floating[id]!, size.width, size.height, viewWidth, viewHeight);
        if (at.x !== layout.floating[id]!.x || at.y !== layout.floating[id]!.y) layout.floating[id] = at;
      }
    });
  });
</script>

<div class="dock-root" bind:clientWidth={viewWidth} bind:clientHeight={viewHeight}>
  <aside class="dock" class:folded={layout.folded} class:narrow aria-label={t.dock.panels}>
    <div class="tabs" role="tablist" aria-orientation="vertical">
      {#each panels as panel (panel.id)}
        <button
          type="button"
          role="tab"
          class="tab"
          class:active={docked?.id === panel.id}
          class:floating={floating.includes(panel.id)}
          aria-selected={docked?.id === panel.id}
          title={panel.title}
          onclick={() => pick(panel.id)}
          ><Icon name={panel.icon} />{#if !layout.folded && !narrow}<span class="tab-title">{panel.title}</span>{/if}</button
        >
      {/each}
      <button
        type="button"
        class="tab fold"
        title={layout.folded ? t.dock.unfold : t.dock.fold}
        aria-label={layout.folded ? t.dock.unfold : t.dock.fold}
        onclick={() => (layout.folded = !layout.folded)}><Icon name={layout.folded ? 'unfold' : 'fold'} /></button
      >
    </div>
    {#if docked && !layout.folded}
      <div class="body" role="tabpanel" aria-label={docked.title}>
        <header>
          <h3>{docked.title}</h3>
          {#if !narrow}
            <button type="button" class="icon-button" title={t.dock.detach} aria-label={t.dock.detach} onclick={() => detach(docked.id)}
              ><Icon name="detach" size={16} /></button
            >
          {/if}
          <button type="button" class="icon-button" title={t.dock.close} aria-label={t.dock.close} onclick={() => (layout.active = null)}
            ><Icon name="close" size={16} /></button
          >
        </header>
        <div class="content">{@render docked.content()}</div>
      </div>
    {/if}
  </aside>
  {#each floating as id (id)}
    {@const panel = panels.find((p) => p.id === id)}
    {@const at = layout.floating[id]}
    {#if panel && at}
      <section
        class="float"
        aria-label={panel.title}
        style:left={`${at.x}px`}
        style:top={`${at.y}px`}
        style:z-index={10 + order.indexOf(id)}
        bind:offsetWidth={() => sizes[id]?.width ?? 0, (w) => (sizes[id] = { width: w, height: sizes[id]?.height ?? 0 })}
        bind:offsetHeight={() => sizes[id]?.height ?? 0, (h) => (sizes[id] = { width: sizes[id]?.width ?? 0, height: h })}
        onpointerdown={() => front(id)}
      >
        <!-- Dragged by the pointer; the dock button puts it back for keyboards. -->
        <header class="handle" role="toolbar" aria-label={panel.title} tabindex="-1" onpointerdown={(event) => drag(id, event)}>
          <Icon name={panel.icon} size={16} />
          <h3>{panel.title}</h3>
          <button type="button" class="icon-button" title={t.dock.dockBack} aria-label={t.dock.dockBack} onclick={() => dockBack(id)}
            ><Icon name="dock" size={16} /></button
          >
        </header>
        <div class="content">{@render panel.content()}</div>
      </section>
    {/if}
  {/each}
</div>

<style>
  .dock-root {
    position: absolute;
    inset: 0;
    pointer-events: none;
  }
  .dock,
  .float {
    pointer-events: auto;
  }
  .dock {
    position: absolute;
    /* Below the viewer's toolbar, above the playback bar. */
    top: 64px;
    right: 12px;
    bottom: 76px;
    display: flex;
    flex-direction: row-reverse;
    align-items: flex-start;
    gap: 8px;
    max-width: calc(100% - 24px);
  }
  /* The playback bar wraps to two lines on narrow views. */
  @media (max-width: 720px) {
    .dock {
      bottom: 132px;
    }
  }
  .tabs {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 4px;
    background: var(--frame-deco), var(--panel);
    clip-path: var(--frame-clip);
    box-shadow: var(--glow);
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }
  .tab {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 8px;
    text-align: left;
    background: transparent;
    border-color: transparent;
    /* The longest title in any language: the tab grows, the text wraps. */
    max-width: 12em;
  }
  .tab.active {
    background: var(--control-bg);
    border-color: var(--accent);
  }
  .tab.floating {
    color: var(--muted);
  }
  .tab-title {
    /* Wraps between words; a word longer than the tab breaks only then. */
    overflow-wrap: break-word;
    min-width: 0;
  }
  .fold {
    justify-content: center;
    color: var(--muted);
  }
  .body,
  .float {
    display: flex;
    flex-direction: column;
    background: var(--frame-deco), var(--panel);
    clip-path: var(--frame-clip);
    box-shadow: 0 4px 16px rgb(0 0 0 / 0.35), var(--glow);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    font-size: 0.85rem;
  }
  .body {
    width: clamp(260px, 26vw, 380px);
    max-height: 100%;
  }
  .narrow .body {
    width: min(380px, calc(100vw - 96px));
  }
  .float {
    position: absolute;
    /* Opaque: a card over the view must not show the labels through it. */
    background: var(--frame-deco), var(--panel-solid);
    width: clamp(240px, 24vw, 340px);
    max-height: calc(100% - 16px);
  }
  header {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 8px 6px 12px;
    border-bottom: 1px solid var(--border);
  }
  .handle {
    cursor: grab;
    touch-action: none;
  }
  .handle:active {
    cursor: grabbing;
  }
  h3 {
    flex: 1;
    margin: 0;
    font-size: 0.8rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--muted);
  }
  .icon-button {
    padding: 3px;
    background: transparent;
    border-color: transparent;
  }
  .content {
    overflow-y: auto;
    padding: 10px 12px;
    display: grid;
    gap: 6px;
  }
</style>
