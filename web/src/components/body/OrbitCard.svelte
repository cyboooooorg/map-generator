<script lang="ts">
  // Orbit and light: where the body goes round, how it spins, what light it
  // gets and how large its star (or, for a moon, its host) looks.
  import './facts.css';
  import { t } from '../../lib/i18n/index.svelte';
  import { formatAu, formatInt, formatUpTo, formatYear, withUnit } from '../../lib/format';
  import { unitTooltip } from '../../lib/units';
  import { resonanceRatio, resonanceTurns } from '../../lib/spinResonance';
  import type { LoadedWorld } from '../../lib/world.svelte';

  let { world }: { world: LoadedWorld } = $props();

  const body = $derived(world.shownSystemBody);
  const meta = $derived(world.meta);
  const star = $derived(meta?.star ?? null);
  const host = $derived(meta?.host ?? null);
  const hours = (h: number) => (h < 48 ? withUnit(formatUpTo(h, 1), 'h') : formatYear(h / 24));
</script>

{#if body}
  <dl class="facts">
    <dt>{t.card.orbit}</dt>
    <dd>
      {t.card.orbitValue({
        distance: body.parent ? withUnit(formatInt(body.orbit.semi_major_axis_km), 'km') : formatAu(body.orbit.semi_major_axis_au),
        eccentricity: formatUpTo(body.orbit.eccentricity, 3),
        period: formatYear(body.orbit.period_days),
      })}
    </dd>
    <dt>{t.card.spin}</dt>
    <dd>
      {body.spin.locked
        ? t.card.locked
        : resonanceTurns(body.spin) > 0
          ? t.card.resonant({ day: hours(body.spin.period_hours), ratio: resonanceRatio(resonanceTurns(body.spin)) })
          : hours(body.spin.period_hours)} · {t.card.tilt({ tilt: `${formatUpTo(body.spin.axial_tilt_deg, 1)}°` })}
    </dd>
    <dt>{t.card.light}</dt>
    <dd>
      {t.card.lightValue({
        flux: formatUpTo(body.surroundings.flux_earth, body.surroundings.flux_earth < 0.1 ? 3 : 2),
        temperature: withUnit(formatInt(Math.round(body.surroundings.equilibrium_k - 273.15)), '°C'),
      })}
    </dd>
    {#if star}
      <dt>{t.worldInfo.starSize}</dt>
      <dd class="unit-help" title={unitTooltip('L☉')}>
        {t.worldInfo.starSizeValue({ size: `${formatUpTo(star.angular_diameter_deg, 2)}°`, sun: `${formatUpTo(0.53, 2)}°` })}
      </dd>
    {/if}
    {#if host}
      <dt title={t.worldInfo.hostDayHelp}>{t.worldInfo.hostDay}</dt>
      <dd>{formatYear(host.period_days)}</dd>
      <dt>{t.cards.sizeInSky}</dt>
      <dd>{t.worldInfo.hostSky({ size: `${formatUpTo(host.angular_diameter_deg, 1)}°` })}</dd>
      <dt title={t.worldInfo.tidalHelp}>{t.worldInfo.tidal}</dt>
      <dd>
        {withUnit(formatUpTo(host.tidal_flux, host.tidal_flux < 1 ? 2 : 0), 'W/m²')}{#if host.inside_edge}
          · <strong>{t.worldInfo.insideEdge}</strong>{/if}
      </dd>
    {/if}
  </dl>
{/if}
