<script lang="ts">
  // A system's card in the main menu. Full: an orbit strip with the star's
  // glow, the name (the designation under it when renamed), the star, one
  // chip per planet type, a line when a planet has liquid water on its
  // surface (`BodySummary.habitable`), and a footer with the moons, the age and the buttons. Compact:
  // a top-down orrery on the left, the same facts in less room.
  import { getLocale, t } from '../lib/i18n/index.svelte';
  import type { SystemSummary } from '../lib/api/types/SystemSummary';
  import { formatAgo } from '../lib/format';
  import { archetypeName } from '../lib/i18n/names.svelte';
  import { hasHabitableWorld, starLetter } from '../lib/menuCards';
  import { cssColour } from '../lib/orrery';
  import { systemName as nameOf } from '../lib/systemList';
  import OrbitStrip from './OrbitStrip.svelte';
  import RenameForm from './RenameForm.svelte';
  import SystemActions from './SystemActions.svelte';
  import SystemOrrery from './SystemOrrery.svelte';

  interface Props {
    system: SystemSummary;
    compact: boolean;
    selecting: boolean;
    chosen: boolean;
    renaming: boolean;
    dirty: boolean;
    onOpen: () => void;
    onChanged: (system: SystemSummary) => void;
    /** The pin was changed (defaults to `onChanged`). */
    onPinned?: ((system: SystemSummary) => void) | undefined;
    onRename: () => void;
    onRenameClose: () => void;
    onDelete: () => void;
    onToggle: () => void;
  }

  let {
    system,
    compact,
    selecting,
    chosen,
    renaming,
    dirty = $bindable(false),
    onOpen,
    onChanged,
    onPinned,
    onRename,
    onRenameClose,
    onDelete,
    onToggle,
  }: Props = $props();

  /** The compact card's orrery, px. */
  const ORRERY = 124;
  /** Chips shown in a compact card before "+N". */
  const MAX_CHIPS = 4;
  const name = $derived(nameOf(system));
  const renamed = $derived(name !== system.designation);
  const starColour = $derived(cssColour(system.star_light));
  const starKind = $derived(t.systemList.starKinds[starLetter(system.star) ?? 'unknown']);
  const habitable = $derived(hasHabitableWorld(system));
  /** One chip per archetype: its count and name, in the colour of the first planet of that class. */
  const chips = $derived(
    Object.entries(system.planets).map(([type, count]) => ({
      type,
      text: `${count} ${archetypeName({ name: type })}`,
      colour: cssColour(system.bodies.find((b) => b.kind === 'planet' && b.body_type === type)?.colour ?? [0.5, 0.5, 0.5]),
    })),
  );
  const ago = $derived(formatAgo(system.modified_ms));
  const when = $derived(new Date(system.modified_ms).toLocaleString(getLocale()));
  const meta = $derived(`${t.systemList.moonsCount({ count: system.moons })} · ${ago}`);
</script>

