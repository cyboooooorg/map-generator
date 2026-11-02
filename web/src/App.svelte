<script lang="ts">
  import { onMount } from 'svelte';
  import GenerateForm from './components/GenerateForm.svelte';
  import InfoPanel from './components/InfoPanel.svelte';
  import Viewer from './components/Viewer.svelte';
  import WorldList from './components/WorldList.svelte';
  import { api } from './lib/api/client';
  import type { FormSpec } from './lib/api/types/FormSpec';
  import type { PresetInfo } from './lib/api/types/PresetInfo';
  import type { WorldMetadata } from './lib/api/types/WorldMetadata';
  import type { WorldSummary } from './lib/api/types/WorldSummary';
  import type { LonLat } from './lib/geo';
  import { loadLayers, type WorldLayers } from './lib/layers';

  let worlds = $state<WorldSummary[]>([]);
  let presets = $state<PresetInfo[]>([]);
  let spec = $state<FormSpec | null>(null);
  let selectedId = $state<string | null>(null);
  let meta = $state<WorldMetadata | null>(null);
  let layers = $state<WorldLayers | null>(null);
  let picked = $state<LonLat | null>(null);
  let generating = $state(false);
  let error = $state<string | null>(null);
  /**
   * Which pane fills the screen on narrow layouts (the CSS ignores it on wide
   * ones, where the sidebar and the viewer sit side by side).
   */
  let pane = $state<'sidebar' | 'viewer'>('sidebar');

  async function refresh(): Promise<void> {
    worlds = await api.worlds();
  }

  onMount(async () => {
    try {
      [presets, spec] = await Promise.all([api.presets(), api.form()]);
      await refresh();
      selectedId = worlds[0]?.id ?? null;
      if (selectedId) {
        pane = 'viewer';
      }
    } catch (e) {
      error = `Cannot reach the server: ${String(e)}`;
    }
  });

  // Load the metadata, then the raw layers in a worker, of the selected world.
  $effect(() => {
    const id = selectedId;
    meta = null;
    layers = null;
    picked = null;
    if (!id) {
      return;
    }
    let cancelled = false;
    api
      .metadata(id)
      .then(async (m) => {
        if (cancelled) return;
        meta = m;
        const decoded = await loadLayers(id, m);
        if (!cancelled) layers = decoded;
      })
      .catch((e: unknown) => {
        if (!cancelled) error = `Could not load ${id}: ${String(e)}`;
      });
    return () => {
      cancelled = true;
    };
  });

  async function remove(world: WorldSummary): Promise<void> {
    if (!confirm(`Delete ${world.id}? This removes its files.`)) {
      return;
    }
    try {
      await api.deleteWorld(world.id);
      if (selectedId === world.id) {
        selectedId = null;
      }
      await refresh();
    } catch (e) {
      error = String(e);
    }
  }

  function select(id: string): void {
    selectedId = id;
    pane = 'viewer';
  }

  function startGenerating(): void {
    generating = true;
    pane = 'sidebar';
  }

  async function generated(worldId: string): Promise<void> {
    generating = false;
    await refresh();
    selectedId = worldId;
    pane = 'viewer';
  }
</script>

<div class="layout" class:show-viewer={pane === 'viewer'}>
  <aside class="sidebar">
    <header>
      <h1>Planets</h1>
      <!-- Only shown on narrow screens, where one pane is visible at a time. -->
      <div class="tabs">
        <button class:active={pane === 'sidebar'} aria-pressed={pane === 'sidebar'} onclick={() => (pane = 'sidebar')}
          >Worlds</button
        >
        <button class:active={pane === 'viewer'} aria-pressed={pane === 'viewer'} onclick={() => (pane = 'viewer')}
          >Viewer</button
        >
      </div>
      <button class="primary" onclick={startGenerating} disabled={!spec || generating}>New world</button>
    </header>
    {#if error}
      <p class="error">{error} <button onclick={() => (error = null)}>Dismiss</button></p>
    {/if}
    <!-- Hidden with CSS, not unmounted, so that a running generation keeps
         following its job while the viewer pane is shown. -->
    <div class="content">
      {#if generating && spec}
        <GenerateForm {presets} {spec} onDone={generated} onCancel={() => (generating = false)} />
      {:else}
        <WorldList {worlds} {selectedId} onSelect={select} onDelete={remove} />
      {/if}
    </div>
  </aside>

  <main>
    <Viewer
      worldId={selectedId}
      revision={worlds.find((w) => w.id === selectedId)?.modified_ms ?? null}
      {meta}
      {layers}
      onPick={(point) => (picked = point)}
    />
    {#if picked && meta}
      <InfoPanel point={picked} {meta} {layers} onClose={() => (picked = null)} />
    {/if}
  </main>
</div>

<style>
  .layout {
    display: grid;
    grid-template-columns: 340px minmax(0, 1fr);
    height: 100%;
  }
  .sidebar {
    border-right: 1px solid var(--border);
    padding: 14px;
    overflow-y: auto;
    min-width: 0;
    display: grid;
    /* Keep wide content (a form control's intrinsic width) inside the column. */
    grid-template-columns: minmax(0, 1fr);
    align-content: start;
    gap: 12px;
  }
  header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  h1 {
    margin: 0;
    font-size: 1.3rem;
  }
  main {
    position: relative;
    min-width: 0;
    min-height: 0;
    /* The viewer overlays (InfoPanel) adapt to the viewer width, not the
       window: the sidebar takes 340 px of it on wide layouts. */
    container-type: inline-size;
  }
  .error {
    color: var(--danger);
    margin: 0;
  }
  .tabs {
    display: none;
  }
  /* Narrow screens: one pane at a time, chosen with the tabs in the header,
     instead of a short sidebar band above the viewer. The sidebar pane is the
     only scroll area and fills the screen; the viewer pane gets all the height
     below the header, so dragging the globe never fights a page scroll. */
  @media (max-width: 760px) {
    .layout {
      grid-template-columns: minmax(0, 1fr);
      grid-template-rows: minmax(0, 1fr);
    }
    .layout.show-viewer {
      grid-template-rows: auto minmax(0, 1fr);
    }
    .layout:not(.show-viewer) main,
    .layout.show-viewer .content {
      display: none;
    }
    .sidebar {
      border-right: none;
      /* The header pads itself, so that it can stick to the very top. */
      padding-top: 0;
      padding-bottom: 0;
    }
    .layout.show-viewer .sidebar {
      border-bottom: 1px solid var(--border);
      padding-bottom: 10px;
    }
    header {
      position: sticky;
      top: 0;
      z-index: 1;
      gap: 8px;
      padding-top: 10px;
      background: var(--bg);
    }
    .layout:not(.show-viewer) header {
      padding-bottom: 10px;
      border-bottom: 1px solid var(--border);
      margin: 0 -14px;
      padding-left: 14px;
      padding-right: 14px;
    }
    .content {
      padding-bottom: 14px;
    }
    .tabs {
      display: flex;
      border: 1px solid var(--border);
      border-radius: 6px;
      overflow: hidden;
      margin-left: auto;
    }
    .tabs button {
      border: none;
      border-radius: 0;
    }
    .tabs button.active {
      background: var(--accent);
      color: var(--accent-text);
    }
  }
</style>
