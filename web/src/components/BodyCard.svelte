<script lang="ts">
  // The property card of the body of a system shown (docs/system-spec.md,
  // *Interface*): a summary of the system (its star and a strip of its
  // planets, which picks one), then the body's facts as the draw fixed
  // them, its given name (renamed here), and whether its maps exist. The
  // world card follows it for a generated body, with its maps' details.
  import { t } from '../lib/i18n/index.svelte';
  import { presetName } from '../lib/i18n/names.svelte';
  import { api } from '../lib/api/client';
  import type { Body } from '../lib/api/types/Body';
  import type { SystemSummary } from '../lib/api/types/SystemSummary';
  import { errorText } from '../lib/i18n/errors';
  import { formatAu, formatInt, formatUpTo, formatYear, withUnit } from '../lib/format';
  import { toasts } from '../lib/toasts.svelte';
  import type { LoadedWorld } from '../lib/world.svelte';
  import { MAX_NAME_CHARS } from '../lib/names';
  import { jobs } from '../lib/jobs.svelte';
  import { dustFedBy, dustOf, ringPart, ringsOf } from '../lib/ringMoons';

  interface Props {
    world: LoadedWorld;
    /** Called once names are saved, with the system's summary. */
    onRenamed: (summary: SystemSummary) => void;
    /** Queues the generation of a body and its neighbourhood. */
    onGenerate: (system: string, body: string) => void;
  }

  let { world, onRenamed, onGenerate }: Props = $props();

  const system = $derived(world.system);
  const names = $derived(world.systemNames);
  const body = $derived(world.shownSystemBody);
  const star = $derived(system?.star ?? null);
  /** Whether a job of this system is queued or running: its bodies are on their way. */
  const busy = $derived(jobs.active.some((j) => j.worldId === world.systemId));
  const planets = $derived(system?.bodies.filter((b) => b.kind === 'planet') ?? []);
  const nameOf = (id: string, designation: string) => names[id] ?? designation;
  const starName = $derived(star ? nameOf('star', star.designation) : '');
  /** The planet the body belongs to (itself for a planet), and its moons. */
  const planet = $derived(body ? (system?.bodies.find((b) => b.id === (body.parent ?? body.id)) ?? null) : null);
  const family = $derived(planet ? (system?.bodies.filter((b) => b.parent === planet.id) ?? []) : []);

  /** A linear colour as CSS. */
  function css([r, g, b]: [number, number, number]): string {
    const srgb = (c: number) => Math.round(255 * Math.min(1, Math.max(0, c)) ** (1 / 2.2));
    return `rgb(${srgb(r)} ${srgb(g)} ${srgb(b)})`;
  }
  const GIANTS = new Set(['gas-giant', 'ice-giant', 'hot-jupiter']);
  const giant = (b: Body) => GIANTS.has(b.type);
  /** A mass in Earth masses, or Jupiter masses for a giant. */
  function mass(b: Body): string {
    const earth = b.physical.mass_earth;
    if (earth >= 30) return withUnit(formatUpTo(earth / 317.8, 2), 'MJ');
    if (earth < 0.01) return withUnit(formatUpTo(earth * 81.3, 2), t.card.lunarMasses);
    return withUnit(formatUpTo(earth, 2), 'M⊕');
  }
  /** A distance from the parent: AU round the star, km round a planet. */
  const distance = (b: Body) =>
    b.parent ? withUnit(formatInt(b.orbit.semi_major_axis_km), 'km') : formatAu(b.orbit.semi_major_axis_au);
  const rings = $derived(body ? ringsOf(body) : null);
  const part = $derived(body ? ringPart(body, planet) : null);
  const dust = $derived(body ? dustOf(body) : []);
  const fed = $derived(body ? dustFedBy(body, planet) : null);
  const hours = (h: number) => (h < 48 ? withUnit(formatUpTo(h, 1), 'h') : formatYear(h / 24));

  // Renaming the body shown: empty goes back to its designation.
  /** The body being renamed (`star` for the star), or null. */
  let renaming = $state<string | null>(null);
  let saving = $state(false);
  let field = $state('');
  // Another body or system closes the editor.
  $effect(() => {
    void [world.systemId, world.systemBody];
    renaming = null;
  });
  function startRenaming(id: string): void {
    field = names[id] ?? '';
    renaming = id;
  }
  async function saveName(event: SubmitEvent, id: string, designation: string): Promise<void> {
    event.preventDefault();
    const systemId = world.systemId;
    if (!systemId) return;
    const next = { ...names };
    const value = field.trim();
    if (value === '' || value === designation) delete next[id];
    else next[id] = value;
    saving = true;
    try {
      const summary = await api.renameSystem(systemId, next);
      renaming = null;
      world.loadSystem(systemId);
      onRenamed(summary);
      toasts.show(t.worldInfo.renamed({ name: value === '' ? designation : value }));
    } catch (e) {
      toasts.error(t.worldInfo.renameFailed({ error: errorText(e) }));
    } finally {
      saving = false;
    }
  }
