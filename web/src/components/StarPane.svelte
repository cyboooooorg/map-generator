<script lang="ts">
  // The star view of a world: its star as a globe, its system around it,
  // and the numbers of both (see StarView.ts).
  import { onMount, untrack } from 'svelte';
  import type { SurfaceMaps } from '../lib/skyBodies';
  import PlanetCard from './PlanetCard.svelte';
  import { extremes, layoutCards, shownCards } from '../lib/planetCard';
  import type { Belt } from '../lib/api/types/Belt';
  import type { SystemPlanet } from '../lib/systemStar';
  import type { StarInfo } from '../lib/api/types/StarInfo';
  import type { StarActivity } from '../lib/api/types/StarActivity';
  import { formatAge, formatAu, formatCelsius, formatEccentricity, formatInt, formatNumber, formatOrbit, formatUpTo, formatYear, withUnit } from '../lib/format';
  import { unitTooltip } from '../lib/units';
  import type { Moon } from '../lib/api/types/Moon';
  import type { Giant } from '../lib/api/types/Giant';
  import type { GiantSystem } from '../lib/api/types/GiantSystem';
  import type { GiantInfo } from '../lib/api/types/GiantInfo';
  import type { HostGiant } from '../lib/api/types/HostGiant';
  import { compressedFrom, defaultSystemSpeed, SYSTEM_SPEED_MAX, SYSTEM_SPEED_MIN, systemCompressedFrom, giantMarkerSize, meanOrbitalSpeed, moonOrbits, orbitalSpeed, PLANET_MARKER, STAR_VIEW_SKY_AXES } from '../lib/viewer/starSystem';
  import { t } from '../lib/i18n/index.svelte';
  import { layoutLabels } from '../lib/viewer/labelLayout';
  import type { ScreenAxis } from '../lib/viewer/axes';
  import AxesGizmo from './AxesGizmo.svelte';
  import Dock, { type DockPanel } from './dock/Dock.svelte';
  import PlaybackBar from './dock/PlaybackBar.svelte';
  import ToggleBar, { type Toggle } from './dock/ToggleBar.svelte';
  import { onThemeChange, overlayColours } from '../lib/themes';
  import { type SystemLabels, StarView, YEAR_S } from '../lib/viewer/StarView';
  import { currentProfile } from '../lib/viewer/quality';
  import type { SkyChoice } from '../lib/viewer/skyBackdrop';
  import type { SkyExposure } from '../lib/viewer/skyClouds';
  import { skyToggles, type SkySetting } from '../lib/viewer/skyToggles';
  import { FIXED_SKY, type SkyLook } from '../lib/viewer/skyLook';

  interface Props {
    star: StarInfo;
    /** The star's activity (its living surface, see lib/viewer/starSurface.ts); null shows its limb and granules only. */
    activity?: StarActivity | null;
    /** Orientation of the orbit's periapsis, radians (see periapsisAngle). */
    omega: number;
    /** Whether the star turns and boils (thirty frames a second at most). */
    animated: boolean;
    /** Whether the live distances are shown. */
    distances: boolean;
    /** Whether the orbit line and the habitable zone are drawn. */
    orbit: boolean;
    zone: boolean;
    /** Whether the coronal loops (and eruptions) are drawn. */
    prominences: boolean;
    /** The planet's moons, nearest first, drawn around it. */
    moons?: readonly Moon[];
    /** Whether the planet and its largest moon are a binary. */
    binary?: boolean;
    planetRadiusKm?: number;
    /** The giant planets of the system and their effects; null when not modelled. */
    giantSystem?: GiantSystem | null;
    /** Whether the giants are drawn. */
    giants: boolean;
    /**
     * In a planetary system, whether each planet has a dotted line to the
     * star with its distance now, and a velocity vector with its speed now.
     */
    links?: boolean;
    velocities?: boolean;
    /** In a planetary system, whether each planet has a vector of the star's pull on it, along its line to the star. */
    pulls?: boolean;
    /** In a planetary system: the frost line, the planets' Hill spheres, and whether it is drawn to scale. */
    frost?: boolean;
    hills?: boolean;
    realScale?: boolean;
    /** In a planetary system: its belts (see `crate::belts`), and whether its belts, trojans and the selected planet's Lagrange points are drawn. */
    belts?: readonly Belt[];
    showBelts?: boolean;
    showTrojans?: boolean;
    showLagrange?: boolean;
    /** The world itself when it is a giant planet, or the giant it orbits. */
    worldGiant?: GiantInfo | null;
    /** The axial tilt of `worldGiant`, degrees: its rings and ring moons lie in its equator. */
    worldTiltDeg?: number;
    /** The giant the world orbits, as its moon. */
    host?: HostGiant | null;
    /**
     * A planetary system's other planets, drawn as the giants are: their
     * kinds as the list shows them (in the giants' order), and what a click
     * on one does.
     */
    kinds?: readonly string[] | null;
    onPickPlanet?: ((index: number) => void) | null;
    /** The name of the planet whose orbit is drawn, labelling it with its distance. */
    planetName?: string | null;
    /**
     * In a planetary system, the index among the giants (then every
     * planet) of the planet `star` describes; null for a lone world.
     */
    selected?: number | null;
    /**
     * A planetary system's planets, in the giants' order, for their cards
     * (hovered, or pinned with a click); null for a lone world, which has
     * none.
     */
    planets?: readonly SystemPlanet[] | null;
    /** The planets whose cards are pinned, by id, in pinning order. */
    pinned?: readonly string[];
    onPin?: (id: string) => void;
    /** A planet to fly out from when the view opens (the travelling back from its view), by index; then `onFlown`. */
    flyFrom?: number | null;
    onFlown?: () => void;
    /** Selects a planet of the system by its index, staying in the star view (its row in the list). */
    onSelectPlanet?: ((index: number) => void) | null;
    onUnpin?: (id: string) => void;
    /** Unpins every card. */
    onUnpinAll?: () => void;
    /** Opens a moon of a planet (a link on its card), by body id. */
    onOpenMoon?: (id: string) => void;
    /** Opens a planet (a double click on its marker, or its card's button). */
    onOpenPlanet?: ((index: number) => void) | null;
    /** The maps wrapped round each planet's marker, in the giants' order, null for its colour. */
    maps?: readonly (SurfaceMaps | null)[] | null;
    /**
     * The day shown, Earth days from the system's t = 0, shared with the
     * star view (StarSurfacePane): the view writes it as its clock runs,
     * and takes a day written from outside when it opens or shows again.
     */
    clockDays?: number;
    /**
     * Hidden behind the star view: the canvas and its chrome are not shown
     * and the animation loop stops (no frame is drawn unless a setting
     * changes), but the view is kept,
     * with its camera, speed and switches, for when it shows again.
     */
    hidden?: boolean;
    /** Opens the star view (the Star button, a double click on the star); null hides the button. */
    onOpenStar?: (() => void) | null;
    /** The system's own sky (see lib/viewer/skyLook.ts). */
    look?: SkyLook;
    /** Which sky is drawn: the system's own, the fixed one or none. */
    sky?: SkyChoice;
    /** How its clouds and nebulae are rendered: as the eye sees them or as a long exposure. */
    skyExposure?: SkyExposure;
    /** The sky's setting, shared with the planet view (Viewer.svelte), for its switches; null shows none. */
    skySetting?: SkySetting | null;
  }

  let {
    star,
    activity = null,
    omega,
    animated = $bindable(),
    distances = $bindable(),
    orbit = $bindable(),
    zone = $bindable(),
    prominences = $bindable(),
    moons = [],
    binary = false,
    planetRadiusKm = 6371,
    giantSystem = null,
    giants = $bindable(),
    links = $bindable(true),
    velocities = $bindable(true),
    pulls = $bindable(true),
    frost = $bindable(true),
    hills = $bindable(false),
    realScale = $bindable(false),
    belts = [],
    showBelts = $bindable(true),
    showTrojans = $bindable(true),
    showLagrange = $bindable(false),
    worldGiant = null,
    worldTiltDeg = 0,
    host = null,
    kinds = null,
    onPickPlanet = null,
    planetName = null,
    selected = null,
    planets = null,
    pinned = [],
    onPin = () => {},
    onSelectPlanet = null,
    flyFrom = null,
    onFlown = () => {},
    onUnpin = () => {},
    onUnpinAll = () => {},
    onOpenMoon = () => {},
    onOpenPlanet = null,
    maps = null,
    clockDays = $bindable(0),
    hidden = false,
    onOpenStar = null,
    look = FIXED_SKY,
    sky = 'system',
    skyExposure = 'photo',
    skySetting = null,
  }: Props = $props();
  // Each giant under its designation (MGS 48213 d), once the summary gives it.
  const giantList = $derived(
    (giantSystem?.giants ?? []).map((g, i) => ({ ...g, name: g.name, kindLabel: kinds?.[i] ?? null })),
  );
  /** Whether some giant lies where the view compresses distances. */
  const compressFrom = $derived(selected !== null ? systemCompressedFrom(star) : compressedFrom(star));
  const compressed = $derived(!(selected !== null && realScale) && giantList.some((g) => g.orbit_au * (1 + g.eccentricity) > compressFrom));
  let labels = $state<SystemLabels | null>(null);
  /** Share of the year elapsed since the periapsis, while animated. */
  let yearFraction = $state(0);
  /** Earth days simulated since the animation started. */
  let elapsedDays = $state(0);
  /** How fast a planetary system's clock runs, a power of two of Earth days per second (see defaultSystemSpeed). */
  let speed = $state(SYSTEM_SPEED_MIN);
  /** The innermost planet's year, days: another system opens at its own speed. */
  const innermostYear = $derived(giantList.length > 0 ? Math.min(...giantList.map((g) => g.year_days)) : 0);
  $effect(() => {
    if (innermostYear > 0) speed = defaultSystemSpeed(innermostYear);
  });
  /** Width of the view, to keep the labels near its right edge on screen. */
  let width = $state(0);

  /** Light the planet gets at `au`, relative to Earth's (the model Sun at 1 AU). */
  const lightAt = (au: number) => star.luminosity / (au * au) / 1.0233;
  const inZone = (au: number) => au >= star.hz_inner_au && au <= star.hz_outer_au;

  let canvas: HTMLCanvasElement;
  let view = $state<StarView | null>(null);

  onMount(() => {
    const created = new StarView(canvas, currentProfile());
    created.setOverlayColours(overlayColours());
    created.onAxes = (next) => (axes = next);
    const stopTheme = onThemeChange(() => created.setOverlayColours(overlayColours()));
    created.onLabels = (next) => (labels = next);
    created.onClock = (fraction, days) => {
      yearFraction = fraction;
      elapsedDays = days;
      reported = days;
      clockDays = days;
    };
    view = created;
    return () => {
      stopTheme();
      created.dispose();
      view = null;
    };
  });

  $effect(() => view?.setSkyLook(look));
  $effect(() => view?.setSkyAxes(STAR_VIEW_SKY_AXES));
  $effect(() => view?.setSkyChoice(sky));
  $effect(() => view?.setSkyExposure(skyExposure));
  $effect(() => view?.setStar(star, omega, giantList, selected, activity));
  /** The last day this view reported (see clockDays): any other is a day set from outside. */
  let reported = Number.NaN;
  // A day set from outside (the star view's, or the planet view's on
  // opening), applied after setStar and only while shown: the hidden view
  // draws nothing.
  $effect(() => {
    const days = clockDays;
    if (!view || hidden || days === reported) return;
    untrack(() => view?.setSystemDay(days));
  });
  $effect(() => {
    if (view) view.onOpenStar = onOpenStar;
  });
  /** The world's axes on screen, for the gizmo. */
  let axes = $state<ScreenAxis[]>([]);
  // Back from a planet's view: out from it to the whole system, once.
  $effect(() => {
    if (view && flyFrom !== null && giantList.length > flyFrom) {
      const from = flyFrom;
      untrack(() => {
        view?.flyOutFrom(from);
        onFlown();
      });
    }
  });
  $effect(() => view?.setPlanetMaps(maps ?? []));
  // Planets not generated yet wear their preview (belts, land, caps).
  $effect(() => view?.setPlanetPreviews(planets?.map((p) => (p.generated ? null : p.preview)) ?? []));

  /** The planet under the pointer, by index, while its card is open unpinned. */
  let hovered = $state<number | null>(null);
  /** Whether the pointer is over a card (an unpinned card then stays open). */
  let overCard = false;
  let closeTimer: ReturnType<typeof setTimeout> | undefined;
  /** Leave the time to cross from a marker to its card before closing it. */
  const CLOSE_MS = 250;
  const scheduleClose = () => {
    clearTimeout(closeTimer);
    closeTimer = setTimeout(() => {
      if (!overCard) hovered = null;
    }, CLOSE_MS);
  };
  const ids = $derived(planets?.map((p) => p.id) ?? []);
  const cards = $derived(planets ? shownCards(ids, pinned, hovered) : []);
  // Another system: no card left open from the last one.
  const idKey = $derived(ids.join(' '));
  $effect(() => {
    void idKey;
    hovered = null;
  });
  $effect(() => {
    if (!view) return;
    view.onHoverPlanet = planets
      ? (index) => {
          if (index === null) scheduleClose();
          else {
            clearTimeout(closeTimer);
            hovered = index;
          }
        }
      : null;
    view.onOpenPlanet = planets ? onOpenPlanet : null;
  });
  $effect(() => () => clearTimeout(closeTimer));
  // Rings round the selected planet, the pinned ones and the one hovered.
  $effect(() => {
    view?.setHighlighted(planets ? [...(selected !== null && selected >= 0 ? [selected] : []), ...cards.map((c) => c.index)] : []);
  });
  /** The size of the view, for the cards to keep inside it. */
  let viewWidth = $state(0);
  let viewHeight = $state(0);
  /** Each card's size once drawn, CSS px, by planet id. */
  let cardSizes = $state<Record<string, { width: number; height: number }>>({});
  /** The cards with their planets' places, pinned first so that they keep theirs. */
  const placedCards = $derived.by(() => {
    if (!planets) return [];
    const spots = cards.flatMap((card) => {
      const planet = planets[card.index];
      const at = labels?.giants.find((g) => g.index === card.index);
      if (!planet || !at) return [];
      const size = cardSizes[planet.id] ?? { width: 220, height: 160 };
      return [{ ...card, planet, at, ...size }];
    });
    const boxes = layoutCards(
      spots.map((s) => ({ x: s.at.x, y: s.at.y, width: s.width, height: s.height })),
      viewWidth,
      viewHeight,
    );
    // The pinned cards compared: each line's highest and lowest marked.
    const pinnedSpots = spots.filter((s) => s.pinned);
    const marks = extremes(
      pinnedSpots.map(({ planet, at }) => ({
        radius: planet.radius_km,
        mass: planet.mass_earth,
        orbit: planet.orbit_au,
        speed: orbitalSpeed(star.mass, planet.orbit_au, at.distance),
        year: planet.year_days,
        day: planet.locked ? planet.year_days * 24 : planet.day_hours,
        light: planet.flux_earth,
        gravity: planet.gravity,
        equilibrium: planet.equilibrium_k,
        moons: planet.moons,
      })),
    );
    return spots.map((s, k) => ({ ...s, box: boxes[k]!, marks: (s.pinned ? marks[pinnedSpots.indexOf(s)] : undefined) ?? {} }));
  });
  $effect(() => {
    if (view) view.onPickPlanet = onPickPlanet;
  });
  // A planetary system's planets are the system itself: always drawn.
  $effect(() => view?.setGiantsVisible(giants || selected !== null));
  $effect(() => view?.setWorldGiant(worldGiant, worldTiltDeg));

  /** Jupiter masses, from Earth masses. */
  const jupiters = (earth: number) => earth / 317.8;
  /** One line on a giant: its letter, kind, mass, orbit and year. */
  const giantText = (g: Giant & { kindLabel?: string | null }) =>
    t.starView.giantLine({
      name: g.name,
      kind: g.kindLabel ?? (g.hot ? t.starView.hotGiant : (t.starView.giantKinds[g.kind] ?? '')),
      mass: jupiters(g.mass_earth) < 0.3 ? withUnit(formatUpTo(g.mass_earth, g.mass_earth < 10 ? 2 : 0), 'M⊕') : withUnit(formatUpTo(jupiters(g.mass_earth), 2), 'MJ'),
      orbit: formatAu(g.orbit_au),
      year: formatYear(g.year_days),
    });
  /** Its clouds, radius, temperature and rings, in a tooltip. */
  const giantHelp = (g: Giant) =>
    t.starView.giantHelp({
      clouds: t.starView.cloudClasses[g.class] ?? '',
      radius: withUnit(formatUpTo(g.radius_earth / 11.21, 2), 'RJ'),
      temperature: withUnit(formatInt(Math.round(g.effective_k)), 'K'),
      eccentricity: formatUpTo(g.eccentricity, 2),
      rings: t.starView.rings[g.rings] ?? '',
    });
  // Hidden, the view runs no animation loop at all.
  $effect(() => view?.setAnimated(animated && !hidden));
  $effect(() => view?.setPlanetLinks(links));
  $effect(() => view?.setVelocities(velocities));
  $effect(() => view?.setPulls(pulls));
  $effect(() => view?.setFrostLine(frost));
  $effect(() => view?.setHillSpheres(hills));
  $effect(() => view?.setBelts(belts));
  $effect(() => view?.setSmallBodies(showBelts, showTrojans, showLagrange));
  /** A belt's name and span, for its label and its row. */
  const beltRange = (belt: Belt) => `${formatUpTo(belt.inner_au, 2)}–${withUnit(formatUpTo(belt.outer_au, 2), 'AU')}`;
  const beltName = (belt: Belt) => t.starView.beltNames[belt.kind] ?? belt.kind;
  $effect(() => view?.setRealScale(realScale));
  /** The longest year of a planetary system, days: the time bar's span. */
  const longestYear = $derived(giantList.length > 0 ? Math.max(...giantList.map((g) => g.year_days)) : 0);
  /** Moves a planetary system's clock to day `days` (the time bar). */
  const goToDay = (days: number) => view?.setSystemDay(days);
  $effect(() => view?.setSpeed(2 ** speed));
  $effect(() => view?.setDistances(distances));
  $effect(() => view?.setOrbitVisible(orbit));
  $effect(() => view?.setZoneVisible(zone));
  $effect(() => view?.setProminences(prominences));
  // A giant's moon: the marker on the orbit is the giant, the world goes
  // round it at its pace relative to the year.
  $effect(() =>
    view?.setMoons(
      host
        ? [
            {
              radius: giantMarkerSize(host.giant.radius_earth) * 1.9,
              size: PLANET_MARKER * 0.55,
              inclination: 0,
              speed: (2 * Math.PI) / Math.max(2, (host.period_days / Math.max(star.year_days, 1e-3)) * YEAR_S),
              phase: 0,
              swing: 0,
            },
          ]
        : moonOrbits(moons, planetRadiusKm, star.year_days, planetYearS, binary),
    ),
  );

  /**
   * Seconds of animation for one year of the planet whose moons are drawn:
   * YEAR_S for a lone world; in a planetary system, its year in days, the
   * system's clock running a day a second at ×1 (the speed then runs the
   * moons with the planets).
   */
  const planetYearS = $derived(selected !== null && giantList.length > 0 ? star.year_days : YEAR_S);
  const kms = (v: number) => withUnit(formatUpTo(v, 1), 'km/s');
  /** Mean speed, and its range on an eccentric orbit. */
  const speedText = $derived.by(() => {
    const mean = kms(meanOrbitalSpeed(star.mass, star.orbit_au));
    if (star.eccentricity <= 0.001) {
      return t.starView.speedCircular({ mean, earth: kms(29.78) });
    }
    return t.starView.speedRange({
      mean,
      fastest: kms(orbitalSpeed(star.mass, star.orbit_au, star.periapsis_au)),
      slowest: kms(orbitalSpeed(star.mass, star.orbit_au, star.apoapsis_au)),
    });
  });

  /** How fast time runs in the animation, and the day of the year it shows. */
  const clockText = $derived.by(() => {
    if (selected !== null && giantList.length > 0) {
      // A planetary system runs on one clock, in Earth days a second.
      return t.starView.systemClock({
        days: formatInt(2 ** speed),
        day: formatInt(Math.floor(elapsedDays) + 1),
      });
    }
    const days = star.year_days / YEAR_S;
    const scale = days < 1 ? t.starView.hours({ hours: formatUpTo(days * 24, 1) }) : t.worldInfo.yearDays({ days: formatUpTo(days, 1) });
    return t.starView.clock({
      scale,
      factor: formatInt(Math.round((days * 86_400) / 100) * 100),
      day: formatInt(Math.floor(yearFraction * star.year_days) + 1),
      year: formatInt(Math.round(star.year_days)),
    });
  });

  /** A label's height, CSS px (0.75rem text and its padding). */
  const LABEL_H = 18;
  /** A label's width, CSS px, estimated from its text (the layout runs on every frame: no measuring). */
  const labelWidth = (text: string) => text.length * 7 + 10;
  /** The star's pull, in m/s² or mm/s² (the Sun's on Earth is 5.9 mm/s²). */
  const pullText = (a: number) =>
    a >= 0.1 ? withUnit(formatUpTo(a, 2), 'm/s²') : withUnit(formatUpTo(a * 1000, a * 1000 < 0.1 ? 3 : 2), 'mm/s²');
  /**
   * A label written along the line from `a` to `b` (CSS px) at the share `at`
   * of the way, kept upright and just above the line: its centre, its angle
   * (degrees) and its box for the layout.
   */
  const alongLine = (a: { x: number; y: number }, b: { x: number; y: number }, at: number, text: string) => {
    const w = labelWidth(text);
    // A line shorter than its text is labelled level, beside it.
    let angle = Math.hypot(b.x - a.x, b.y - a.y) < w ? 0 : (Math.atan2(b.y - a.y, b.x - a.x) * 180) / Math.PI;
    if (angle > 90) angle -= 180;
    if (angle < -90) angle += 180;
    const rad = (angle * Math.PI) / 180;
    // Just above the line: the normal pointing up the screen.
    const x = a.x + (b.x - a.x) * at + Math.sin(rad) * (LABEL_H / 2);
    const y = a.y + (b.y - a.y) * at - Math.cos(rad) * (LABEL_H / 2);
    const boxW = Math.abs(w * Math.cos(rad)) + Math.abs(LABEL_H * Math.sin(rad));
    const boxH = Math.abs(w * Math.sin(rad)) + Math.abs(LABEL_H * Math.cos(rad));
    // Tilted, it stays on its line or hides (a shifted one would leave it); level, it may move a line.
    return { x, y, angle, text, box: { x: x - boxW / 2, y: y - boxH / 2, width: boxW, height: boxH, step: angle === 0 ? LABEL_H : 0 } };
  };
  /**
   * The labels of a planetary system's planets, kept apart (see
   * layoutLabels), the most useful first: the names (the selected planet's
   * first), each on the side of its planet away from its velocity vector;
   * the distances, along the lines to the star; the speeds, at the
   * vectors; the star's pulls, along the lines near the planets; and the
   * habitable zone's width and the compression circle's meaning.
   */
  const placed = $derived.by(() => {
    const none = { names: [], lines: [], speeds: [], small: [] };
    if (!labels) return none;
    // The selected planet's rich label (with Distances) already gives its distance.
    const rich = labels.planet !== null;
    const system = selected !== null;
    const giants = labels.giants
      .filter((g) => !(rich && g.index === selected))
      .sort((a, b) => Number(b.index === selected) - Number(a.index === selected) || a.index - b.index);
    const names = giants.map((g) => {
      const text = system && links ? g.name : t.starView.orbitLabel({ name: g.name, distance: formatAu(g.au) });
      const width = labelWidth(text);
      // Away from the velocity vector, else above and to the right.
      let ux = 0.7;
      let uy = -0.7;
      if (g.arrow) {
        const dx = g.arrow.x - g.x;
        const dy = g.arrow.y - g.y;
        const d = Math.hypot(dx, dy);
        if (d > 1) {
          ux = -dx / d;
          uy = -dy / d;
        }
      }
      const ax = g.x + ux * 12;
      const ay = g.y + uy * 12;
      const box = { x: ux >= 0 ? ax : ax - width, y: ay - LABEL_H / 2 + (uy * LABEL_H) / 2, width, height: LABEL_H };
      return { index: g.index, text, box };
    });
    type Along = ReturnType<typeof alongLine> & { key: string; kind: 'distance' | 'pull' | 'zone' | 'boundary' | 'frost' };
    const lines: Along[] = [];
    if (system && labels.star) {
      for (const g of giants) {
        if (links) lines.push({ ...alongLine(labels.star, g, 0.65, formatAu(g.distance)), key: `d${g.index}`, kind: 'distance' });
      }
      for (const g of giants) {
        if (!g.pull) continue;
        // Beside the planet, just past the pull arrow towards the star, not over the planet.
        const text = pullText(g.gravity);
        const span = Math.hypot(g.x - labels.star.x, g.y - labels.star.y);
        const arrow = 2 * Math.hypot(g.x - g.pull.x, g.y - g.pull.y);
        const share = span > 0 ? 1 - (arrow + labelWidth(text) / 2 + 12) / span : 1;
        lines.push({ ...alongLine(labels.star, g, Math.min(0.95, Math.max(0.45, share)), text), key: `p${g.index}`, kind: 'pull' });
      }
    }
    if (system && labels.inner && labels.outer) {
      const area = Math.PI * (star.hz_outer_au ** 2 - star.hz_inner_au ** 2);
      const text = t.starView.zoneWidth({
        width: formatAu(star.hz_outer_au - star.hz_inner_au),
        area: formatUpTo(area, area < 0.01 ? 4 : area < 1 ? 3 : 2),
      });
      lines.push({ ...alongLine(labels.inner, labels.outer, 0.5, text), key: 'zone', kind: 'zone' });
    }
    if (labels.frost) {
      const text = t.starView.frostLabel({ distance: formatAu(labels.frost.au) });
      const w = labelWidth(text);
      lines.push({
        x: labels.frost.x,
        y: labels.frost.y + LABEL_H / 2,
        angle: 0,
        text,
        box: { x: labels.frost.x - w / 2, y: labels.frost.y, width: w, height: LABEL_H, step: LABEL_H },
        key: 'frost',
        kind: 'frost',
      });
    }
    if (labels.boundary) {
      const text = t.starView.compressedLabel({ distance: formatAu(labels.boundary.au) });
      const w = labelWidth(text);
      lines.push({
        x: labels.boundary.x,
        y: labels.boundary.y - LABEL_H / 2,
        angle: 0,
        text,
        box: { x: labels.boundary.x - w / 2, y: labels.boundary.y - LABEL_H, width: w, height: LABEL_H, step: LABEL_H },
        key: 'boundary',
        kind: 'boundary',
      });
    }
    const speeds = giants.flatMap((g) => {
      if (!g.arrow) return [];
      const text = kms(g.speed);
      const w = labelWidth(text);
      return [{ index: g.index, x: g.arrow.x, y: g.arrow.y, text, box: { x: g.arrow.x - w / 2, y: g.arrow.y - 1.3 * LABEL_H, width: w, height: LABEL_H } }];
    });
    // The labels drawn on their own: the planet's rich one, its apsides and the zone's edges.
    const fixed = (at: { x: number; y: number } | null, text: string) =>
      at ? [{ x: at.x > width * 0.6 ? at.x - 8 - labelWidth(text) : at.x + 8, y: at.y - LABEL_H / 2, width: labelWidth(text), height: LABEL_H }] : [];
    const keepClear = [
      ...(labels.planet ? [{ x: labels.planet.x + 8, y: labels.planet.y - 30, width: 170, height: 60 }] : []),
      ...fixed(labels.periapsis, t.starView.periapsis({ distance: formatAu(star.periapsis_au) })),
      ...fixed(labels.apoapsis, t.starView.apoapsis({ distance: formatAu(star.apoapsis_au) })),
      ...fixed(labels.inner, formatAu(star.hz_inner_au)),
      ...fixed(labels.outer, formatAu(star.hz_outer_au)),
    ];
    const distances = lines.filter((l) => l.kind !== 'pull');
    const pullLabels = lines.filter((l) => l.kind === 'pull');
    // The small bodies last, level beside their point: the Lagrange points,
    // then the belts, then the swarms of the planets selected or pinned
    // (every planet's would crowd the view).
    const level = (at: { x: number; y: number }, text: string, key: string, kind: string) => {
      const w = labelWidth(text);
      return { x: at.x, y: at.y, text, key, kind, box: { x: at.x + 6, y: at.y - LABEL_H, width: w, height: LABEL_H, step: LABEL_H } };
    };
    const shownSwarms = new Set([...(selected !== null && selected >= 0 ? [selected] : []), ...(planets ?? []).flatMap((p, i) => (pinned.includes(p.id) ? [i] : []))]);
    const small = labels.small
      ? [
          ...labels.small.lagrange.map((l) => level(l, l.name, `lp${l.name}`, l.stable ? 'lagrange' : 'lagrange unstable')),
          ...labels.small.belts.flatMap((l) => {
            const belt = belts[l.index];
            return belt
              ? [level(l, t.starView.beltLabel({ name: beltName(belt), range: beltRange(belt) }), `b${l.index}`, belt.composition === 'icy' ? 'belt icy' : 'belt')]
              : [];
          }),
          ...labels.small.swarms
            .filter((l) => shownSwarms.has(l.planet))
            .map((l) => level(l, t.starView.swarmLabel({ point: l.point, count: formatInt(l.count) }), `s${l.planet}${l.point}`, 'swarm')),
        ]
      : [];
    // The speeds come right after the names: they hide last of the numbers.
    const order = [
      ...names.map((n) => n.box),
      ...speeds.map((v) => v.box),
      ...distances.map((l) => l.box),
      ...pullLabels.map((l) => l.box),
      ...small.map((l) => l.box),
    ];
    const places = layoutLabels(order, keepClear);
    let k = 0;
    const take = () => places[k++]!;
    // To scale, a name sits slanted at the tip of its planet's pin, always shown.
    const placedNames = names.map((n) => {
      const place = take();
      const pin = labels!.giants.find((g) => g.index === n.index)?.pin ?? null;
      return pin ? { ...n, box: { ...n.box, x: pin.x, y: pin.y }, dy: 0, visible: true, slanted: true } : { ...n, ...place, slanted: false };
    });
    const placedSpeeds = speeds.map((v) => ({ ...v, ...take() }));
    return {
      names: placedNames,
      lines: [...distances, ...pullLabels].map((l) => ({ ...l, ...take() })),
      speeds: placedSpeeds,
      small: small.map((l) => ({ ...l, ...take() })),
    };
  });

  /** The display switches, as the icon bar groups them: lines, measures, structure, sky. */
  const toggleGroups = $derived<Toggle[][]>([
    [
      { key: 'orbit', label: t.starView.showOrbit, icon: 'orbit', on: orbit, set: (v) => (orbit = v) },
      { key: 'zone', label: t.starView.showZone, icon: 'zone', on: zone, set: (v) => (zone = v) },
      // A star without activity, or a radiative one (no convective zone, no
      // field to hold loops), has none to show.
      ...(activity !== null && activity.envelope !== 'radiative'
        ? [{ key: 'prominences', label: t.starView.prominences, help: t.starView.prominencesHelp, icon: 'prominence' as const, on: prominences, set: (v: boolean) => (prominences = v) }]
        : []),
      ...(giantList.length > 0 && selected === null
        ? [{ key: 'giants', label: t.starView.showGiants, help: t.starView.showGiantsHelp, icon: 'giants' as const, on: giants, set: (v: boolean) => (giants = v) }]
        : []),
    ],
    [
      { key: 'distances', label: t.starView.distances, help: t.starView.distancesHelp, icon: 'distance', on: distances, set: (v) => (distances = v) },
      ...(selected !== null
        ? [
            { key: 'links', label: t.starView.showLinks, help: t.starView.showLinksHelp, icon: 'links' as const, on: links, set: (v: boolean) => (links = v) },
            { key: 'velocities', label: t.starView.showVelocities, help: t.starView.showVelocitiesHelp, icon: 'velocity' as const, on: velocities, set: (v: boolean) => (velocities = v) },
            { key: 'pulls', label: t.starView.showPulls, help: t.starView.showPullsHelp, icon: 'pull' as const, on: pulls, set: (v: boolean) => (pulls = v) },
          ]
        : []),
    ],
    selected !== null
      ? [
          { key: 'frost', label: t.starView.showFrost, help: t.starView.showFrostHelp, icon: 'frost', on: frost, set: (v) => (frost = v) },
          { key: 'hills', label: t.starView.showHills, help: t.starView.showHillsHelp, icon: 'hills', on: hills, set: (v) => (hills = v) },
          { key: 'belts', label: t.starView.showBelts, help: t.starView.showBeltsHelp, icon: 'belt', on: showBelts, set: (v) => (showBelts = v) },
          { key: 'trojans', label: t.starView.showTrojans, help: t.starView.showTrojansHelp, icon: 'trojans', on: showTrojans, set: (v) => (showTrojans = v) },
          { key: 'lagrange', label: t.starView.showLagrange, help: t.starView.showLagrangeHelp, icon: 'lagrange', on: showLagrange, set: (v) => (showLagrange = v) },
          { key: 'scale', label: t.starView.realScale, help: t.starView.realScaleHelp, icon: 'scale', on: realScale, set: (v) => (realScale = v) },
        ]
      : [],
    // The sky behind the system: the same switches as the planet view's, on the same setting.
    skySetting ? skyToggles(skySetting) : [],
  ]);
  /** The panels of the dock: the star, the planet shown, and the system's planets. */
  const dockPanels = $derived<DockPanel[]>([
    { id: 'star', title: t.starView.starSection, icon: 'star', content: starPanel },
    { id: 'planet', title: selected !== null ? t.starView.selectedSection : t.starView.planetSection, icon: 'planet', content: planetPanel },
    ...(!worldGiant || selected !== null
      ? [{ id: 'planets', title: kinds ? t.starView.planets : t.starView.giantsSection, icon: 'list' as const, content: planetsPanel }]
      : []),
  ]);

  /** Back to the starting view of the system. */
  export function resetView(): void {
    view?.resetView();
  }

  /** Flies the camera to planet `index`, then calls `done` (the travelling into its view). */
  export function flyToPlanet(index: number, done: () => void): void {
    if (view) view.flyToPlanet(index, done);
    else done();
  }
