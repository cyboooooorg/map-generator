<script lang="ts">
  // A planet's card in the star view of a planetary system: its facts beside
  // its marker, linked to it by a line, opened by hovering the marker and
  // kept open by its pin (see StarPane.svelte and lib/planetCard.ts).
  import { scale } from 'svelte/transition';
  import type { StarInfo } from '../lib/api/types/StarInfo';
  import { formatAu, formatInt, formatUpTo, formatYear, withUnit } from '../lib/format';
  import { t } from '../lib/i18n/index.svelte';
  import { type CardBox, linkEnd, zonePlace } from '../lib/planetCard';
  import type { SystemPlanet } from '../lib/systemStar';
  import { orbitalSpeed } from '../lib/viewer/starSystem';

  interface Props {
    planet: SystemPlanet;
    /** The system's star: its mass sets the speeds, its habitable zone the planet's place. */
    star: StarInfo;
    /** The planet's marker on the view, CSS px, and its distance from the star now, AU. */
    x: number;
    y: number;
    distance: number;
    /** Where the card sits (see layoutCards), and a call with its size, CSS px, once known or changed. */
    box: CardBox;
    onSize: (width: number, height: number) => void;
    /** Whether the planet is the one selected (the panel's). */
    selected: boolean;
    pinned: boolean;
    onPin: () => void;
    onUnpin: () => void;
    /** Opens the planet (its maps, or its generation). */
    onOpen: () => void;
    /** The pointer comes over the card, or leaves it (an unpinned card stays while it is over it). */
    onEnter: () => void;
    onLeave: () => void;
  }

  let { planet, star, x, y, distance, box, onSize, selected, pinned, onPin, onUnpin, onOpen, onEnter, onLeave }: Props = $props();

  let width = $state(220);
  let height = $state(160);
  $effect(() => onSize(width, height));
  const end = $derived(linkEnd(box, width, height, y));

  /** Jupiter masses, from Earth masses. */
  const jupiters = (earth: number) => earth / 317.8;
  const mass = $derived(
    jupiters(planet.mass_earth) < 0.3
      ? withUnit(formatUpTo(planet.mass_earth, planet.mass_earth < 10 ? 2 : 0), 'M⊕')
      : withUnit(formatUpTo(jupiters(planet.mass_earth), 2), 'MJ'),
  );
  const radius = $derived(withUnit(formatUpTo(planet.radius_km / 6371, planet.radius_km < 63_710 ? 2 : 1), 'R⊕'));
  const speed = $derived(withUnit(formatUpTo(orbitalSpeed(star.mass, planet.orbit_au, distance), 1), 'km/s'));
  const zone = $derived.by(() => {
    const place = zonePlace(planet.orbit_au, star.hz_inner_au, star.hz_outer_au);
    if (place.inside) return t.planetCard.zoneInside;
    const gap = formatAu(place.gapAu);
    return place.side === 'inner' ? t.planetCard.zoneInner({ gap }) : t.planetCard.zoneOuter({ gap });
  });
</script>

<svg class="link" aria-hidden="true" transition:scale={{ duration: 180, start: 1, opacity: 0 }}>
  <line x1={x} y1={y} x2={end.x} y2={end.y} />
  <circle cx={x} cy={y} r="3" />
</svg>
<section
  class="card"
  class:pinned
  aria-label={t.planetCard.label({ name: planet.name })}
  style:left={`${box.left}px`}
  style:top={`${box.top}px`}
  style:transform-origin={box.side === 'right' ? 'left center' : 'right center'}
  bind:clientWidth={width}
  bind:clientHeight={height}
  onpointerenter={onEnter}
  onpointerleave={onLeave}
  transition:scale={{ duration: 180, start: 0.9 }}
>
  <header>
    <div class="title">
      <strong>{planet.name}</strong>
      <small>{planet.typeLabel}{selected ? ` · ${t.planetCard.selected}` : ''}</small>
    </div>
    <button
      type="button"
      class="pin"
      class:on={pinned}
      aria-pressed={pinned}
      title={pinned ? t.planetCard.unpin : t.planetCard.pin}
      aria-label={pinned ? t.planetCard.unpin : t.planetCard.pin}
      onclick={() => (pinned ? onUnpin() : onPin())}
    >
      <svg viewBox="0 0 16 16" aria-hidden="true"
        ><path d="M5 1h6l-1 5 3 3H9.5L8 15l-1.5-6H3l3-3z" fill={pinned ? 'currentColor' : 'none'} stroke="currentColor" stroke-linejoin="round" /></svg
      >
    </button>
  </header>
  <dl>
    <dt>{t.planetCard.size}</dt>
    <dd>{t.planetCard.sizeValue({ radius, mass })}</dd>
    <dt>{t.planetCard.orbit}</dt>
    <dd>{t.planetCard.orbitValue({ distance: formatAu(planet.orbit_au), e: formatUpTo(planet.eccentricity, 3) })}</dd>
    <dt>{t.planetCard.motion}</dt>
    <dd>{t.planetCard.motionValue({ speed, year: formatYear(planet.year_days) })}</dd>
    <dt>{t.planetCard.equilibrium}</dt>
    <dd>{withUnit(formatInt(Math.round(planet.equilibrium_k - 273.15)), '°C')}</dd>
    <dt>{t.planetCard.zone}</dt>
    <dd>{zone}</dd>
    <dt>{t.planetCard.moons}</dt>
    <dd>{formatInt(planet.moons)}</dd>
  </dl>
  <footer>
    <small class="muted">{planet.generated ? t.planetCard.generated : t.planetCard.notGenerated}</small>
    <button type="button" onclick={onOpen}>{planet.generated ? t.planetCard.open : t.planetCard.openGenerate}</button>
  </footer>
</section>

<style>
  .link {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    pointer-events: none;
    overflow: visible;
  }
  .link line {
    stroke: #ffd27a;
    stroke-width: 1;
    opacity: 0.7;
  }
  .link circle {
    fill: none;
    stroke: #ffd27a;
    opacity: 0.7;
  }
  .card {
    position: absolute;
    width: max-content;
    max-width: 240px;
    min-width: 190px;
    display: grid;
    gap: 6px;
    padding: 8px 10px;
    background: var(--panel);
    border: 1px solid var(--border);
    border-radius: 8px;
    font-size: 0.78rem;
    pointer-events: auto;
    box-shadow: 0 4px 16px rgb(0 0 0 / 0.4);
  }
  .card.pinned {
    border-color: #ffd27a80;
  }
  header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 8px;
  }
  .title {
    display: grid;
  }
  .title small,
  .muted {
    color: var(--muted);
  }
  .pin {
    flex: none;
    width: 24px;
    height: 24px;
    padding: 3px;
    display: grid;
    place-items: center;
    color: var(--muted);
  }
  .pin.on {
    color: #ffd27a;
  }
  .pin svg {
    width: 14px;
    height: 14px;
  }
  dl {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 1px 10px;
    margin: 0;
  }
  dt {
    color: var(--muted);
  }
  dd {
    margin: 0;
  }
  footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }
</style>