<article class="card" class:compact class:editing={renaming} title={t.systemList.seedTitle({ seed: system.seed })}>
  {#if renaming && !system.outdated}
    {#if compact}
      <SystemOrrery {system} width={ORRERY} height={ORRERY} radius={10} scale="log" />
    {:else}
      <OrbitStrip {system} />
    {/if}
    <div class="body"><RenameForm {system} {onChanged} onClose={onRenameClose} bind:dirty /></div>
  {:else}
    {#if system.outdated}
      <div class="open outdated">
        <span class="body">
          <span class="head">
            <strong class="name">{name}</strong>
            {#if renamed}<small>{system.designation}</small>{/if}
          </span>
          <span class="badge">{t.systemList.outdated}</span>
        </span>
      </div>
    {:else}
    <button class="open" aria-pressed={selecting ? chosen : undefined} onclick={() => (selecting ? onToggle() : onOpen())}>
      {#if compact}
        <SystemOrrery {system} width={ORRERY} height={ORRERY} radius={10} scale="log" />
      {:else}
        <OrbitStrip {system} />
      {/if}
      <span class="body">
        <span class="head">
          <strong class="name">{name}</strong>
          {#if renamed}<small>{system.designation}</small>{/if}
        </span>
        <span class="star"
          ><span class="dot" style:background={starColour} aria-hidden="true"></span><b>{system.star}</b> · {starKind}</span
        >
        <span class="chips">
          {#each compact ? chips.slice(0, MAX_CHIPS) : chips as chip (chip.type)}
            <span class="chip"><span class="dot small" style:background={chip.colour} aria-hidden="true"></span>{chip.text}</span>
          {/each}
          {#if compact && chips.length > MAX_CHIPS}
            <span class="more" title={chips.slice(MAX_CHIPS).map((c) => c.text).join(', ')}>+{chips.length - MAX_CHIPS}</span>
          {/if}
        </span>
        {#if habitable}
          <span class="habitable">● {compact ? t.systemList.habitableShort : t.systemList.habitableLine}</span>
        {/if}
      </span>
    </button>
    {/if}
    <span class="foot">
      <small class="meta" title={when}>{meta}</small>
      <SystemActions {system} {selecting} {chosen} {onChanged} {onPinned} {onRename} {onDelete} {onToggle} />
    </span>
  {/if}
</article>

<style>
  .card {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-width: 0;
    overflow: hidden;
    background: var(--frame-deco), var(--panel);
    clip-path: var(--frame-clip);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    box-shadow: var(--glow);
    color: var(--text);
  }
  .card:hover,
  .card:focus-within {
    border-color: var(--accent);
  }
  /* Pinned cards keep an accent border, selected ones an outline too, the one to compare a dashed border. */
  :global(li.pinned) > .card {
    border-color: var(--accent);
    box-shadow: var(--glow), inset 0 0 0 1px var(--accent);
  }
  :global(li.selected) > .card {
    border-color: var(--accent);
    outline: 1px solid var(--accent);
  }
  :global(li.compared) > .card {
    border-style: dashed;
    border-color: var(--accent);
  }
  .open {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-width: 0;
    padding: 0;
    text-align: left;
    background: none;
    border: 0;
    border-radius: 0;
    color: inherit;
  }
  .open:hover:not(:disabled) {
    border: 0;
  }
  .open:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: -2px;
  }
  .body {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 12px 14px 8px;
    min-width: 0;
    flex: 1;
  }
  .head {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .name {
    font-family: var(--font-display);
    font-size: 1.25em;
    line-height: 1.25;
    overflow-wrap: anywhere;
  }
  small {
    color: var(--muted);
    overflow-wrap: anywhere;
  }
  .star {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .dot {
    width: 10px;
    height: 10px;
    flex: none;
    border-radius: 50%;
  }
  .dot.small {
    width: 8px;
    height: 8px;
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .chip {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 2px 9px 2px 7px;
    border-radius: 999px;
    background: color-mix(in srgb, var(--text) 9%, transparent);
    font-size: 0.9em;
  }
  /* A meaning, not a theme colour: green in every theme. */
  .habitable {
    color: #4fc27c;
    font-size: 0.9em;
  }
  /* A meaning, not a theme colour: amber in every theme. */
  .badge {
    align-self: flex-start;
    padding: 2px 9px;
    border: 1px solid #d9a441;
    border-radius: 999px;
    color: #d9a441;
    font-size: 0.9em;
  }
  .foot {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    padding: 4px 6px 6px 14px;
    border-top: 1px solid var(--border);
  }

  /* Compact: the orrery on the left, the text beside it at the top, the footer in its own row. */
  .compact .open {
    flex: none;
    flex-direction: row;
    align-items: flex-start;
    gap: 12px;
    padding: 10px 12px 6px 10px;
  }
  .compact.editing {
    flex-direction: row;
    align-items: flex-start;
    gap: 12px;
    padding: 10px;
  }
  .compact :global(svg.orrery) {
    flex: none;
  }
  .compact .body {
    padding: 0;
    gap: 4px;
    flex: 1;
  }
  .compact .name {
    font-size: 1.1em;
  }
  .compact .chips {
    gap: 2px 10px;
    /* At most two lines: the rest is under "+N". */
    max-height: 3.4em;
    overflow: hidden;
  }
  .compact .chip {
    padding: 0;
    background: none;
  }
  .more {
    color: var(--muted);
    font-size: 0.9em;
  }
  .compact .foot {
    padding-top: 0;
    margin-top: auto;
  }
</style>
