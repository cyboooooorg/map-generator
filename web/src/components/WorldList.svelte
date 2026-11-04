<script lang="ts">
  import { getLocale, t } from '../lib/i18n/index.svelte';
  import type { WorldSummary } from '../lib/api/types/WorldSummary';
  import { formatAgo, formatNumber } from '../lib/format';
  import { loadSetting, saveSetting } from '../lib/settings';
  import Thumbnail from './Thumbnail.svelte';

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
  const SORTS: { id: Sort }[] = [
    { id: 'newest' },
    { id: 'oldest' },
    { id: 'planet' },
    { id: 'seed' },
    { id: 'size' },
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
  <p class="empty">{t.worldList.empty}</p>
{:else}
  <div class="toolbar">
    <input type="search" placeholder={t.worldList.search} bind:value={query} aria-label={t.worldList.searchLabel} />
    <select bind:value={planet} aria-label={t.worldList.filterLabel}>
      <option value="">{t.worldList.allPlanets}</option>
      {#each planets as name (name)}
        <option value={name}>{name}</option>
      {/each}
    </select>
    <select bind:value={sort} aria-label={t.worldList.sortLabel}>
      {#each SORTS as option (option.id)}
        <option value={option.id}>{t.worldList.sorts[option.id]}</option>
      {/each}
    </select>
    <button onclick={() => (selecting ? stopSelecting() : (selecting = true))} aria-pressed={selecting}>
      {selecting ? t.worldList.done : t.worldList.select}
    </button>
  </div>

  {#if selecting}
    <div class="selection">
      <small>{t.worldList.selected({ count: chosen.length })}</small>
      <button class="link" onclick={() => (chosen = shown.map((w) => w.id))}>{t.worldList.all}</button>
      <button class="link" onclick={() => (chosen = [])} disabled={chosen.length === 0}>{t.worldList.none}</button>
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
        title={t.worldList.compareHelp}>{t.worldList.compare}</button
      >
      <button
        class="danger"
        onclick={() => {
          onDelete(chosen);
          stopSelecting();
        }}
        disabled={chosen.length === 0}>{t.worldList.delete}</button
      >
    </div>
  {/if}

  {#if shown.length === 0}
    <p class="empty">{t.worldList.noMatch}</p>
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
              aria-label={t.worldList.selectOne({ id: world.id })}
            />
          </label>
        {/if}
        <button class="pick" onclick={() => (selecting ? toggle(world.id) : onSelect(world.id))}>
          <Thumbnail id={world.id} revision={world.modified_ms} width={72} height={40} />
          <span class="text">
            <strong>{world.planet}</strong>
            <small
              >{t.worldList.seedSize({
                seed: world.seed,
                width: world.width,
                height: world.height,
                star: world.star ?? '',
              })}</small
            >
            <small title={new Date(world.modified_ms).toLocaleString(getLocale())}
              >{t.worldList.details({
                ago: formatAgo(world.modified_ms),
                sea: formatNumber(world.sea_level, 2),
                tilt: formatNumber(world.axial_tilt_deg),
              })}</small
            >
          </span>
        </button>
        {#if !selecting}
          <button class="delete" onclick={() => onDelete([world.id])} aria-label={t.worldList.deleteOne({ id: world.id })}>×</button>
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