</script>

{#snippet renameForm(id: string, designation: string)}
  <form class="rename" onsubmit={(e) => saveName(e, id, designation)}>
    <input bind:value={field} placeholder={designation} maxlength={MAX_NAME_CHARS} aria-label={t.worldInfo.rename} />
    <button type="submit" disabled={saving}>{saving ? t.worldInfo.sending : t.worldInfo.renameSave}</button>
    <button type="button" onclick={() => (renaming = null)}>{t.worldInfo.renameCancel}</button>
  </form>
{/snippet}

{#if system && star}
  <section class="card system" aria-label={t.card.system}>
    <header>
      {#if renaming === 'star'}
        {@render renameForm('star', star.designation)}
      {:else}
        <strong>{starName}</strong>
        <button class="link" onclick={() => startRenaming('star')} title={t.worldInfo.renameHelp}>{t.worldInfo.rename}</button>
      {/if}
      <small>{starName === star.designation ? '' : `${star.designation} · `}{star.type} · {t.card.starFacts({
          temperature: withUnit(formatInt(Math.round(star.info.teff_k / 10) * 10), 'K'),
          luminosity: withUnit(formatUpTo(star.info.luminosity, star.info.luminosity < 0.01 ? 4 : 2), 'L☉'),
        })}</small>
    </header>
    <div class="strip" role="group" aria-label={t.card.planets}>
      {#each planets as p (p.id)}
        <button
          class="dot"
          class:current={planet?.id === p.id}
          class:stub={!p.generated}
          style:background={css(p.preview.colours[0] ?? [0.5, 0.5, 0.5])}
          style:width={`${giant(p) ? 16 : 11}px`}
          style:height={`${giant(p) ? 16 : 11}px`}
          title={`${nameOf(p.id, p.designation)} · ${presetName({ name: p.type })} · ${formatAu(p.orbit.semi_major_axis_au)}`}
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
      })}</small
    >
  </section>
{/if}

{#if system && body}
  {@const title = nameOf(body.id, body.designation)}
  <section class="card body" aria-label={t.card.body}>
    <header>
      {#if renaming === body.id}
        {@render renameForm(body.id, body.designation)}
      {:else}
        <strong>{title}</strong>
        <button class="link" onclick={() => startRenaming(body.id)} title={t.worldInfo.renameHelp}>{t.worldInfo.rename}</button>
      {/if}
      <small
        >{title === body.designation ? '' : `${body.designation} · `}{presetName({ name: body.type })} · {body.kind === 'moon'
          ? t.card.moonOf({ planet: planet ? nameOf(planet.id, planet.designation) : '' })
          : t.card.planetOf({ star: starName })}</small
      >
    </header>
    <dl>
      <dt>{t.card.orbit}</dt>
      <dd>
        {t.card.orbitValue({
          distance: distance(body),
          eccentricity: formatUpTo(body.orbit.eccentricity, 3),
          period: formatYear(body.orbit.period_days),
        })}
      </dd>
      <dt>{t.card.size}</dt>
      <dd>
        {withUnit(formatInt(body.physical.radius_km), 'km')}{#if body.physical.axes_km}
          (<span class="unit-help" title={t.card.axesHelp}>{t.card.axes({
            a: withUnit(formatInt(2 * body.physical.axes_km[0]), 'km'),
            b: formatInt(2 * body.physical.axes_km[1]),
            c: formatInt(2 * body.physical.axes_km[2]),
          })}</span>){/if} · {mass(body)} · {withUnit(formatUpTo(body.physical.density, 2), 'g/cm³')}
        · {withUnit(formatUpTo(body.physical.gravity, 2), 'g')}
      </dd>
      <dt>{t.card.spin}</dt>
      <dd>
        {body.spin.locked
          ? t.card.locked
          : body.spin.spin_orbit_3_2
            ? t.card.resonant({ day: hours(body.spin.period_hours) })
            : hours(body.spin.period_hours)} · {t.card.tilt({
          tilt: `${formatUpTo(body.spin.axial_tilt_deg, 1)}°`,
        })}
      </dd>
      <dt>{t.card.light}</dt>
      <dd>
        {t.card.lightValue({
          flux: formatUpTo(body.surroundings.flux_earth, body.surroundings.flux_earth < 0.1 ? 3 : 2),
          temperature: withUnit(formatInt(Math.round(body.surroundings.equilibrium_k - 273.15)), '°C'),
        })}
      </dd>
      {#if rings}
        <dt>{t.card.rings}</dt>
        <dd>
          {t.card.ringsValue({
            kind: rings.bright ? 'bright' : 'faint',
            inner: formatUpTo(rings.inner, 2),
            outer: formatUpTo(rings.outer, 2),
            gaps: rings.gaps.length,
          })}
        </dd>
      {/if}
      {#if dust.length > 0}
        <dt>{t.card.dustRings}</dt>
        <dd>
          {#each dust as ring, k (k)}
            {@const moon = family.find((m) => m.id === ring.source)}
            <span class="dust"
              >{t.card.dustRing({
                kind: ring.kind,
                moon: moon ? nameOf(moon.id, moon.designation) : ring.source,
                inner: formatUpTo(ring.inner, 1),
                outer: formatUpTo(ring.outer, 1),
              })}</span
            >
          {/each}
        </dd>
      {/if}
      {#if fed}
        <dt>{t.card.dustSource}</dt>
        <dd>{t.card.dustFed({ kind: fed.kind })}</dd>
      {/if}
      {#if part}
        <dt>{t.card.ringRole}</dt>
        <dd>{t.card.ringRoles[part.role]({ gap: withUnit(formatInt(part.gapKm ?? 0), 'km') })}</dd>
      {/if}
      {#if body.kind === 'planet' && family.length > 0}
        <dt>{t.card.moons}</dt>
        <dd class="moons">
          {#each family as moon (moon.id)}
            <button class="link" onclick={() => world.showSystemBody(moon.id)}>{nameOf(moon.id, moon.designation)}</button>
          {/each}
        </dd>
      {/if}
    </dl>
    {#if !body.generated}
      <p class="muted">{busy ? t.card.generating : t.card.notGenerated}</p>
      {#if !busy}
        <button class="primary" onclick={() => world.systemId && onGenerate(world.systemId, body.id)}>{t.card.generate}</button>
      {/if}
    {/if}
  </section>
{/if}

<style>
  .dust {
    display: block;
  }
  .card {
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 10px 12px;
    margin-bottom: 10px;
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
  dl {
    display: grid;
    grid-template-columns: max-content minmax(0, 1fr);
    gap: 2px 10px;
    margin: 0;
  }
  dt {
    color: var(--muted);
  }
  dd {
    margin: 0;
  }
  .moons {
    display: flex;
    flex-wrap: wrap;
    gap: 2px 8px;
  }
  .link {
    border: none;
    background: none;
    padding: 0;
    color: var(--link);
  }
  .muted {
    color: var(--muted);
    margin: 0;
  }
  .rename {
    display: flex;
    gap: 4px;
    flex: 1;
  }
  .rename input {
    flex: 1;
    min-width: 0;
  }
</style>
