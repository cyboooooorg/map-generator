<script lang="ts">
  // The main menu's systems: a grid of cards, one per drawn system, each
  // with its orrery, its name, its star, its planets by type and its date,
  // searched, sorted, selected to compare or delete (docs/system-spec.md,
  // *Interface*).
  import { getLocale, t } from '../lib/i18n/index.svelte';
  import type { SystemSummary } from '../lib/api/types/SystemSummary';
  import { formatAgo } from '../lib/format';
  import { presetName } from '../lib/i18n/names.svelte';
  import { loadSetting, saveSetting } from '../lib/settings';
  import SystemOrrery from './SystemOrrery.svelte';

  interface Props {
    systems: SystemSummary[];
    selectedId: string | null;
    compareId: string | null;
    onSelect: (id: string) => void;
    onDelete: (ids: string[]) => void;
    onCompare: (a: string, b: string) => void;
    /** Ids of the systems shown, in their order (written by the list). */
    order?: string[];
  }

  let { systems, selectedId, compareId, onSelect, onDelete, onCompare, order = $bindable([]) }: Props = $props();

  type Sort = 'newest' | 'oldest' | 'planets' | 'seed';
  const SORTS: Sort[] = ['newest', 'oldest', 'planets', 'seed'];
  const planetCount = (s: SystemSummary) => s.bodies.filter((b) => b.kind === 'planet').length;
  const COMPARE: Record<Sort, (a: SystemSummary, b: SystemSummary) => number> = {
    newest: (a, b) => b.modified_ms - a.modified_ms,
    oldest: (a, b) => a.modified_ms - b.modified_ms,
    planets: (a, b) => planetCount(b) - planetCount(a) || b.modified_ms - a.modified_ms,
    seed: (a, b) => a.seed - b.seed,
  };

  const saved = loadSetting('systemList', { sort: 'newest' as string });
  let sort = $state<Sort>(SORTS.includes(saved.sort as Sort) ? (saved.sort as Sort) : 'newest');
  let query = $state('');
  let selecting = $state(false);
  let chosen = $state<string[]>([]);
  $effect(() => saveSetting('systemList', { sort }));

  /** A system's name: the one given to its star, else its designation. */
  const nameOf = (s: SystemSummary) => s.names['star'] ?? s.designation;
  const shown = $derived.by(() => {
    const q = query.trim().toLowerCase();
    const matches = (s: SystemSummary) =>
      q === '' ||
      [s.id, s.designation, String(s.seed), ...Object.values(s.names), ...s.bodies.map((b) => b.designation)].some((text) =>
        text.toLowerCase().includes(q),
      );
    return systems.filter(matches).sort(COMPARE[sort]);
  });
  $effect(() => {
    order = shown.map((s) => s.id);
  });
  $effect(() => {
    const ids = new Set(systems.map((s) => s.id));
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

  /** The planets per type, as a short line. */
  function counts(s: SystemSummary): string {
    return Object.entries(s.planets)
      .map(([type, n]) => `${n} ${presetName({ name: type })}`)
      .join(' · ');
  }
</script>

{#if systems.length === 0}
  <p class="empty">{t.systemList.empty}</p>
{:else}
  <div class="toolbar">
    <input type="search" placeholder={t.systemList.search} bind:value={query} aria-label={t.systemList.searchLabel} />
    <select bind:value={sort} aria-label={t.worldList.sortLabel}>
      {#each SORTS as option (option)}
        <option value={option}>{t.systemList.sorts[option]}</option>
      {/each}
    </select>
    <button onclick={() => (selecting ? stopSelecting() : (selecting = true))} aria-pressed={selecting}>
      {selecting ? t.worldList.done : t.worldList.select}
    </button>
    {#if selecting}
      <span class="selection">
        <small>{t.worldList.selected({ count: chosen.length })}</small>
        <button class="link" onclick={() => (chosen = shown.map((s) => s.id))}>{t.worldList.all}</button>
        <button class="link" onclick={() => (chosen = [])} disabled={chosen.length === 0}>{t.worldList.none}</button>
        <button
          onclick={() => {
            const [a, b] = chosen;
            if (a && b) {
              onCompare(a, b);
              stopSelecting();
            }
          }}
          disabled={chosen.length !== 2}
          title={t.systemList.compareHelp}>{t.worldList.compare}</button
        >
        <button
          class="danger"
          onclick={() => {
            onDelete(chosen);
            stopSelecting();
          }}
          disabled={chosen.length === 0}>{t.worldList.delete}</button
        >
      </span>
    {/if}
  </div>

  {#if shown.length === 0}
    <p class="empty">{t.systemList.noMatch}</p>
  {/if}
  <ul class="grid">
    {#each shown as system (system.id)}
      <li class:selected={system.id === selectedId || chosen.includes(system.id)} class:compared={system.id === compareId}>
        <button
          class="card"
          aria-pressed={selecting ? chosen.includes(system.id) : undefined}
          onclick={() => (selecting ? toggle(system.id) : onSelect(system.id))}
        >
          <SystemOrrery {system} width={240} height={120} radius={0} />
          <span class="text">
            <strong>{nameOf(system)}</strong>
            <small>{counts(system)}</small>
            <small title={new Date(system.modified_ms).toLocaleString(getLocale())}
              >{t.systemList.details({ star: system.star, seed: system.seed, moons: system.moons, ago: formatAgo(system.modified_ms) })}</small
            >
          </span>
        </button>
        {#if selecting}
          <input
            class="choose"
            type="checkbox"
            checked={chosen.includes(system.id)}
            onchange={() => toggle(system.id)}
            aria-label={t.worldList.selectOne({ id: nameOf(system) })}
          />
        {:else}
          <button class="delete" onclick={() => onDelete([system.id])} aria-label={t.worldList.deleteOne({ id: nameOf(system) })} title={t.worldList.deleteOne({ id: nameOf(system) })}>×</button>
        {/if}
      </li>
    {/each}
  </ul>
{/if}

<style>
  .toolbar {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    align-items: center;
    margin-bottom: 12px;
  }
  .toolbar input[type='search'] {
    flex: 1 1 16em;
    min-width: 0;
  }
  .selection {
    display: flex;
    flex-wrap: wrap;
    gap: 6px 8px;
    align-items: center;
  }
  .grid {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(min(100%, 240px), 1fr));
    gap: 14px;
  }
  /* Every card of a row as tall as the tallest, its text at the top. */
  .grid {
    grid-auto-rows: 1fr;
  }
  li {
    position: relative;
    min-width: 0;
    display: grid;
  }
  .card {
    display: grid;
    grid-template-rows: auto 1fr;
    align-content: start;
    width: 100%;
    height: 100%;
    padding: 0;
    text-align: left;
    overflow: hidden;
    background: var(--frame-deco), var(--panel);
    clip-path: var(--frame-clip);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    box-shadow: var(--glow);
    color: var(--text);
    cursor: pointer;
  }
  .card :global(svg.orrery) {
    width: 100%;
    height: auto;
    aspect-ratio: 2 / 1;
  }
  .card:hover,
  .card:focus-visible {
    border-color: var(--accent);
  }
  li.selected .card {
    border-color: var(--accent);
    outline: 1px solid var(--accent);
  }
  li.compared .card {
    border-style: dashed;
    border-color: var(--accent);
  }
  .text {
    display: grid;
    align-content: start;
    gap: 2px;
    padding: 8px 10px 10px;
    min-width: 0;
  }
  .text strong {
    font-family: var(--font-display);
    overflow-wrap: anywhere;
  }
  .text small {
    color: var(--muted);
    overflow-wrap: anywhere;
  }
  .delete,
  .choose {
    position: absolute;
    top: 6px;
    right: 6px;
  }
  .delete {
    padding: 0 7px;
    line-height: 1.5;
    opacity: 0;
  }
  li:hover .delete,
  .delete:focus-visible {
    opacity: 1;
  }
  .choose {
    width: 18px;
    height: 18px;
  }
  .empty {
    color: var(--muted);
  }
  button.link {
    padding: 0;
    border: none;
    background: none;
    color: var(--link);
    cursor: pointer;
  }
  .danger {
    color: var(--danger);
  }
  @media (hover: none) {
    .delete {
      opacity: 1;
    }
  }
</style>
