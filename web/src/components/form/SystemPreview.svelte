<script lang="ts">
  // The preview of a whole system: its star, and the planets it will have in
  // the order they sit from the star. The form's seed is held, so Generate
  // draws exactly this.
  import { t } from '../../lib/i18n/index.svelte';
  import { classLabel } from '../../lib/i18n/names.svelte';
  import { starDotColour } from '../../lib/starForm';
  import { formatAge, formatAu, formatStar, formatUpTo } from '../../lib/format';
  import { formatMassWithNote } from '../../lib/massFormat';
  import type { System } from '../../lib/api/types/System';
  import Glossed from '../Glossed.svelte';
  import StarDot from './StarDot.svelte';

  /** The archetypes asked that set the system's age (see archetypeAge.ts), named on the star line. */
  let { system, ageSources = [] }: { system: System; ageSources?: string[] } = $props();

  const info = $derived(system.star.info);
  const planets = $derived(system.bodies.filter((b) => b.kind !== 'moon'));
  const moonsOf = (id: string) => system.bodies.filter((b) => b.parent === id).length;
  const text = $derived(t.form.preview.system);
</script>

<div class="system">
  <section aria-label={text.star}>
    <h4>{text.star}</h4>
    <p class="star">
      <StarDot colour={starDotColour(system.star.type)} label={system.star.type} />
      <strong>{formatStar(info)}</strong>
    </p>
    <p class="muted">{text.starLine({ age: formatAge(info.age_gyr), metallicity: formatUpTo(system.star.metallicity, 2) })}{ageSources.length > 0 ? ` ${text.ageSetBy({ names: ageSources.join(', ') })}` : ''}</p>
  </section>
  <section aria-label={text.planets}>
    <h4>{text.planets}</h4>
    {#if planets.length === 0}
      <p class="muted">{text.none}</p>
    {:else}
      <table>
        <thead>
          <tr>
            <th scope="col">{text.name}</th>
            <th scope="col">{text.class}</th>
            <th scope="col">{text.mass}</th>
            <th scope="col">{text.orbit}</th>
            <th scope="col">{text.moons}</th>
          </tr>
        </thead>
        <tbody>
          {#each planets as body (body.id)}
            <tr>
              <th scope="row">{body.designation}</th>
              <td>{classLabel(body.type, body.model)}</td>
              <td><Glossed text={formatMassWithNote(body.physical.mass_earth)} /></td>
              <td>{formatAu(body.orbit.semi_major_axis_au)}</td>
              <td>{moonsOf(body.id)}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}
  </section>
</div>

<style>
  .system {
    display: grid;
    gap: 10px;
  }
  h4 {
    margin: 0 0 2px;
    font-size: 0.95rem;
  }
  p {
    margin: 0;
  }
  .star {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
  }
  .muted {
    color: var(--muted);
    font-size: 0.9rem;
  }
  table {
    border-collapse: collapse;
    width: 100%;
    font-size: 0.9rem;
    font-variant-numeric: tabular-nums;
  }
  th,
  td {
    padding: 2px 8px 2px 0;
    text-align: left;
    vertical-align: top;
  }
  thead th {
    color: var(--muted);
    font-weight: normal;
  }
  tbody th {
    font-weight: 600;
    white-space: nowrap;
  }
</style>
