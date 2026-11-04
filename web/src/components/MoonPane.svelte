<script lang="ts">
  // The panel over a moon shown in the viewer: what the moon is, its orbit
  // around its planet, and what bounds the planet's moons (see mapgen's
  // moons module). The moon itself is drawn by the planet view, as a world.
  import type { Moon } from '../lib/api/types/Moon';
  import type { MoonSystem } from '../lib/api/types/MoonSystem';
  import { formatKm, formatNumber, formatShare, formatUpTo, withUnit } from '../lib/format';
  import { t } from '../lib/i18n/index.svelte';

  interface Props {
    moon: Moon;
    system: MoonSystem;
    /** The world the moon orbits, for the heading. */
    world: string;
    /** The heading instead, when the moon has a name or a designation (`MGS 48213 c I`). */
    title?: string | null;
  }

  let { moon, system, world, title = null }: Props = $props();

  /** A size in km with three significant digits (small moons keep their decimals). */
  const km = (value: number) => (value < 100 ? withUnit(formatUpTo(value, 1), 'km') : formatKm(value));
  const days = (value: number) =>
    value < 2
      ? t.moons.periodHours({ hours: formatUpTo(value * 24, 1) })
      : t.moons.periodValue({ days: formatUpTo(value, value < 10 ? 2 : 1) });
  const size = $derived(
    moon.round
      ? t.moons.roundSize({ radius: km(moon.radius_km) })
      : t.moons.irregularSize({
          radius: km(moon.radius_km),
          axes: moon.axes_km.map((a) => formatUpTo(2 * a, 1)).join(' × ') + `${t.format.unitSpace}km`,
        }),
  );
  const degrees = (value: number) => `${formatUpTo(value, value < 10 ? 2 : 0)}°`;
  const inclination = $derived(
    moon.inclination_deg > 90
      ? t.moons.retrograde({ degrees: degrees(moon.inclination_deg) })
      : t.moons.prograde({ degrees: degrees(moon.inclination_deg) }),
  );
  /** The barycentre only matters for a moon heavy enough to move its planet. */
  const barycentre = $derived(
    moon.migration === 'locked'
      ? system.binary
        ? t.moons.barycentreOutside({ km: km(moon.barycentre_km) })
        : t.moons.barycentreInside({ km: km(moon.barycentre_km) })
      : null,
  );
</script>

<details class="moon-panel" role="region" aria-label={t.moons.title} open>
  <summary><strong>{title ?? t.moons.heading({ name: moon.name, world })}</strong></summary>
  <h4>{t.moons.moonSection}</h4>
  <dl>
    <dt title={t.moons.originHelp}>{t.moons.origin}</dt>
    <dd>{t.moons.origins[moon.origin]}</dd>
    <dt>{t.moons.composition}</dt>
    <dd>{t.moons.compositions[moon.composition]}</dd>
    <dt title={t.moons.sizeHelp}>{t.moons.size}</dt>
    <dd>{size}</dd>
    <dt title={t.moons.massHelp}>{t.moons.mass}</dt>
    <dd>{formatShare(moon.mass_ratio)}</dd>
    <dt>{t.moons.density}</dt>
    <dd>{withUnit(formatNumber(moon.density, 1), 'g/cm³')}</dd>
    <dt title={t.moons.gravityHelp}>{t.moons.gravity}</dt>
    <dd>{withUnit(formatUpTo(moon.gravity, moon.gravity < 0.01 ? 4 : 3), 'g')}</dd>
    <dt title={t.moons.lockedHelp}>{t.moons.locked}</dt>
    <dd>{moon.tidally_locked ? t.moons.lockedYes : t.moons.lockedNo}</dd>
  </dl>
  <h4>{t.moons.orbitSection}</h4>
  <dl>
    <dt title={t.moons.distanceHelp}>{t.moons.distance}</dt>
    <dd>{t.moons.distanceValue({ km: km(moon.semi_major_axis_km), radii: formatUpTo(moon.semi_major_axis_radii, 1) })}</dd>
    <dt>{t.moons.eccentricity}</dt>
    <dd>{formatNumber(moon.eccentricity, 3)}</dd>
    <dt title={t.moons.inclinationHelp}>{t.moons.inclination}</dt>
    <dd>{inclination}</dd>
    <dt title={t.moons.periodHelp}>{t.moons.period}</dt>
    <dd>{days(moon.period_days)}</dd>
    <dt title={t.moons.migrationHelp}>{t.moons.migration}</dt>
    <dd>{t.moons.migrations[moon.migration]}</dd>
    <dt title={t.moons.apparentHelp}>{t.moons.apparent}</dt>
    <dd>{degrees(moon.angular_diameter_deg)}</dd>
    {#if barycentre}
      <dt title={t.moons.barycentreHelp}>{t.moons.barycentre}</dt>
      <dd>{barycentre}</dd>
    {/if}
    <dt title={t.moons.dayHelp}>{t.moons.day}</dt>
    <dd>{t.moons.periodHours({ hours: formatUpTo(system.planet_day_hours, 1) })}</dd>
    <dt title={t.moons.synchronousHelp}>{t.moons.synchronous}</dt>
    <dd>{km(system.synchronous_orbit_km)}</dd>
    <dt title={t.moons.hillHelp}>{t.moons.hill}</dt>
    <dd>{km(system.hill_radius_km)}</dd>
  </dl>
  <small class="muted">{t.moons.schematic}</small>
</details>

<style>
  .moon-panel summary {
    cursor: pointer;
  }
  .moon-panel[open] summary {
    margin-bottom: 4px;
  }
  .moon-panel {
    position: absolute;
    left: 12px;
    bottom: 12px;
    max-width: min(380px, calc(100% - 24px));
    /* Leave room for the toolbar above; scroll rather than cover it. */
    max-height: calc(100% - 72px);
    overflow-y: auto;
    display: grid;
    gap: 6px;
    padding: 10px 12px;
    background: var(--panel);
    border: 1px solid var(--border);
    border-radius: 8px;
    font-size: 0.85rem;
  }
  h4 {
    margin: 4px 0 0;
    font-size: 0.75rem;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--muted);
  }
  dl {
    display: grid;
    grid-template-columns: 9.5em 1fr;
    gap: 2px 10px;
    margin: 0;
  }
  dt {
    color: var(--muted);
  }
  dt[title] {
    cursor: help;
  }
  dd {
    margin: 0;
  }
  .muted {
    color: var(--muted);
  }
</style>
