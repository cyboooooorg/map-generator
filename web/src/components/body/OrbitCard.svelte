<script lang="ts">
  // Orbit and light: where the body goes round, how it spins, what light it
  // gets and how large its star (or, for a moon, its host) looks.
  import './facts.css';
  import { t } from '../../lib/i18n/index.svelte';
  import { formatAu, formatInt, formatUpTo, formatYear, withUnit } from '../../lib/format';
  import Glossed from '../Glossed.svelte';
  import { resonanceRatio, resonanceTurns } from '../../lib/spinResonance';
  import { equilibriumK } from '../../lib/planetClass';
  import type { CardSource } from '../../lib/cardSource';

  let { world }: { world: CardSource } = $props();

  const body = $derived(world.shownSystemBody);
  const meta = $derived(world.meta);
  const star = $derived(meta?.star ?? null);
  const host = $derived(meta?.host ?? null);
  const hours = (h: number) => (h < 48 ? withUnit(formatUpTo(h, 1), 'h') : formatYear(h / 24));
</script>

{#if body}
  <dl class="facts">
    <dt><Glossed text={t.card.orbit} /></dt>
    <dd>
      <Glossed text={t.card.orbitValue({
        distance: body.parent ? withUnit(formatInt(body.orbit.semi_major_axis_km), 'km') : formatAu(body.orbit.semi_major_axis_au),
        eccentricity: formatUpTo(body.orbit.eccentricity, 3),
        period: formatYear(body.orbit.period_days),
      })} />
    </dd>
    <dt><Glossed text={t.card.spin} /></dt>
    <dd>
      {body.spin.locked
        ? t.card.locked
        : resonanceTurns(body.spin) > 0
          ? t.card.resonant({ day: hours(body.spin.period_hours), ratio: resonanceRatio(resonanceTurns(body.spin)) })
          : hours(body.spin.period_hours)} · {t.card.tilt({ tilt: `${formatUpTo(body.spin.axial_tilt_deg, 1)}°` })}
    </dd>
    <dt><Glossed text={t.card.light} /></dt>
    <dd>
      <Glossed text={t.card.lightValue({
        flux: formatUpTo(body.surroundings.flux_earth, body.surroundings.flux_earth < 0.1 ? 3 : 2),
        temperature: withUnit(formatInt(Math.round(equilibriumK(body.model, body.surroundings.equilibrium_k) - 273.15)), '°C'),
      })} />
    </dd>
    {#if star}
      <dt><Glossed text={t.worldInfo.starSize} /></dt>
      <dd>
        <Glossed text={t.worldInfo.starSizeValue({ size: `${formatUpTo(star.angular_diameter_deg, 2)}°`, sun: `${formatUpTo(0.53, 2)}°` })} />
      </dd>
    {/if}
    {#if host}
      <dt title={t.worldInfo.hostDayHelp}><Glossed text={t.worldInfo.hostDay} /></dt>
      <dd><Glossed text={formatYear(host.period_days)} /></dd>
      <dt><Glossed text={t.cards.sizeInSky} /></dt>
      <dd><Glossed text={t.worldInfo.hostSky({ size: `${formatUpTo(host.angular_diameter_deg, 1)}°` })} /></dd>
      <dt title={t.worldInfo.tidalHelp}><Glossed text={t.worldInfo.tidal} /></dt>
      <dd>
        <Glossed text={withUnit(formatUpTo(host.tidal_flux, host.tidal_flux < 1 ? 2 : 0), 'W/m²')} />{#if host.inside_edge}
          · <strong><Glossed text={t.worldInfo.insideEdge} /></strong>{/if}
      </dd>
    {/if}
  </dl>
{/if}
