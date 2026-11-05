<script lang="ts">
  // The library: the drawn systems, each with a strip of its planets in
  // order from the star, coloured as their previews, the generated ones
  // full and the others faded (docs/system-spec.md, *Interface*).
  import { getLocale, t } from '../lib/i18n/index.svelte';
  import type { SystemSummary } from '../lib/api/types/SystemSummary';
  import type { BodySummary } from '../lib/api/types/BodySummary';
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

  /** A linear colour as CSS. */
  function css([r, g, b]: [number, number, number]): string {
    const srgb = (c: number) => Math.round(255 * Math.min(1, Math.max(0, c)) ** (1 / 2.2));
    return `rgb(${srgb(r)} ${srgb(g)} ${srgb(b)})`;
  }
  /** A planet's dot in the strip: giants larger. */
  const GIANTS = new Set(['gas-giant', 'ice-giant', 'hot-jupiter']);
  const dotSize = (b: BodySummary) => (GIANTS.has(b.body_type) ? 12 : 8);
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
  </div>

  {#if selecting}
    <div class="selection">
      <small>{t.worldList.selected({ count: chosen.length })}</small>
      <button class="link" onclick={() => (chosen = shown.map((s) => s.id))}>{t.worldList.all}</button>
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
    </div>
  {/if}

  {#if shown.length === 0}
    <p class="empty">{t.systemList.noMatch}</p>
  {/if}
  <ul>
    {#each shown as system (system.id)}
      <li class:selected={system.id === selectedId} class:compared={system.id === compareId}>
        {#if selecting}
          <label class="choose">
            <input
              type="checkbox"
              checked={chosen.includes(system.id)}
              onchange={() => toggle(system.id)}
              aria-label={t.worldList.selectOne({ id: nameOf(system) })}
            />
          </label>
        {/if}
        <button class="pick" onclick={() => (selecting ? toggle(system.id) : onSelect(system.id))}>
          <SystemOrrery {system} width={72} height={40} />
          <span class="text">
            <strong>{nameOf(system)}</strong>
            <span class="strip" aria-label={t.systemList.stripLabel}>
              {#each system.bodies.filter((b) => b.kind === 'planet') as planet (planet.id)}
                <span
                  class="dot"
                  class:stub={!planet.generated}
                  style:background={css(planet.colour)}
                  style:width={`${dotSize(planet)}px`}
                  style:height={`${dotSize(planet)}px`}
                  title={`${system.names[planet.id] ?? planet.designation} · ${presetName({ name: planet.body_type })}${planet.generated ? '' : ` · ${t.systemList.notGenerated}`}`}
                ></span>
              {/each}
            </span>
            <small>{counts(system)}</small>
            <small title={new Date(system.modified_ms).toLocaleString(getLocale())}
              >{t.systemList.details({ star: system.star, seed: system.seed, moons: system.moons, ago: formatAgo(system.modified_ms) })}</small
            >
          </span>
        </button>
        {#if !selecting}
          <button class="delete" onclick={() => onDelete([system.id])} aria-label={t.worldList.deleteOne({ id: nameOf(system) })}>×</button>
        {/if}
      </li>
    {/each}
  </ul>
{/if}

<style>
  .strip {
    display: flex;
    align-items: center;
    gap: 4px;
    min-height: 14px;
  }
  .dot {
    display: inline-block;
    border-radius: 50%;
    flex: none;
  }
  .dot.stub {
    opacity: 0.45;
    outline: 1px dashed var(--muted);
    outline-offset: 1px;
  }
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