</script>

<div class="star-pane" class:concealed={hidden}>
<canvas bind:this={canvas} class="star-canvas"></canvas>
{#if labels}
  <div class="labels" aria-hidden="true" bind:clientWidth={width}>
    {#if labels.planet}
      <span class="label planet" class:left={labels.planet.x > width * 0.6} style:left={`${labels.planet.x}px`} style:top={`${labels.planet.y}px`}
        >{t.starView.now({
          distance: formatAu(labels.planet.au),
          speed: kms(orbitalSpeed(star.mass, star.orbit_au, labels.planet.au)),
          light: `${formatUpTo(lightAt(labels.planet.au), 2)}×`,
          zone: inZone(labels.planet.au) ? t.starView.inZone : t.starView.outOfZone,
        })}</span
      >
    {/if}
    {#if labels.periapsis && star.eccentricity > 0.001}
      <span class="label apsis" class:left={labels.periapsis.x > width * 0.6} style:left={`${labels.periapsis.x}px`} style:top={`${labels.periapsis.y}px`}
        >{t.starView.periapsis({ distance: formatAu(star.periapsis_au) })}</span
      >
    {/if}
    {#if labels.apoapsis && star.eccentricity > 0.001}
      <span class="label apsis" class:left={labels.apoapsis.x > width * 0.6} style:left={`${labels.apoapsis.x}px`} style:top={`${labels.apoapsis.y}px`}
        >{t.starView.apoapsis({ distance: formatAu(star.apoapsis_au) })}</span
      >
    {/if}
    {#if labels.inner}
      <span class="label zone" class:left={labels.inner.x > width * 0.6} style:left={`${labels.inner.x}px`} style:top={`${labels.inner.y}px`}
        >{formatAu(star.hz_inner_au)}</span
      >
    {/if}
    {#if labels.outer}
      <span class="label zone" class:left={labels.outer.x > width * 0.6} style:left={`${labels.outer.x}px`} style:top={`${labels.outer.y}px`}
        >{formatAu(star.hz_outer_au)}</span
      >
    {/if}
    {#if labels.self && planetName}
      <span class="label giant self" style:left={`${labels.self.x}px`} style:top={`${labels.self.y}px`}
        >{t.starView.orbitLabel({ name: planetName, distance: formatAu(star.orbit_au) })}</span
      >
    {/if}
    {#each placed.names as giant (giant.index)}
      <span
        class="label giant boxed"
        class:slanted={giant.slanted}
        class:self={giant.index === selected}
        class:hidden={!giant.visible}
        style:left={`${giant.box.x}px`}
        style:top={`${giant.box.y + giant.dy}px`}>{giant.text}</span
      >
    {/each}
    {#if labels.starPin}
      <span class="label giant boxed slanted star-pin" style:left={`${labels.starPin.x}px`} style:top={`${labels.starPin.y}px`}
        >{star.type}</span
      >
    {/if}
    {#each placed.lines as line (line.key)}
      <span
        class="label along {line.kind}"
        class:hidden={!line.visible}
        style:left={`${line.x}px`}
        style:top={`${line.y + line.dy}px`}
        style:transform={`translate(-50%, -50%) rotate(${line.angle}deg)`}>{line.text}</span
      >
    {/each}
    {#each placed.small as label (label.key)}
      <span class="label small {label.kind}" class:hidden={!label.visible} style:left={`${label.x}px`} style:top={`${label.y + label.dy}px`}
        >{label.text}</span
      >
    {/each}
    {#each placed.speeds as speed (speed.index)}
      <span class="label speed" class:hidden={!speed.visible} style:left={`${speed.x}px`} style:top={`${speed.y + speed.dy}px`}
        >{speed.text}</span
      >
    {/each}
  </div>
{/if}
{#if planets}
  <div class="cards" bind:clientWidth={viewWidth} bind:clientHeight={viewHeight}>
    {#each placedCards as card (card.planet.id)}
      {@const planet = card.planet}
      <PlanetCard
        {planet}
        {star}
        x={card.at.x}
        y={card.at.y}
        distance={card.at.distance}
        box={card.box}
        onSize={(width, height) => {
          const old = cardSizes[planet.id];
          if (!old || old.width !== width || old.height !== height) cardSizes[planet.id] = { width, height };
        }}
        selected={card.index === selected}
        pinned={card.pinned}
        onPin={() => onPin(planet.id)}
        onUnpin={() => {
          onUnpin(planet.id);
          hovered = null;
        }}
        onOpen={() => onOpenPlanet?.(card.index)}
        marks={card.marks}
        {onOpenMoon}
        onEnter={() => {
          overCard = true;
          clearTimeout(closeTimer);
        }}
        onLeave={() => {
          overCard = false;
          scheduleClose();
        }}
      />
    {/each}
    {#if pinned.length > 1}
      <button type="button" class="unpin-all" title={t.planetCard.unpinAllHelp} onclick={onUnpinAll}>{t.planetCard.unpinAll}</button>
    {/if}
  </div>
{/if}
</div>
{#snippet starPanel()}
  <p class="heading"><strong>{t.starView.heading({ type: star.type, kind: t.starView.classNames[star.type[0] ?? 'G'] ?? '' })}</strong></p>
  <dl>
    <dt title={t.starView.temperatureHelp}>{t.starView.temperature}</dt>
    <dd>{withUnit(formatInt(Math.round(star.teff_k / 10) * 10), 'K')}</dd>
    <dt title={t.starView.luminosityHelp}>{t.starView.luminosity}</dt>
    <dd class="unit-help" title={unitTooltip('L☉')}>{withUnit(formatUpTo(star.luminosity, star.luminosity < 0.01 ? 4 : 2), 'L☉')}</dd>
    <dt title={t.starView.sizeHelp}>{t.starView.size}</dt>
    <dd>
      {t.starView.sizeValue({
        radius: withUnit(formatUpTo(star.radius, 2), 'R☉'),
        mass: withUnit(formatUpTo(star.mass, 2), 'M☉'),
      })}
    </dd>
    <dt title={t.starView.metallicityHelp}>{t.starView.metallicity}</dt>
    <dd>
      {t.starView.metallicityValue({
        dex: `${star.metallicity >= 0 ? '+' : ''}${formatUpTo(star.metallicity, 2)}`,
        ratio: `${formatUpTo(10 ** star.metallicity, 2)}×`,
      })}
    </dd>
    <dt title={t.starView.lifetimeHelp}>{t.starView.lifetime}</dt>
    <dd>{withUnit(formatUpTo(star.lifetime_gyr, star.lifetime_gyr < 10 ? 1 : 0), t.starView.gyr)}</dd>
    <dt title={t.starView.ageHelp}>{t.starView.age}</dt>
    <dd>{formatAge(star.age_gyr)}</dd>
    <dt title={t.starView.zoneHelp}><span class="swatch zone"></span>{t.starView.zone}</dt>
    <dd>
      {t.starView.range({
        inner: formatAu(star.hz_inner_au),
        outer: formatAu(star.hz_outer_au),
      })}
    </dd>
  </dl>
  {#if selected !== null && !realScale}
    <small class="muted">{t.starView.systemScale}</small>
  {:else}
    <small class="muted">{t.starView.schematic}</small>
  {/if}
  {#if compressed && selected === null && giants}
    <small class="muted">{t.starView.compressed({ distance: formatAu(compressFrom) })}</small>
  {/if}
{/snippet}
{#snippet planetPanel()}
  {#if selected === -1}
    <p class="muted hint">{t.starView.pickHint}</p>
  {:else}
  <dl>
    <dt title={t.starView.orbitHelp}><span class="swatch orbit"></span>{t.worldInfo.orbit}</dt>
    <dd class="unit-help" title={unitTooltip('AU')}>{formatOrbit(star)}</dd>
    <dt title={t.starView.eccentricityHelp}>{t.worldInfo.eccentricity}</dt>
    <dd>{formatEccentricity(star)}</dd>
    <dt title={t.starView.speedHelp}>{t.starView.speed}</dt>
    <dd>{speedText}</dd>
    <dt>{t.worldInfo.year}</dt>
    <dd>{formatYear(star.year_days)}</dd>
    <dt title={t.starView.equilibriumHelp}>{t.starView.equilibrium}</dt>
    <dd>{withUnit(formatNumber(star.equilibrium_k - 273.15), '°C')}</dd>
    {#if selected !== null && selected >= 0 && planets?.[selected]?.surface_k != null}
      <dt>{t.starView.surfaceTemperature}</dt>
      <dd>{formatCelsius(planets[selected]!.surface_k!)}</dd>
    {/if}
  </dl>
  {#if distances && star.eccentricity <= 0.001}
    <!-- A circle has no closest or farthest point: say why none is marked. -->
    <small class="muted">{t.starView.circular({ distance: formatAu(star.orbit_au) })}</small>
  {/if}
  {/if}
{/snippet}
{#snippet planetsPanel()}
  <h4 title={t.starView.giantsHelp}>{kinds ? t.starView.planets : t.starView.giantsSection}</h4>
  {#if giantSystem === null}
    <small class="muted">{t.starView.giantsOff}</small>
  {:else}
    {#if giantList.length === 0}
      <small class="muted">{t.starView.noGiants}</small>
    {:else}
      <ul class="giants">
        {#each giantList as giant, i (i)}
          {@const id = planets?.[i]?.id}
          <li title={giantHelp(giant)} class:selected={i === selected}>
            {#if onSelectPlanet}
              <button
                type="button"
                class="pick"
                aria-current={i === selected}
                title={t.starView.selectPlanetHelp}
                onclick={() => onSelectPlanet?.(i)}
                ><span class="swatch" style:background={`rgb(${giant.colour.map((c) => Math.round(c * 255)).join(' ')})`}></span>{giantText(giant)}</button
              >
              {#if id}
                <button
                  type="button"
                  class="pin"
                  class:on={pinned.includes(id)}
                  aria-pressed={pinned.includes(id)}
                  title={t.starView.pinFromListHelp}
                  aria-label={t.starView.pinFromListHelp}
                  onclick={() => (pinned.includes(id) ? onUnpin(id) : onPin(id))}
                  ><svg viewBox="0 0 16 16" aria-hidden="true"
                    ><path d="M5 1h6l-1 5 3 3H9.5L8 15l-1.5-6H3l3-3z" fill={pinned.includes(id) ? 'currentColor' : 'none'} stroke="currentColor" stroke-linejoin="round" /></svg
                  ></button
                >
              {/if}
            {:else}
              <span class="swatch" style:background={`rgb(${giant.colour.map((c) => Math.round(c * 255)).join(' ')})`}></span>{giantText(giant)}
            {/if}
          </li>
        {/each}
      </ul>
    {/if}
    {#if selected !== null}
      <h4 title={t.starView.smallBodiesHelp}>{t.starView.smallBodies}</h4>
      {#if belts.length === 0}
        <small class="muted">{t.starView.noSmallBodies}</small>
      {/if}
      <ul class="small-bodies">
        {#each belts as belt, k (k)}
          {@const shaper = belt.shaped_by ? planets?.find((p) => p.id === belt.shaped_by) : undefined}
          <li>
            {t.starView.beltRow({
              name: beltName(belt),
              range: beltRange(belt),
              mass: formatUpTo(belt.mass_earth, belt.mass_earth < 0.01 ? 6 : 3),
              composition: t.starView.beltCompositions[belt.composition] ?? belt.composition,
            })}{#if shaper}<br /><small class="muted">{t.starView.beltShaped({ planet: shaper.name })}</small>{/if}{#if belt.gaps.length > 0}<br /><small
                class="muted">{t.starView.beltGaps({ gaps: belt.gaps.map((g) => withUnit(formatUpTo(g[0], 2), 'AU')).join(', ') })}</small
              >{/if}
          </li>
        {/each}
        {#each planets ?? [] as planet (planet.id)}
          {#if planet.trojans && planet.trojans.l4 + planet.trojans.l5 > 0}
            <li>{t.starView.trojansRow({ planet: planet.name, l4: formatInt(planet.trojans.l4), l5: formatInt(planet.trojans.l5) })}</li>
          {/if}
        {/each}
      </ul>
    {/if}
    <dl>
      <dt title={t.starView.forcedHelp}>{t.starView.forced}</dt>
      <dd>{formatUpTo(giantSystem.forced_eccentricity, 3)}</dd>
      <dt title={t.starView.impactsHelp}>{t.starView.impacts}</dt>
      <dd>{t.starView.impactsValue({ factor: `${formatUpTo(giantSystem.impact_factor, 2)}×` })}</dd>
    </dl>
  {/if}
{/snippet}
<div class="star-pane" class:concealed={hidden}>
<div class="star-chrome">
  <div class="gizmo"><AxesGizmo {axes} /></div>
  {#if onOpenStar}
    <button type="button" class="view-switch" title={t.starView.openStarHelp} onclick={onOpenStar}>{t.starView.openStar}</button>
  {/if}
  <div class="toggles">
    <ToggleBar label={t.starView.displaySection} groups={toggleGroups} />
  </div>
  <Dock kind="star" panels={dockPanels} defaultActive="star" />
  <PlaybackBar
    bind:playing={animated}
    speed={selected !== null
      ? {
          value: speed,
          min: SYSTEM_SPEED_MIN,
          max: SYSTEM_SPEED_MAX,
          text: t.starView.daysPerSecond({ days: formatInt(2 ** speed) }),
          help: t.starView.systemSpeedHelp,
          set: (v) => (speed = v),
        }
      : null}
    time={selected !== null && longestYear > 0
      ? {
          value: elapsedDays,
          max: Math.ceil(longestYear),
          text: t.starView.dayValue({ day: formatInt(Math.floor(elapsedDays) + 1) }),
          help: t.starView.timeBarHelp,
          set: goToDay,
        }
      : null}
    clock={clockText}
    jumps={selected !== null && selected >= 0
      ? [
          { key: 'start', label: t.starView.toStart, icon: 'start', run: () => goToDay(0) },
          {
            key: 'periapsis',
            label: t.starView.toPeriapsis({ name: giantList[selected]?.name ?? '' }),
            help: t.starView.toPeriapsisHelp,
            icon: 'periapsis',
            run: () => {
              const day = view?.nextPeriapsisDay(selected ?? 0);
              if (day !== null && day !== undefined) goToDay(day);
            },
          },
        ]
      : selected !== null
        ? [{ key: 'start', label: t.starView.toStart, icon: 'start', run: () => goToDay(0) }]
        : []}
  />
</div>
</div>

<style>
  /* The pane's parts are placed in the viewer: the wrapper only hides them. */
  .star-pane {
    display: contents;
  }
  .star-pane.concealed {
    display: none;
  }
  .view-switch {
    position: absolute;
    left: 12px;
    bottom: 72px;
    pointer-events: auto;
    /* Framed as the theme frames panels (see app.css and lib/themes.ts). */
    background: var(--frame-deco), var(--panel);
    clip-path: var(--frame-clip);
    box-shadow: var(--glow);
    padding: 4px 10px;
  }
  /* The playback bar wraps to two lines on narrow views. */
  @media (max-width: 720px) {
    .view-switch {
      bottom: 128px;
    }
  }
  .star-canvas {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    display: block;
    cursor: grab;
  }
  .star-canvas:active {
    cursor: grabbing;
  }
  .unpin-all {
    position: absolute;
    top: 12px;
    /* Left of the axes gizmo. */
    right: 88px;
    pointer-events: auto;
    font-size: 0.8rem;
  }
  .cards {
    position: absolute;
    inset: 0;
    pointer-events: none;
    overflow: hidden;
  }
  .labels {
    position: absolute;
    inset: 0;
    pointer-events: none;
    overflow: hidden;
  }
  .label {
    position: absolute;
    transform: translate(8px, -50%);
    padding: 1px 5px;
    border-radius: 4px;
    background: var(--label-bg);
    font-size: 0.75rem;
    white-space: nowrap;
  }
  .label.left {
    transform: translate(calc(-100% - 8px), -50%);
  }
  .label.planet {
    color: #ffd27a;
    white-space: normal;
    width: max-content;
    max-width: 170px;
  }
  .label.apsis {
    color: #ffe3a8;
  }
  .label.belt {
    color: #e2cfb4;
  }
  .label.belt.icy {
    color: #bcd8f2;
  }
  .label.swarm {
    color: #f0d590;
  }
  .label.small {
    transform: translate(6px, -100%);
  }
  .label.lagrange {
    color: #9ff0b8;
  }
  .label.lagrange.unstable {
    color: #ffb49c;
  }
  .small-bodies {
    margin: 0 0 8px;
    padding-left: 1.1em;
    font-size: 0.8rem;
  }
  .label.giant {
    color: var(--label-text);
    transform: translate(10px, -110%);
  }
  .label.self {
    color: #ffd27a;
  }
  .label.speed {
    color: #9fe0ff;
    transform: translate(-50%, -130%);
  }
  .label.hidden {
    opacity: 0;
  }
  /* Placed by the layout: its box is where it goes. */
  .label.giant.boxed {
    transform: none;
  }
  /* To scale: at a pin's tip, slanted up and away from it. */
  .label.giant.boxed.slanted {
    transform: translate(0, -100%) rotate(-35deg);
    transform-origin: left bottom;
  }
  .label.star-pin {
    color: #ffd27a;
  }
  .label.along {
    background: none;
    padding: 0 2px;
    text-shadow: 0 0 3px #05070d, 0 0 3px #05070d;
  }
  .label.along.distance {
    color: var(--overlay-line);
  }
  .label.along.pull {
    color: #ffa07a;
  }
  .label.along.zone {
    color: #8fe0aa;
  }
  .label.along.boundary {
    color: var(--overlay-muted);
  }
  .label.along.frost {
    color: #9cc8ff;
  }
  .giants {
    margin: 0;
    padding: 0;
    list-style: none;
    display: grid;
    gap: 2px;
  }
  .giants li {
    cursor: help;
  }
  .giants li.selected {
    font-weight: 600;
  }
  .giants li {
    display: flex;
    align-items: flex-start;
    gap: 4px;
  }
  .giants .pick {
    flex: 1;
    text-align: left;
    background: none;
    border: none;
    padding: 0;
    color: inherit;
    font: inherit;
    cursor: pointer;
  }
  .giants .pick:hover {
    text-decoration: underline;
  }
  .giants .pin {
    background: none;
    border: none;
    padding: 0 2px;
    width: 18px;
    height: 16px;
    color: #ffd27a;
    cursor: pointer;
    opacity: 0.35;
  }
  .giants .pin.on,
  .giants .pin:hover {
    opacity: 1;
  }
  .label.zone {
    color: #8fe0aa;
  }





  h4 {
    margin: 4px 0 0;
    font-size: 0.75rem;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--muted);
  }
  dt[title] {
    cursor: help;
  }




  .star-chrome {
    position: absolute;
    inset: 0;
    pointer-events: none;
  }
  .toggles {
    position: absolute;
    left: 12px;
    top: 72px;
    pointer-events: auto;
  }
  .gizmo {
    position: absolute;
    top: 4px;
    right: 12px;
  }
  .heading {
    margin: 0;
  }
  .hint {
    margin: 0;
  }
  dl {
    display: grid;
    /* As wide as the longest term in the language, never more than half. */
    grid-template-columns: fit-content(50%) 1fr;
    gap: 2px 10px;
    margin: 0;
  }
  dt {
    color: var(--muted);
  }
  dd {
    margin: 0;
  }
  .swatch {
    display: inline-block;
    width: 10px;
    height: 10px;
    margin-right: 6px;
    border-radius: 2px;
    vertical-align: -1px;
  }
  .swatch.zone {
    background: #3fbf6f;
    opacity: 0.6;
  }
  .swatch.orbit {
    background: #c8d2e6;
  }

  .muted {
    color: var(--muted);
  }
</style>
