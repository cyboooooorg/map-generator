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

  async function refresh(): Promise<void> {
    worlds = await api.worlds();
  }

  onMount(async () => {
    try {
      [presets, spec] = await Promise.all([api.presets(), api.form()]);
      await refresh();
      selectedId = worlds[0]?.id ?? null;
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

  async function generated(worldId: string): Promise<void> {
    generating = false;
    await refresh();
    selectedId = worldId;
  }
</script>

<div class="layout">
  <aside class="sidebar">
    <header>
      <h1>Planets</h1>
      <button class="primary" onclick={() => (generating = true)} disabled={!spec || generating}>New world</button>
    </header>
    {#if error}
      <p class="error">{error} <button onclick={() => (error = null)}>Dismiss</button></p>
    {/if}
    {#if generating && spec}
      <GenerateForm {presets} {spec} onDone={generated} onCancel={() => (generating = false)} />
    {:else}
      <WorldList {worlds} {selectedId} onSelect={(id) => (selectedId = id)} onDelete={remove} />
    {/if}
  </aside>

  <main>
    <Viewer worldId={selectedId} {meta} {layers} onPick={(point) => (picked = point)} />
    {#if picked && meta}
      <InfoPanel point={picked} {meta} {layers} onClose={() => (picked = null)} />
    {/if}
  </main>
</div>

<style>
  .layout {
    display: grid;
    grid-template-columns: 340px 1fr;
    height: 100%;
  }
  .sidebar {
    border-right: 1px solid var(--border);
    padding: 14px;
    overflow-y: auto;
    display: grid;
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
  }
  .error {
    color: var(--danger);
    margin: 0;
  }
  @media (max-width: 760px) {
    .layout {
      grid-template-columns: 1fr;
      grid-template-rows: auto 60vh;
    }
    .sidebar {
      border-right: none;
      border-bottom: 1px solid var(--border);
    }
  }
</style>
