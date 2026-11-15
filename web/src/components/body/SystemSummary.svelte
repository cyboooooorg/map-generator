<script lang="ts">
  // The summary of the system shown (docs/system-spec.md, *Interface*): its
  // star, a strip of its planets that picks one, and its counts. The only
  // place the system's age is shown.
  import { t } from '../../lib/i18n/index.svelte';
  import { classLabel } from '../../lib/i18n/names.svelte';
  import { isGiantType } from '../../lib/planetClass';
  import type { SystemSummary } from '../../lib/api/types/SystemSummary';
  import { formatAge, formatAu, formatInt, formatUpTo, withUnit } from '../../lib/format';
  import type { LoadedWorld } from '../../lib/world.svelte';
  import NameEditor from './NameEditor.svelte';

  interface Props {
    world: LoadedWorld;
    /** Called once the star is renamed, with the system's summary. */
    onRenamed: (summary: SystemSummary) => void;
  }

  let { world, onRenamed }: Props = $props();

  const system = $derived(world.system);
  const names = $derived(world.systemNames);
  const star = $derived(system?.star ?? null);
  const body = $derived(world.shownSystemBody);
  const planets = $derived(system?.bodies.filter((b) => b.kind === 'planet') ?? []);
  const nameOf = (id: string, designation: string) => names[id] ?? designation;
  const starName = $derived(star ? nameOf('star', star.designation) : '');
  /** The planet the body belongs to (itself for a planet). */
  const planetId = $derived(body ? (body.parent ?? body.id) : null);

  /** A linear colour as CSS. */
  function css([r, g, b]: [number, number, number]): string {
    const srgb = (c: number) => Math.round(255 * Math.min(1, Math.max(0, c)) ** (1 / 2.2));
    return `rgb(${srgb(r)} ${srgb(g)} ${srgb(b)})`;
  }
</script>

{#if system && star}
  <section class="system" aria-label={t.card.system}>
    <header>
      <NameEditor {world} id="star" designation={star.designation} name={starName} {onRenamed} />
      <small>{starName === star.designation ? '' : `${star.designation} · `}{star.type} · {t.card.starFacts({
          temperature: withUnit(formatInt(Math.round(star.info.teff_k / 10) * 10), 'K'),
          luminosity: withUnit(formatUpTo(star.info.luminosity, star.info.luminosity < 0.01 ? 4 : 2), 'L☉'),
        })}</small>
    </header>
    <div class="strip" role="group" aria-label={t.card.planets}>
      {#each planets as p (p.id)}
        {@const size = isGiantType(p.type) ? 16 : 11}
        <button type="button"
          class="dot"
          class:current={planetId === p.id}
          class:stub={!p.generated}
          style:background={css(p.preview.colours[0] ?? [0.5, 0.5, 0.5])}
          style:width={`${size}px`}
          style:height={`${size}px`}
          title={`${nameOf(p.id, p.designation)} · ${classLabel(p.type, p.model)} · ${formatAu(p.orbit.semi_major_axis_au)}`}
          aria-label={nameOf(p.id, p.designation)}
          onclick={() => world.showSystemBody(p.id)}
        ></button>
      {/each}
    </div>
    <small class="muted"
      >{t.card.systemFacts({
        planets: planets.length,
        moons: system.bodies.length - planets.length,
        ice: formatAu(star.ice_line_au),
        metallicity: `${star.metallicity >= 0 ? '+' : ''}${formatUpTo(star.metallicity, 2)}`,
        age: formatAge(star.info.age_gyr),
      })}</small
    >
  </section>
{/if}

<style>
  .system {
    display: grid;
    gap: 6px;
  }
  header {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 2px 8px;
  }
  header small {
    flex-basis: 100%;
    color: var(--muted);
  }
  .strip {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-wrap: wrap;
  }
  .dot {
    border: none;
    border-radius: 50%;
    padding: 0;
    cursor: pointer;
  }
  .dot.stub {
    opacity: 0.5;
  }
  .dot.current {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }
  .muted {
    color: var(--muted);
  }
</style>
