<script lang="ts">
  // The main menu's systems, in sections (pinned, today, older), filtered
  // by star class and habitable world, searched and sorted, shown as cards,
  // compact cards or a table; renamed, pinned, selected to compare or delete
  // (docs/system-spec.md, *Interface*).
  import { getLocale, t } from '../lib/i18n/index.svelte';
  import type { SystemSummary } from '../lib/api/types/SystemSummary';
  import { formatAgo } from '../lib/format';
  import {
    NO_FILTERS,
    filterSystems,
    groupSections,
    hasHabitableWorld,
    loadView,
    presentClasses,
    msToMidnight,
    pruneChosen,
    starClass,
    starLetter,
    toggleCompact,
    type StarClass,
    type View,
  } from '../lib/menuCards';
  import { cssColour } from '../lib/orrery';
  import { loadSetting, saveSetting } from '../lib/settings';
  import { SORTS, sortSystems, systemName as nameOf, type Sort } from '../lib/systemList';
  import { tick } from 'svelte';
  import DisplayMenu from './DisplayMenu.svelte';
  import OrbitStrip from './OrbitStrip.svelte';
  import PinButton from './PinButton.svelte';
  import RenameForm from './RenameForm.svelte';
  import SystemActions from './SystemActions.svelte';
  import SystemCard from './SystemCard.svelte';

  interface Props {
    systems: SystemSummary[];
    selectedId: string | null;
    compareId: string | null;
    onSelect: (id: string) => void;
    onDelete: (ids: string[]) => void;
    onCompare: (a: string, b: string) => void;
    /** A system was renamed or (un)pinned: its new summary. */
    onChanged: (system: SystemSummary) => void;
    /** Ids of the systems shown, in their order (written by the list). */
    order?: string[];
  }

  let { systems, selectedId, compareId, onSelect, onDelete, onCompare, onChanged, order = $bindable([]) }: Props = $props();

  const saved = loadSetting('systemList', { sort: 'newest' as string });
  let sort = $state<Sort>(SORTS.includes(saved.sort as Sort) ? (saved.sort as Sort) : 'newest');
  let view = $state<View>(loadView());
  let query = $state('');
  let classes = $state<StarClass[]>([]);
  let habitableOnly = $state(false);
  let selecting = $state(false);
  let chosen = $state<string[]>([]);
  $effect(() => saveSetting('systemList', { sort, view }));

  // The filters offered: the star classes of the systems, and the habitable worlds.
  const classesPresent = $derived(presentClasses(systems));
  const anyHabitable = $derived(systems.some(hasHabitableWorld));
  // A class whose systems are all gone (deleted) no longer filters.
  const activeClasses = $derived(classes.filter((c) => classesPresent.includes(c)));
  const activeHabitable = $derived(habitableOnly && anyHabitable);
  /** A class chip's dot: the light of the first star of that class. */
  const classLight = $derived(
    Object.fromEntries(
      classesPresent.map((c) => [c, cssColour(systems.find((s) => starClass(s.star) === c)?.star_light ?? [1, 1, 1])]),
    ),
  );

  const shown = $derived(
    sortSystems(filterSystems(systems, query, { ...NO_FILTERS, classes: activeClasses, habitable: activeHabitable }), sort),
  );
  // "Today" follows the clock: refreshed at the next local midnight and when the tab is shown again.
  let now = $state(Date.now());
  $effect(() => {
    const refresh = () => (now = Date.now());
    let timer: ReturnType<typeof setTimeout>;
    const arm = () => {
      clearTimeout(timer);
      timer = setTimeout(() => {
        refresh();
        arm();
      }, msToMidnight(Date.now()) + 1000);
    };
    const visible = () => {
      if (document.visibilityState === 'visible') {
        refresh();
        arm();
      }
    };
    arm();
    document.addEventListener('visibilitychange', visible);
    return () => {
      clearTimeout(timer);
      document.removeEventListener('visibilitychange', visible);
    };
  });
  const sections = $derived(groupSections(shown, now));
  $effect(() => {
    order = sections.flatMap((g) => g.items.map((s) => s.id));
  });
  // What a filter, a chip or the search hides, or a deletion removes, is no longer selected:
  // Delete and Compare only act on systems that are shown.
  $effect(() => {
    const kept = pruneChosen(chosen, shown.map((s) => s.id));
    if (kept !== chosen) chosen = kept;
  });

  /** A pin moves its card to another section, which remounts it: put the focus back on its pin. */
  async function pinChanged(system: SystemSummary): Promise<void> {
    onChanged(system);
    await tick();
    document.querySelector<HTMLElement>(`[data-pin="${CSS.escape(system.id)}"]`)?.focus();
  }

  function toggleClass(c: StarClass): void {
    classes = classes.includes(c) ? classes.filter((x) => x !== c) : [...classes, c];
  }

  // Renaming a system (`RenameForm`): a single form is open at a time.
  let renaming = $state<string | null>(null);
  let dirty = $state(false);
  /** Back to the pencil of a card once its form is closed. */
  async function focusPencil(id: string): Promise<void> {
    await tick();
    document.querySelector<HTMLElement>(`[data-rename="${CSS.escape(id)}"]`)?.focus();
  }
  function startRenaming(id: string): void {
    // A form with text typed stays open (and gets the focus) rather than
    // being dropped by another card's pencil.
    if (renaming !== null && renaming !== id && dirty) {
      document.querySelector<HTMLElement>('.editing input')?.focus();
      return;
    }
    renaming = id;
  }
  function stopRenaming(): void {
    const id = renaming;
    renaming = null;
    dirty = false;
    if (id) void focusPencil(id);
  }

  function toggle(id: string): void {
    chosen = chosen.includes(id) ? chosen.filter((c) => c !== id) : [...chosen, id];
  }

  function stopSelecting(): void {
    selecting = false;
    chosen = [];
  }

  const open = (id: string) => (selecting ? toggle(id) : onSelect(id));
  const starKind = (s: SystemSummary) => t.systemList.starKinds[starLetter(s.star) ?? 'unknown'];
  const planetCount = (s: SystemSummary) => s.bodies.filter((b) => b.kind === 'planet').length;
