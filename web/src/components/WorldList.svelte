<script lang="ts">
  import { worldFile } from '../lib/api/client';
  import type { WorldSummary } from '../lib/api/types/WorldSummary';
  import { formatAgo } from '../lib/format';
  import { loadSetting, saveSetting } from '../lib/settings';

  interface Props {
    worlds: WorldSummary[];
    selectedId: string | null;
    compareId: string | null;
    onSelect: (id: string) => void;
    onDelete: (ids: string[]) => void;
    onCompare: (a: string, b: string) => void;
    /** Ids of the worlds shown, in their order (written by the list). */
    order?: string[];
  }

  let { worlds, selectedId, compareId, onSelect, onDelete, onCompare, order = $bindable([]) }: Props = $props();

  type Sort = 'newest' | 'oldest' | 'planet' | 'seed' | 'size';
  const SORTS: { id: Sort; label: string }[] = [
    { id: 'newest', label: 'Newest first' },
    { id: 'oldest', label: 'Oldest first' },
    { id: 'planet', label: 'Planet' },
    { id: 'seed', label: 'Seed' },
    { id: 'size', label: 'Largest map' },
  ];
  const COMPARE: Record<Sort, (a: WorldSummary, b: WorldSummary) => number> = {
    newest: (a, b) => b.modified_ms - a.modified_ms,
    oldest: (a, b) => a.modified_ms - b.modified_ms,
    planet: (a, b) => a.planet.localeCompare(b.planet) || b.modified_ms - a.modified_ms,
    seed: (a, b) => a.seed - b.seed,
    size: (a, b) => b.width * b.height - a.width * a.height || b.modified_ms - a.modified_ms,
  };

  const saved = loadSetting('worldList', { sort: 'newest' as string, planet: '' });
  let sort = $state<Sort>(SORTS.some((s) => s.id === saved.sort) ? (saved.sort as Sort) : 'newest');
  let planet = $state(saved.planet);
  let query = $state('');
  let selecting = $state(false);
  let chosen = $state<string[]>([]);
  $effect(() => saveSetting('worldList', { sort, planet }));

  const planets = $derived([...new Set(worlds.map((w) => w.planet))].sort());
  const shown = $derived.by(() => {
    const q = query.trim().toLowerCase();
    return worlds
      .filter((w) => (planet === '' || w.planet === planet) && (q === '' || w.id.toLowerCase().includes(q)))
      .sort(COMPARE[sort]);
  });
  $effect(() => {
    order = shown.map((w) => w.id);
  });
  // Forget chosen worlds that disappeared (deleted).
  $effect(() => {
    const ids = new Set(worlds.map((w) => w.id));
    if (chosen.some((id) => !ids.has(id))) {
      chosen = chosen.filter((id) => ids.has(id));
    }
  });

  function toggle(id: string): void {
    chosen = chosen.includes(id) ? chosen.filter((c) => c !== id) : [...chosen, id];
  }

  function stopSelecting(): void {
    selecting = false;
    chosen = [];
  }
</script>

{#if worlds.length === 0}
  <p class="empty">No world yet.</p>
{:else}
  <div class="toolbar">
    <input type="search" placeholder="Search id or seed" bind:value={query} aria-label="Search worlds" />
    <select bind:value={planet} aria-label="Filter by planet">
      <option value="">All planets</option>
      {#each planets as name (name)}
        <option value={name}>{name}</option>
      {/each}
    </select>
    <select bind:value={sort} aria-label="Sort">
      {#each SORTS as option (option.id)}
        <option value={option.id}>{option.label}</option>
      {/each}
    </select>
    <button onclick={() => (selecting ? stopSelecting() : (selecting = true))} aria-pressed={selecting}>
      {selecting ? 'Done' : 'Select'}
    </button>
  </div>

  {#if selecting}
    <div class="selection">
      <small>{chosen.length} selected</small>
      <button class="link" onclick={() => (chosen = shown.map((w) => w.id))}>all</button>
      <button class="link" onclick={() => (chosen = [])} disabled={chosen.length === 0}>none</button>
      <span class="spacer"></span>
      <button
        onclick={() => {
          const [a, b] = chosen;
          if (a && b) {
            onCompare(a, b);
            stopSelecting();
          }
        }}
        disabled={chosen.length !== 2}
        title="Select exactly two worlds">Compare</button
      >
      <button
        class="danger"
        onclick={() => {
          onDelete(chosen);
          stopSelecting();
        }}
        disabled={chosen.length === 0}>Delete</button
      >
    </div>
  {/if}

  {#if shown.length === 0}
    <p class="empty">No world matches.</p>
  {/if}
  <ul>
    {#each shown as world (world.id)}
      <li class:selected={world.id === selectedId} class:compared={world.id === compareId}>
        {#if selecting}
          <label class="choose">
            <input
              type="checkbox"
              checked={chosen.includes(world.id)}
              onchange={() => toggle(world.id)}
              aria-label={`Select ${world.id}`}
            />
          </label>
        {/if}
        <button class="pick" onclick={() => (selecting ? toggle(world.id) : onSelect(world.id))}>
          <img src={worldFile(world.id, 'world.png', world.modified_ms)} alt="" loading="lazy" />
          <span class="text">
            <strong>{world.planet}</strong>
            <small>seed {world.seed} · {world.width}×{world.height}</small>
            <small title={new Date(world.modified_ms).toLocaleString()}
              >{formatAgo(world.modified_ms)} · sea {world.sea_level.toFixed(2)} · tilt {world.axial_tilt_deg.toFixed(
                0,
              )}°</small
            >
          </span>
        </button>
        {#if !selecting}
          <button class="delete" onclick={() => onDelete([world.id])} aria-label={`Delete ${world.id}`}>×</button>
        {/if}
      </li>
    {/each}
  </ul>
{/if}

<style>
  .toolbar {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
    gap: 6px;
    margin-bottom: 8px;
  }
  .toolbar input {
    grid-column: 1 / -1;
    min-width: 0;
    background: #1b2130;
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 6px 10px;
  }
  .toolbar select {
    min-width: 0;
  }
  .toolbar button {
    grid-column: 1 / -1;
  }
  .selection {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 8px;
  }
  .selection small {
    color: var(--muted);
  }
  .spacer {
    flex: 1;
  }
  .link {
    border: none;
    background: none;
    padding: 0;
    color: var(--link);
  }
  .danger:hover:not(:disabled) {
    border-color: var(--danger);
    color: var(--danger);
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 6px;
  }
  li {
    display: flex;
    align-items: stretch;
    border: 1px solid var(--border);
    border-radius: 8px;
    overflow: hidden;
  }
  li.selected {
    border-color: var(--accent);
  }
  li.compared {
    border-style: dashed;
    border-color: var(--accent);
  }
  .choose {
    display: grid;
    place-items: center;
    padding: 0 0 0 8px;
  }
  .pick {
    flex: 1;
    display: flex;
    gap: 10px;
    align-items: center;
    border: none;
    border-radius: 0;
    text-align: left;
    padding: 6px;
    min-width: 0;
  }
  img {
    width: 72px;
    height: 40px;
    object-fit: cover;
    border-radius: 4px;
    flex: none;
  }
  .text {
    display: grid;
    min-width: 0;
  }
  small {
    color: var(--muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .delete {
    border: none;
    border-radius: 0;
    padding: 0 10px;
    color: var(--muted);
  }
  .delete:hover {
    color: var(--danger);
  }
  .empty {
    color: var(--muted);
  }
</style>