</script>

{#if systems.length === 0}
  <p class="empty">{t.systemList.empty}</p>
{:else}
  <div class="toolbar">
    <input type="search" placeholder={t.systemList.search} bind:value={query} aria-label={t.systemList.searchLabel} />
    {#if view !== 'list'}
      <button class="compact-toggle" onclick={() => (view = toggleCompact(view))} aria-pressed={view === 'compact'} title={t.systemList.compactToggle}
        >{t.systemList.compactToggle}</button
      >
    {/if}
    <DisplayMenu {view} onPick={(next) => (view = next)} />
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

  {#if classesPresent.length > 0 || anyHabitable}
    <div class="filters" role="group" aria-label={t.systemList.filtersLabel}>
      {#if classesPresent.length > 0}
        <button class="chip" aria-pressed={activeClasses.length === 0} onclick={() => (classes = [])}>
          <span class="dot all" aria-hidden="true"></span>{t.systemList.allStars}
        </button>
        {#each classesPresent as c (c)}
          <button class="chip" aria-pressed={activeClasses.includes(c)} onclick={() => toggleClass(c)}>
            <span class="dot" style:background={classLight[c]} aria-hidden="true"></span>{t.systemList.starClasses[c]}
          </button>
        {/each}
      {/if}
      {#if anyHabitable}
        <button class="chip" aria-pressed={activeHabitable} onclick={() => (habitableOnly = !habitableOnly)}>
          <span class="dot habitable" aria-hidden="true"></span>{t.systemList.habitableFilter}
        </button>
      {/if}
    </div>
  {/if}

  {#if shown.length === 0}
    <p class="empty">{t.systemList.noMatch}</p>
  {/if}
  {#each sections as group (group.section)}
    {@const title = t.systemList.sections[group.section]}
    <section class="group" aria-label={title}>
      <h3>{title} <span class="count">· {group.items.length}</span></h3>
      {#if view === 'list'}
        <div class="scroll">
          <table aria-label={t.systemList.listLabel({ section: title })}>
            <thead>
              <tr>
                <th class="narrow"><span class="sr">{t.systemList.columns.pin}</span></th>
                <th>{t.systemList.columns.name}</th>
                <th>{t.systemList.columns.star}</th>
                <th>{t.systemList.columns.planets}</th>
                <th>{t.systemList.columns.moons}</th>
                <th>{t.systemList.columns.created}</th>
                <th><span class="sr">{t.systemList.columns.actions}</span></th>
              </tr>
            </thead>
            <tbody>
              {#each group.items as system (system.id)}
                <tr
                  class:selected={system.id === selectedId || chosen.includes(system.id)}
                  class:compared={system.id === compareId}
                  class:pinned={system.pinned}
                  class:editing={renaming === system.id}
                  title={t.systemList.seedTitle({ seed: system.seed })}
                >
                  <td class="narrow"><span class="pin-cell"><PinButton {system} onChanged={pinChanged} /></span></td>
                  {#if renaming === system.id}
                    <td colspan="6"><RenameForm {system} {onChanged} onClose={stopRenaming} bind:dirty /></td>
                  {:else}
                    <td>
                      <button class="name-button" aria-pressed={selecting ? chosen.includes(system.id) : undefined} onclick={() => open(system.id)}>
                        <strong>{nameOf(system)}</strong>
                        <small>{nameOf(system) === system.designation ? '' : `${system.designation} · `}{t.systemList.planetsCount({ count: planetCount(system) })}</small>
                      </button>
                    </td>
                    <td>
                      <span class="star">
                        <span class="dot" style:background={cssColour(system.star_light)} aria-hidden="true"></span>
                        <span><strong>{system.star}</strong><small>{starKind(system)}</small></span>
                      </span>
                    </td>
                    <td class="strip-cell"><OrbitStrip {system} variant="row" /></td>
                    <td>{t.systemList.moonsCount({ count: system.moons })}</td>
                    <td class="muted" title={new Date(system.modified_ms).toLocaleString(getLocale())}>{formatAgo(system.modified_ms)}</td>
                    <td class="actions-cell">
                      <SystemActions
                        {system}
                        showPin={false}
                        {selecting}
                        chosen={chosen.includes(system.id)}
                        {onChanged}
                        onPinned={pinChanged}
                        onRename={() => startRenaming(system.id)}
                        onDelete={() => onDelete([system.id])}
                        onToggle={() => toggle(system.id)}
                      />
                    </td>
                  {/if}
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      {:else}
        <ul class="grid" class:compact={view === 'compact'}>
          {#each group.items as system (system.id)}
            <li class:selected={system.id === selectedId || chosen.includes(system.id)} class:compared={system.id === compareId} class:pinned={system.pinned}>
              <SystemCard
                {system}
                compact={view === 'compact'}
                {selecting}
                chosen={chosen.includes(system.id)}
                renaming={renaming === system.id}
                bind:dirty
                onOpen={() => open(system.id)}
                {onChanged}
                onPinned={pinChanged}
                onRename={() => startRenaming(system.id)}
                onRenameClose={stopRenaming}
                onDelete={() => onDelete([system.id])}
                onToggle={() => toggle(system.id)}
              />
            </li>
          {/each}
        </ul>
      {/if}
    </section>
  {/each}
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
  .compact-toggle[aria-pressed='true'],
  .chip[aria-pressed='true'] {
    background: color-mix(in srgb, var(--accent) 25%, var(--control-bg));
    border-color: var(--accent);
  }
  .selection {
    display: flex;
    flex-wrap: wrap;
    gap: 6px 8px;
    align-items: center;
  }
  .filters {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    margin-bottom: 16px;
  }
  .chip {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    border-radius: 999px;
    padding: 4px 14px;
  }
  .dot {
    width: 9px;
    height: 9px;
    flex: none;
    border-radius: 50%;
  }
  .dot.all {
    background: var(--muted);
  }
  /* A meaning, not a theme colour: green in every theme. */
  .dot.habitable {
    background: #4fc27c;
  }
  .group {
    margin-bottom: 22px;
  }
  .group h3 {
    margin: 0 0 10px;
    font-size: 0.9em;
    font-weight: 600;
    letter-spacing: 0.07em;
    text-transform: uppercase;
    color: var(--muted);
  }
  .count {
    opacity: 0.7;
  }
  .grid {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(min(100%, 270px), 1fr));
    gap: 14px;
    /* Every card of a row as tall as the tallest. */
    grid-auto-rows: 1fr;
  }
  .grid.compact {
    grid-template-columns: repeat(auto-fill, minmax(min(100%, 340px), 1fr));
    gap: 12px;
  }
  li {
    min-width: 0;
    display: grid;
  }

  /* The table: scrolls sideways on narrow screens. */
  .scroll {
    overflow-x: auto;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--panel);
  }
  table {
    width: 100%;
    min-width: 820px;
    border-collapse: collapse;
  }
  th {
    padding: 8px 12px;
    text-align: left;
    font-weight: 600;
    font-size: 0.85em;
    color: var(--muted);
    border-bottom: 1px solid var(--border);
  }
  td {
    padding: 6px 12px;
    vertical-align: middle;
    border-bottom: 1px solid var(--border);
  }
  tbody tr:last-child td {
    border-bottom: 0;
  }
  th.narrow,
  td.narrow {
    width: 44px;
    padding-right: 0;
  }
  .strip-cell {
    width: 32%;
  }
  .actions-cell {
    text-align: right;
    white-space: nowrap;
  }
  .muted,
  small {
    color: var(--muted);
  }
  td small,
  .name-button small {
    display: block;
    overflow-wrap: anywhere;
  }
  .name-button {
    display: grid;
    gap: 2px;
    padding: 0;
    text-align: left;
    background: none;
    border: 0;
    color: inherit;
  }
  .name-button:hover:not(:disabled) strong {
    color: var(--link);
  }
  .name-button strong {
    font-family: var(--font-display);
  }
  .star {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .sr {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
    white-space: nowrap;
  }
  tr.pinned td:first-child {
    box-shadow: inset 3px 0 0 var(--accent);
  }
  tr.selected td {
    background: color-mix(in srgb, var(--accent) 16%, transparent);
  }
  tr.compared td {
    border-bottom-style: dashed;
    border-bottom-color: var(--accent);
  }
  .pin-cell :global(button) {
    min-width: 32px;
    height: 32px;
    background: transparent;
    border-color: transparent;
    color: var(--muted);
  }
  .pin-cell :global(button.on) {
    color: var(--accent);
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
  @media (hover: none), (pointer: coarse) {
    .pin-cell :global(button) {
      min-width: 40px;
      height: 40px;
    }
    .chip,
    .compact-toggle {
      min-height: 40px;
    }
  }
</style>
