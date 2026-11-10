<script lang="ts">
  import { errorText } from '../lib/i18n/errors';
  import { t } from '../lib/i18n/index.svelte';
  import { nightSources } from '../lib/nightLight';
  import { biomeName } from '../lib/i18n/names.svelte';
  import { onDestroy, onMount, untrack, type Snippet } from 'svelte';
  import Dock, { type DockPanel } from './dock/Dock.svelte';
  import PlaybackBar from './dock/PlaybackBar.svelte';
  import ToggleBar, { type Toggle } from './dock/ToggleBar.svelte';
  import AxesGizmo from './AxesGizmo.svelte';
  import SizeChips from './SizeChips.svelte';
  import { parseSize, pendingToFollow } from '../lib/sizes';
  import type { ScreenAxis } from '../lib/viewer/axes';
  import type { SurfaceMaps } from '../lib/skyBodies';
  import { ApiFailure, api, followJob, peeked, sizeName, systemBodyId, worldFile } from '../lib/api/client';
  import type { BiomeInfo } from '../lib/api/types/BiomeInfo';
  import type { JobStatus } from '../lib/api/types/JobStatus';
  import type { Stage } from '../lib/api/types/Stage';
  import type { HexMap } from '../lib/api/types/HexMap';
  import { linkedChanges, type LinkedDisplay } from '../lib/display';
  import { downloadJson } from '../lib/download';
  import { reliefScale } from '../lib/relief';
  import { formatArea, formatInt, formatKm, formatNumber, formatShare, formatUpTo, formatYear, saveBlob, surfaceColour, withUnit } from '../lib/format';
  import type { LonLat } from '../lib/geo';
  import type { ViewLink } from '../lib/link';
  import type { LoadProgress } from '../lib/progress';
  import { coastline } from '../lib/sample';
  import { seaIsLiquid } from '../lib/sea';
  import { retryWhileBusy } from '../lib/retry';
  import { debouncedSetting, loadSetting, saveSetting } from '../lib/settings';
  import { UNITS, unitTooltip } from '../lib/units';
  import { epochFile, epochRange, epochSteps, formatEpoch, nearestStep } from '../lib/epochs';
  import { altAllowed, RELIEF_DEFAULT, RELIEF_MAX, shortcutFor, shortcutHelp, stepRelief } from '../lib/shortcuts';
  import { orbitReadout } from '../lib/orbitClock';
  import { clockDays, seasonOfYearDay, yearDayAt } from '../lib/seasonClock';
  import { temperatureRange } from '../lib/temperatureRange';
  import { cloudTint, namedSeason, skyFromWorld, sunDeclinationDeg } from '../lib/sky';
  import { dustOf, innerRingOf } from '../lib/ringMoons';
  import { planetLook, type PlanetLook, type SkyBodyShow } from '../lib/skyBodies';
  import { inScope, localGroup, migrateDisplay, pressBody, pressScope, scopeOn, type ScopePair } from '../lib/switches';
  import { bodyTitle, forgetTold, remadeToasts } from '../lib/evicted';
  import { toasts } from '../lib/toasts.svelte';
  import { cssGradient } from '../lib/viewer/colormaps';
  import { starLitColour, vegetationShares } from '../lib/vegetation';
  import StarPane from './StarPane.svelte';
  import StarSurfacePane from './StarSurfacePane.svelte';
  import SkyLabels, { type SkyLabel } from './SkyLabels.svelte';
  import DialLabels from './DialLabels.svelte';
  import { bodyFileName } from '../lib/moons';
  import type { Moon } from '../lib/api/types/Moon';
  import { hillReach, hillSphere, smallBodiesSky, systemReach, systemSky } from '../lib/systemSky';
  import { dialFigureText, formatDialAngle, hostCompass as hostCompassOf, rotationDial } from '../lib/rotationDial';
  import { systemStarView } from '../lib/systemStar';
  import { SUN_SYSTEM, withOrbitDefaults } from '../lib/viewer/starSystem';
  import {
    type CameraState,
    giantTemperatureRange,
    PlanetView,
    SUN_STAR,
    vegetationShift,
    type ViewMode,
  } from '../lib/viewer/PlanetView';
  import { currentProfile } from '../lib/viewer/quality';
  import { giantStorms } from '../lib/giantStorms';
  import { RING_ROCK_SCALES, ringRocks } from '../lib/ringRocks';
  import type { Layer } from '../lib/viewer/shaders';
  import type { LoadedWorld } from '../lib/world.svelte';
  import LoadingBar from './LoadingBar.svelte';

  interface Props {
    world: LoadedWorld;
    /**
     * The system's revision (modification time): null while the world is
     * regenerated in place. The body's own files and tiles are keyed by the
     * revision of the resolution shown (`world.revision`).
     */
    revision: number | null;
    /** Measured path to draw. */
    path: readonly LonLat[];
    measuring: boolean;
    /** Whether this viewer answers keyboard shortcuts (one viewer at a time). */
    keyboard: boolean;
    /** The hex grid shown, if any (for the point information). */
    hexMap?: HexMap | null;
    onPick: (point: LonLat | null) => void;
    onHover: (point: LonLat | null) => void;
    onToggleMeasure: () => void;
    /** Escape: close what the pane shows over the viewer. */
    onEscape: () => void;
    onCameraChange?: (state: CameraState) => void;
    /** `[` / `]`: show the previous (-1) or next (1) world of the list. */
    onStepWorld?: (direction: 1 | -1) => void;
    /** Called with the linked display settings whenever one changes (the main pane). */
    onDisplayChange?: (display: LinkedDisplay) => void;
    /** Settings to mirror ("Link display", a compare pane), or null to keep its own. */
    linkedDisplay?: LinkedDisplay | null;
    /**
     * Whether display changes are saved as the preferences. Only the main
     * viewer saves them: a compare pane starts from them but would otherwise
     * overwrite them with its own.
     */
    persistDisplay?: boolean;
    /** The body's details (its card and world sheet), shown as the dock's first panel. */
    info?: Snippet | undefined;
    /** The drift slider of a world with plates (see DriftPanel), shown as a panel of the dock. */
    drift?: Snippet | undefined;
  }

  let {
    world,
    revision,
    path,
    measuring,
    keyboard,
    hexMap = $bindable(null),
    onPick,
    onHover,
    onToggleMeasure,
    onEscape,
    onCameraChange,
    onStepWorld,
    onDisplayChange,
    linkedDisplay = null,
    persistDisplay = true,
    info,
    drift,
  }: Props = $props();

  type Needs = 'none' | 'extra' | 'floodplain';
  const LAYERS: { id: Layer; needs: Needs }[] = [
    { id: 'biomes', needs: 'none' },
    { id: 'elevation', needs: 'none' },
    { id: 'temperature', needs: 'none' },
    { id: 'moisture', needs: 'none' },
    { id: 'flow', needs: 'none' },
    { id: 'volcanic', needs: 'extra' },
    { id: 'coast', needs: 'extra' },
    { id: 'floodplain', needs: 'floodplain' },
  ];
  /** One year of the season animation, in seconds. */
  const YEAR_SECONDS = 12;
  /** One orbit of the orbit animation, in seconds. */
  const ORBIT_SECONDS = 10;
  /**
   * The orbit animation's slowest and fastest speeds, powers of two (see
   * orbitSpeed): slower still, down to real time, for a short orbit (see
   * orbitSpeedMin).
   */
  const ORBIT_SPEED_MIN = -8;
  // Up to a year of seasons in seconds: the playback bar's one clock moves them too.
  const ORBIT_SPEED_MAX = 12;
  /** A time in days as the speed's rate shows it: in seconds, minutes, hours or days. */
  const simulatedSecond = (days: number): string =>
    days < 1 / 1440
      ? withUnit(formatUpTo(days * 86400, 1), t.viewer.secondUnit)
      : days < 1 / 24
        ? withUnit(formatUpTo(days * 1440, 1), t.viewer.minuteUnit)
        : days < 1
          ? withUnit(formatUpTo(days * 24, 1), t.viewer.hourUnit)
          : withUnit(formatUpTo(days, 1), t.viewer.dayUnit);
  /** mapgen::hex::DEFAULT_RIVER_FLOW_KM2, on the log10 slider. */
  const DEFAULT_RIVER_LOG = Math.log10(500_000);
  /** Wait before asking a server busy generating for the hex grid again, ms. */
  const HEX_RETRY_MS = 5000;

  /** Display preferences kept between visits. */
  const savedDisplay = loadSetting('display', {
    mode: 'globe' as string,
    layer: 'biomes' as string,
    relief: RELIEF_DEFAULT as number,
    /**
     * 3 once the relief became an exaggeration of the true relief (1): a
     * value saved before was on the old, arbitrary scale, so it gives way
     * to the default.
     */
    reliefVersion: 1 as number,
    graticule: false,
    referenceLines: true,
    contours: true,
    landContours: true,
    atmosphere: true,
    stars: true,
    // The planet in a moon's sky, by its own part of each switch (see lib/switches.ts).
    hostAtmosphere: true,
    hostClouds: true,
    hostCloudDrift: false,
    hostGrid: false,
    hostTropics: false,
    hostRings: true,
    hostSpinAxes: false,
    hostSpinDirection: false,
    // Orbits, labels and small bodies: the flag without a suffix is the star system's, `Local` the host planet's and its moons'.
    orbitLines: false,
    orbitLinesLocal: false,
    hillSphere: false,
    // The trojan swarms and belts ringed in the sky.
    smallBodies: false,
    smallBodiesLocal: false,
    // The rocks of the rings round a ring moon: 0 none, else their drawn
    // size over their true size (1 true; more is not to scale).
    ringRocks: 100 as number,
    // The spin axes of the globe and the bodies of the sky, their spin
    // arrows, and the link to the host planet (one switch before they split).
    spinAxes: false,
    spinDirection: false,
    hostLink: false,
    // The ring round the body shown, graduated in degrees, reading its turn.
    rotationDial: false,
    // The same round the host planet in a moon's sky, following the moon.
    hostCompass: false,
    // While the clock plays: the camera stays still against the sky, the
    // globe turning under it, or turns with the body (always the same
    // face, the sky turning). A new key: the former cameraFollowsSky
    // saved the other default.
    cameraOnBody: false,
    // The same for a body locked to what it orbits, on by default: its
    // planet, filling its sky, would otherwise sweep round it once per
    // orbit, as if the camera went round the body.
    cameraOnLockedBody: true,
    // Names tied to the bodies of the sky.
    skyLabels: true,
    skyLabelsLocal: true,
    starLight: false,
    clouds: true,
    cloudDrift: false,
    sun: false,
    season: 0.25,
    sunLongitude: 0,
    hexColumns: 64,
    hexRiverLog: DEFAULT_RIVER_LOG,
    legendOpen: true,
    panelOpen: true,
  });
  /** What was saved, with the switches that split since taking the value of the one they came from. */
  const saved = migrateDisplay(loadSetting<unknown>('display', null), savedDisplay);

  let canvas: HTMLCanvasElement;
  let view = $state<PlanetView | null>(null);
  let mode = $state<ViewMode>(saved.mode === 'map' ? 'map' : 'globe');
  let layer = $state<Layer>(LAYERS.some((l) => l.id === saved.layer) ? (saved.layer as Layer) : 'biomes');
  let relief = $state(saved.reliefVersion < 3 ? RELIEF_DEFAULT : Math.min(saved.relief, RELIEF_MAX));
  let graticule = $state(saved.graticule);
  let referenceLines = $state(saved.referenceLines);
  // `contours` keeps its saved name: it was the only (depth) contour toggle.
  let seaContours = $state(saved.contours);
  let landContours = $state(saved.landContours);
  let atmosphere = $state(saved.atmosphere);
  let stars = $state(saved.stars);
  let starAnimated = $state(false);
  let starDistances = $state(false);
  let starOrbit = $state(true);
  let starZone = $state(true);
  let starProminences = $state(true);
  /** Whether the star view draws the star through its warm display filter (true colour by default). */
  let starFilter = $state(false);
  /**
   * The day the system view and the star view show, Earth days from the
   * system's t = 0: both panes bind it, so switching keeps the day. 0 with
   * each new system; opening the star from a planet starts at the planet
   * view's day (sysDays). The planet view's own clock is left as it is.
   */
  let starDays = $state(0);
  let starDaysFor: { system: string | null; subject: 'planet' | 'star' } | null = null;
  $effect(() => {
    const system = world.systemId;
    const subject = world.subject;
    untrack(() => {
      const before = starDaysFor;
      starDaysFor = { system, subject };
      if (!before || before.system !== system) starDays = 0;
      else if (subject === 'star' && before.subject === 'planet') starDays = sysDays;
    });
  });
  let starGiants = $state(true);
  /** A planetary system's dotted star–planet lines and velocity vectors in the star view. */
  let starLinks = $state(true);
  let starVelocities = $state(true);
  let starPulls = $state(true);
  let starFrost = $state(true);
  let starHills = $state(false);
  let starBelts = $state(true);
  let starTrojans = $state(true);
  let starLagrange = $state(false);
  let starRealScale = $state(false);
  let starPane = $state<ReturnType<typeof StarPane> | null>(null);
  let starSurfacePane = $state<ReturnType<typeof StarSurfacePane> | null>(null);
  let starLight = $state(saved.starLight);
  let clouds = $state(saved.clouds);
  /** The planet seen in a moon's sky (or the giant in its moon world's), drawn by its own switches. */
  let hostAtmosphere = $state(saved.hostAtmosphere);
  let hostClouds = $state(saved.hostClouds);
  let hostCloudDrift = $state(saved.hostCloudDrift);
  let hostGrid = $state(saved.hostGrid);
  let hostTropics = $state(saved.hostTropics);
  let hostRingsShown = $state(saved.hostRings);
  let hostAxes = $state(saved.hostSpinAxes);
  let hostSpinDirection = $state(saved.hostSpinDirection);
  /** The orbits of the moons (or of a giant's other moons) drawn in the sky. */
  let orbitLines = $state(saved.orbitLines);
  let orbitLinesLocal = $state(saved.orbitLinesLocal);
  /** The Hill sphere of the body shown (its host planet's from a moon) drawn in the sky. */
  let hillOn = $state(saved.hillSphere);
  let smallBodies = $state(saved.smallBodies);
  let smallBodiesLocal = $state(saved.smallBodiesLocal);
  let ringRockScale = $state(RING_ROCK_SCALES.includes(saved.ringRocks) ? saved.ringRocks : 100);
  /** The body shown's spin axis (and the sky bodies' when it has no host), its spin arrow, and the link to its host planet. */
  let axesShown = $state(saved.spinAxes);
  let spinDirection = $state(saved.spinDirection);
  let hostLink = $state(saved.hostLink);
  let dialShown = $state(saved.rotationDial);
  let hostCompassShown = $state(saved.hostCompass);
  let cameraOnBody = $state(saved.cameraOnBody);
  let cameraOnLockedBody = $state(saved.cameraOnLockedBody);
  $effect(() => view?.setSpinAxes(axesShown));
  $effect(() => view?.setSpinDirection(spinDirection));
  $effect(() => view?.setHostLink(hostLink));
  /** Labels on the bodies of the sky, and where those are on the screen. */
  let skyLabels = $state(saved.skyLabels);
  let skyLabelsLocal = $state(saved.skyLabelsLocal);
  let skyPositions = $state<ReturnType<PlanetView['skyScreenPositions']> | null>(null);
  let cloudDrift = $state(saved.cloudDrift);
  let sun = $state(saved.sun);
  let season = $state(saved.season);
  let sunLongitude = $state(saved.sunLongitude);
  let playing = $state(false);
  /** Days the moons have moved on along their orbits past the season. */
  let orbitDays = $state(0);
  /** The orbit time of a link being opened, for its world (see applyView). */
  let orbitLink: number | null = null;
  /** The world the orbit time belongs to: another one starts its moons over. */
  let orbitWorld: string | null = null;
  let orbitsPlaying = $state(false);
  /** The orbit animation's speed, a power of two of one orbit every ORBIT_SECONDS. */
  let orbitSpeed = $state(0);
  /** Whether the canvas has a size: hidden (narrow screens showing the list), nothing animates. */
  let canvasVisible = $state(true);
  let hexOn = $state(false);
  let hexColumns = $state(saved.hexColumns);
  let hexRiverLog = $state(saved.hexRiverLog);
  let hexBusy = $state(false);
  /** Whether the server answered that it is busy generating: the request is retried. */
  let hexServerBusy = $state(false);
  // Narrow screens fold the legend and the display panel, which would cover
  // the planet; wide ones open them as last chosen on a wide screen. Both
  // follow the window as it is resized across the limit.
  const wideQuery = window.matchMedia('(min-width: 900px)');
  let wide = $state(wideQuery.matches);
  /** Whether the icon bar shows its names (it widens, and the legend steps right of it). */
  let barExpanded = $state(loadSetting('toggles.expanded', false));
  /** The legend and panel states chosen on a wide screen, saved as the preferences. */
  let wideChoice = { legendOpen: saved.legendOpen, panelOpen: saved.panelOpen };
  let legendOpen = $state(saved.legendOpen && wideQuery.matches);
  /**
   * Where the legend card was dragged: its left edge and its bottom edge
   * from the view's left and bottom, CSS px (it grows upwards as it opens);
   * null for its usual place. Remembered; a double click puts it back.
   */
  let legendAt = $state(loadSetting<{ left: number; bottom: number } | null>('legendAt', null));
  /** Whether the press on the legend turned into a drag: the click ending it toggles nothing. */
  let legendDragged = false;

  /**
   * Drags the legend card with the mouse or a pen, from anywhere on it
   * (its rows too): a press that moves 5 px is a drag, else a click. A
   * finger on its list scrolls the list instead.
   */
  function dragLegend(event: PointerEvent): void {
    if (event.button !== 0) return;
    if (event.pointerType === 'touch' && (event.target as HTMLElement).closest('ul')) return;
    const card = event.currentTarget as HTMLElement;
    const view = card.offsetParent as HTMLElement | null;
    if (!view) return;
    const rect = card.getBoundingClientRect();
    const box = view.getBoundingClientRect();
    const start = { left: rect.left - box.left, bottom: box.bottom - rect.bottom };
    const from = { x: event.clientX, y: event.clientY };
    legendDragged = false;
    const move = (e: PointerEvent) => {
      const dx = e.clientX - from.x;
      const dy = e.clientY - from.y;
      if (!legendDragged) {
        if (Math.hypot(dx, dy) < 5) return;
        legendDragged = true;
      }
      // Kept inside the view, its title always reachable.
      legendAt = {
        left: Math.min(Math.max(start.left + dx, 8), Math.max(8, box.width - rect.width - 8)),
        bottom: Math.min(Math.max(start.bottom - dy, 8), Math.max(8, box.height - 48)),
      };
    };
    // Followed on the window: the pointer leaves the card before the drag
    // starts (capturing it would send the click to the card, not its buttons).
    const end = () => {
      window.removeEventListener('pointermove', move);
      window.removeEventListener('pointerup', end);
      window.removeEventListener('pointercancel', end);
      if (legendDragged) saveSetting('legendAt', legendAt);
    };
    window.addEventListener('pointermove', move);
    window.addEventListener('pointerup', end);
    window.addEventListener('pointercancel', end);
  }

  /** The click that ends a drag of the legend does not reach its buttons. */
  function legendClick(event: MouseEvent): void {
    if (!legendDragged) return;
    legendDragged = false;
    event.stopPropagation();
    event.preventDefault();
  }

  /** A double click on the legend (off its buttons) puts it back in its usual place. */
  function legendHome(event: MouseEvent): void {
    if ((event.target as HTMLElement).closest('button')) return;
    legendAt = null;
    saveSetting('legendAt', null);
  }
  let hovered = $state<number | null>(null);
  let pinned = $state<number | null>(null);
  let loading = $state(false);
  /** The base map's download and decoding, while it loads. */
  let mapProgress = $state.raw<LoadProgress | null>(null);
  let error = $state<string | null>(null);
  let showHelp = $state(false);
  let panelOpen = $state(saved.panelOpen && wideQuery.matches);

  $effect(() => {
    const change = (event: MediaQueryListEvent) => {
      wide = event.matches;
      legendOpen = wide && wideChoice.legendOpen;
      panelOpen = wide && wideChoice.panelOpen;
    };
    wideQuery.addEventListener('change', change);
    return () => wideQuery.removeEventListener('change', change);
  });

  /**
   * The epoch chosen on the time slider, Ma before present (negative in the
   * future): kept from world to world, each one showing the nearest of its
   * own epochs (the present for a world without any).
   */
  let epochChoice = $state(0);
  /** The epoch under the slider while it is dragged, previewed at a quarter of the resolution. */
  let epochPreview = $state<number | null>(null);

  const worldId = $derived(world.id);
  /** The resolution shown (`WxH`) and its revision, which key its files and tiles. */
  const size = $derived(world.size);
  const sizeRevision = $derived(world.revision);
  const meta = $derived(world.meta);
  /** `U`: each body of the family of the planet shown in turn (the star aside, on `O`). */
  function stepBody(): void {
    if (!sys || !sysBody) return;
    const planet = sysBody.parent ?? sysBody.id;
    const family = sys.bodies.filter((b) => b.id === planet || b.parent === planet);
    const at = family.findIndex((b) => b.id === sysBody.id);
    world.subject = 'planet';
    world.showSystemBody(family[(at + 1) % family.length]!.id);
  }
  /** The body picker's choice: the system view, the star view, or a body of the system. */
  function pickBody(key: string): void {
    if (key === 'system' || key === 'star') {
      if (!sys) return;
      toStar();
      if (key === 'star') world.starMode = 'star';
      return;
    }
    // From the star view, the camera flies to the planet (a moon's planet) first.
    const body = sys?.bodies.find((b) => b.id === key);
    const index = sysStar && body ? sysStar.ids.indexOf(body.parent ?? body.id) : -1;
    const open = () => {
      world.subject = 'planet';
      world.showSystemBody(key);
    };
    if (world.subject === 'star' && world.starMode === 'system' && starPane && index >= 0) starPane.flyToPlanet(index, open);
    else open();
  }
  // Only planets have a hex grid.
  $effect(() => {
    if (sysBody?.kind !== 'planet') hexOn = false;
  });
  const epochs = $derived(meta ? epochRange(meta) : null);
  /** The slider's stops, from the past (left) to the future (right). */
  const epochStops = $derived(epochs ? epochSteps(epochs).reverse() : []);
  const epoch = $derived(epochs ? nearestStep(epochStops, epochChoice) : 0);
  const epochShown = $derived(epochPreview ?? epoch);
  const layers = $derived(world.layers);
  const dataReady = $derived(layers !== null);
  /** A giant planet: cloud bands, no ground (see GiantInfo). */
  const giant = $derived(meta?.giant ?? null);
  /**
   * The °C range of the temperature colours, for the globe, its detail tiles
   * and the legend alike: a giant's cloud tops', else the world's own when it
   * leaves the Earth-like -30 to 40 °C.
   */
  const tempRange = $derived<[number, number]>(
    giant ? giantTemperatureRange(giant) : temperatureRange(layers?.temperatureSpan),
  );
  /** The layers a giant has: its bands, cloud tops, temperatures and winds (on the moisture layer). */
  const GIANT_LAYERS: readonly Layer[] = ['biomes', 'elevation', 'temperature', 'moisture'];
  /** A layer's name: a giant's cloud tops and winds are named as such. */
  function layerName(id: Layer): string {
    if (giant && id === 'elevation') return t.viewer.cloudTops;
    if (giant && id === 'moisture') return t.viewer.windLayer;
    return t.viewer.layers[id];
  }
  function available(needs: Needs, id?: Layer): boolean {
    if (giant && id !== undefined && !GIANT_LAYERS.includes(id)) return false;
    return (
      needs === 'none' ||
      (needs === 'extra' && layers?.extra != null) ||
      (needs === 'floodplain' && layers?.extra != null && meta?.layers.floodplain != null)
    );
  }
  /** The layer drawn: the chosen one when this world has it, the biomes otherwise. */
  const shown = $derived.by((): Layer => {
    const option = LAYERS.find((l) => l.id === layer);
    return dataReady && option && available(option.needs, option.id) ? layer : 'biomes';
  });
  /**
   * The season as the panel names it: the equinox or solstice when the sun
   * truly is there, else the day of the year (the declination is defined
   * further down, from the same clock the sun is drawn with).
   */
  const seasonLabel = $derived.by(() => {
    // A moon's sun latitude follows the planet's clock but the moon's own
    // axis: only a planet (or a lone world) gets the solstices named.
    const named = sysBody?.parent ? null : namedSeason(season, declination, meta?.axial_tilt_deg ?? 23.44);
    if (named !== null) return t.viewer.seasons[named] ?? '';
    // In a system the day is the planet's own, counted from its spring equinox.
    return t.viewer.dayOfYear({ day: yearDay !== null ? Math.floor(yearDay) : Math.round(season * 365) % 365 });
  });
  const highlight = $derived(hovered ?? pinned);

  // Debounced: the season animation and the sliders change these every frame.
  const display = debouncedSetting<typeof saved>('display', 300);
  $effect(() => {
    if (!persistDisplay) {
      return;
    }
    // Folded for lack of room is not a choice: only wide screens change it.
    if (wide) {
      wideChoice = { legendOpen, panelOpen };
    }
    display.save({
      mode,
      layer,
      relief,
      reliefVersion: 3,
      graticule,
      referenceLines,
      contours: seaContours,
      landContours,
      atmosphere,
      stars,
      starLight,
      clouds,
      cloudDrift,
      hostAtmosphere,
      hostClouds,
      hostCloudDrift,
      hostGrid,
      hostTropics,
      hostRings: hostRingsShown,
      hostSpinAxes: hostAxes,
      hostSpinDirection,
      orbitLines,
      orbitLinesLocal,
      hillSphere: hillOn,
      smallBodies,
      smallBodiesLocal,
      ringRocks: ringRockScale,
      spinAxes: axesShown,
      spinDirection,
      hostLink,
      rotationDial: dialShown,
      hostCompass: hostCompassShown,
      cameraOnBody,
      cameraOnLockedBody,
      skyLabels,
      skyLabelsLocal,
      sun,
      season,
      sunLongitude,
      hexColumns,
      hexRiverLog,
      ...wideChoice,
    });
  });

  /** The settings "Link display" mirrors, as this pane has them. */
  function currentLinked(): LinkedDisplay {
    return { layer, relief, sun, season, sunLongitude, atmosphere, stars, starLight, clouds, graticule };
  }

  $effect(() => {
    const current = currentLinked();
    onDisplayChange?.(current);
  });

  // A compare pane with "Link display": follow the main pane's settings.
  $effect(() => {
    const source = linkedDisplay;
    if (!source) {
      return;
    }
    const changes = untrack(() => linkedChanges(currentLinked(), source));
    if (changes.layer !== undefined && LAYERS.some((l) => l.id === changes.layer)) layer = changes.layer as Layer;
    if (changes.relief !== undefined) relief = changes.relief;
    if (changes.sun !== undefined) sun = changes.sun;
    if (changes.season !== undefined) season = changes.season;
    if (changes.sunLongitude !== undefined) sunLongitude = changes.sunLongitude;
    if (changes.atmosphere !== undefined) atmosphere = changes.atmosphere;
    if (changes.stars !== undefined) stars = changes.stars;
    if (changes.starLight !== undefined) starLight = changes.starLight;
    if (changes.clouds !== undefined) clouds = changes.clouds;
    if (changes.graticule !== undefined) graticule = changes.graticule;
  });

  /** A camera to show (a link's, a linked pane's), kept until the view exists. */
  let pendingCamera: CameraState | null = null;

  onMount(() => {
    try {
      view = new PlanetView(canvas, currentProfile());
      if (pendingCamera) {
        view.setCamera(pendingCamera);
        pendingCamera = null;
      }
      view.onPick = (point) => onPick(point);
      view.onAxes = (next) => (planetAxes = next);
      view.onHover = (point) => onHover(point);
      view.onCameraChange = (state) => onCameraChange?.(state);
      canvasVisible = view.visible;
      view.onVisibilityChange = (visible) => (canvasVisible = visible);
      // A detail tile of the resolution shown answered 404: deleted elsewhere, maybe.
      view.onDetailMissing = () => world.fileMissing();
    } catch (e) {
      error = t.viewer.noWebgl({ error: String(e) });
    }
  });

  onDestroy(() => {
    display.flush();
    view?.dispose();
  });

  /** The world whose base map is shown (or loading). */
  let textureOf: string | null = null;

  // The base map, reloaded when the world or its revision changes (a world
  // regenerated under the same id). A newer load cancels this one: its
  // result and its errors are dropped.
  $effect(() => {
    const current = view;
    const id = worldId;
    const shownSize = size;
    const rev = sizeRevision;
    // No revision: the world left the list, typically while it is being
    // regenerated in place. Keep its map until the new one is listed.
    if (revision === null && id !== null && id === textureOf) {
      loading = false;
      return;
    }
    // The same body at another resolution, listed in a moment: keep its map
    // (and the point picked, the hexes) meanwhile.
    if (id !== null && id === textureOf && shownSize === null) {
      return;
    }
    if (id !== textureOf) {
      pinned = null;
      hovered = null;
      hexOn = false;
    }
    if (!current || !id) {
      textureOf = null;
      loading = false;
      return;
    }
    textureOf = id;
    loading = true;
    mapProgress = null;
    error = null;
    // A new body: its map loads once its resolution is listed.
    if (shownSize === null) {
      return;
    }
    const controller = new AbortController();
    const progress = (p: LoadProgress) => {
      if (!controller.signal.aborted) mapProgress = p;
    };
    // The render without lines, which the reference lines and contours are
    // drawn over. At another epoch, its render (a world shown at
    // the same epoch as the one before).
    const epochMa = untrack(() => epoch);
    textureEpoch = { id, epochMa, scale: 1 };
    const maps = epochMaps(id, shownSize, epochMa, rev, 1);
    current
      .setMaps(maps.colour, maps.normal, controller.signal, progress)
      .catch((e: unknown) => {
        if (controller.signal.aborted) return;
        error = t.viewer.mapFailed({ error: String(e) });
        // Its resolution deleted elsewhere, maybe: the world lists them again.
        world.fileMissing();
      })
      .finally(() => {
        if (!controller.signal.aborted) {
          loading = false;
          mapProgress = null;
        }
      });
    return () => controller.abort();
  });

  $effect(() => {
    view?.setData(layers, meta ? coastline(meta) : 0, meta ? seaIsLiquid(meta) : true, meta ? reliefScale(meta.circumference_km) : 0);
  });
  // The layers (relief, contours, picking) follow the epoch once the slider is released.
  $effect(() => {
    const epochMa = epoch;
    // Untracked: the world's own epoch state must not re-run this (a failed
    // load would be retried at once, again and again).
    untrack(() => world.setEpoch(epochMa));
  });
  $effect(() => {
    if (world.epochError) toasts.error(world.epochError);
  });

  /** The base map shown: world, epoch and reduction. */
  let textureEpoch = { id: null as string | null, epochMa: 0, scale: 1 };
  /** Whether the base map of another epoch is loading. */
  let epochMapLoading = $state(false);

  /**
   * URLs of the maps of a world at an epoch: its colour (without lines or
   * hillshade) and its relief normals, stored at the present, rendered by
   * the server at another epoch.
   */
  function epochMaps(id: string, size: string, epochMa: number, rev: number | null, scale: 1 | 4): { colour: string; normal: string } {
    if (epochMa === 0) {
      return { colour: worldFile(id, size, 'raw_data/colour.png', rev), normal: worldFile(id, size, 'raw_data/normal.png', rev) };
    }
    return {
      colour: epochFile(id, size, epochMa, 'colour.png', rev, scale),
      normal: epochFile(id, size, epochMa, 'normal.png', rev, scale),
    };
  }

  // The base map of another epoch of the world shown: reduced while the
  // slider moves (after a short pause), full once it is released.
  $effect(() => {
    const current = view;
    const id = worldId;
    const shownSize = size;
    const rev = sizeRevision;
    const epochMa = epochShown;
    const scale: 1 | 4 = epochPreview === null ? 1 : 4;
    if (!current || !id || !shownSize || id !== textureOf || loading) {
      return;
    }
    if (textureEpoch.id === id && textureEpoch.epochMa === epochMa && textureEpoch.scale === scale) {
      return;
    }
    const controller = new AbortController();
    const show = () => {
      epochMapLoading = true;
      const maps = epochMaps(id, shownSize, epochMa, rev, scale);
      current
        .setMaps(maps.colour, maps.normal, controller.signal)
        .then(() => {
          textureEpoch = { id, epochMa, scale };
        })
        .catch((e: unknown) => {
          if (controller.signal.aborted) return;
          toasts.error(t.epochs.failed({ error: String(e) }));
          world.fileMissing();
        })
        .finally(() => {
          if (!controller.signal.aborted) epochMapLoading = false;
        });
    };
    const timer = setTimeout(show, scale === 4 ? 120 : 0);
    return () => {
      clearTimeout(timer);
      controller.abort();
    };
  });

  /** Moves the time slider `steps` stops towards the past (negative: the future). */
  function stepEpoch(steps: number): void {
    if (!epochs) return;
    const i = epochStops.indexOf(epoch);
    // The stops run from the past to the future.
    const next = Math.min(epochStops.length - 1, Math.max(0, i - steps));
    epochChoice = epochStops[next] ?? 0;
  }
  // The cloud layer of worlds that have one.
  $effect(() => {
    const current = view;
    const id = worldId;
    const file = meta?.layers.clouds?.file;
    if (!current) {
      return;
    }
    const controller = new AbortController();
    const url = id && size && file ? worldFile(id, size, `raw_data/${file}`, sizeRevision) : null;
    current
      .setClouds(url, meta ? cloudTint(meta) : [1, 1, 1], controller.signal)
      .catch((e: unknown) => {
        if (controller.signal.aborted) return;
        console.warn('Could not load the clouds', e);
        if (url) world.fileMissing();
      });
    return () => controller.abort();
  });
  // An irregular moon's shape, once per body.
  $effect(() => {
    const id = worldId;
    const shownSize = size;
    const shape = meta?.layers.shape;
    if (!view) return;
    const controller = new AbortController();
    view
      .setShape(
        id && shownSize && shape ? { url: worldFile(id, shownSize, `raw_data/${shape.file}`, sizeRevision), min: shape.min, max: shape.max } : null,
        controller.signal,
      )
      .catch((e: unknown) => {
        if (controller.signal.aborted) return;
        console.warn('Could not load the shape', e);
        if (id && shownSize && shape) world.fileMissing();
      });
    return () => controller.abort();
  });
  $effect(() => view?.setCloudsVisible(clouds));
  $effect(() => view?.setCloudsDrifting(cloudDrift));
  $effect(() => view?.setGiant(giant));
  $effect(() => view?.setTemperatureRange(tempRange[0], tempRange[1]));
  // A giant's storms, found in its biome map, each drifting whole.
  $effect(() => {
    const stormId = giant && meta ? meta.biomes.find((b) => b.key === 'GiantStorm')?.id : undefined;
    view?.setGiantStorms(stormId !== undefined && layers ? giantStorms(layers.biomes, layers.width, layers.height, stormId) : []);
  });
  // The dust rings its system drew for the giant shown.
  $effect(() => view?.setDustRings(giant && sysBody ? [...dustOf(sysBody), ...innerRingOf(sysBody)] : []));
  $effect(() => {
    // No detail while the slider moves: the reduced preview shows through.
    // A giant has no ground to detail.
    view?.setDetail(
      worldId && size && meta && !giant && sizeRevision !== null && epochPreview === null
        ? { id: worldId, size, revision: sizeRevision, baseWidth: meta.width, epochMa: epoch }
        : null,
    );
  });
  $effect(() => view?.setMode(mode));
  $effect(() => view?.setLayer(shown));
  $effect(() => view?.setRelief(dataReady ? relief : 0));
  $effect(() => view?.setGraticule(graticule));
  $effect(() => view?.setReferenceTilt(meta ? meta.axial_tilt_deg : null));
  $effect(() => view?.setReferenceLines(referenceLines));
  // Cloud tops have no sea and no altitude contours.
  $effect(() => view?.setSeaContours(seaContours && !giant));
  $effect(() => view?.setLandContours(landContours && !giant));
  $effect(() => view?.setAtmosphere(atmosphere));
  $effect(() => view?.setStars(stars));
  $effect(() => view?.setStarLight(starLight));
  /**
   * A biome's colour in the legend, as the globe shows it: under "Star
   * light", recoloured for the star's vegetation and lit by its light.
   */
  function legendColour(biome: BiomeInfo): string {
    const base = hexOn ? biome.color : surfaceColour(biome);
    const star = meta?.star;
    if (!starLight || !star) return base;
    // The hex grid shows the classification colours, without the vegetation.
    const plants = hexOn ? 0 : (vegetationShares([biome])[biome.id] ?? 0);
    return starLitColour(base, plants, vegetationShift(star.teff_k), star.light);
  }
  $effect(() => {
    const biomes = meta?.biomes;
    if (biomes) view?.setVegetation(vegetationShares(biomes));
  });
  $effect(() => {
    const star = meta?.star;
    view?.setHostStar(
      star
        ? {
            colour: star.light,
            angularDiameterDeg: star.angular_diameter_deg,
            insolation: star.insolation,
            teffK: star.teff_k,
          }
        : SUN_STAR,
    );
  });
  $effect(() => view?.setSky(meta ? skyFromWorld(meta) : null));
  $effect(() => view?.setHighlight(dataReady ? highlight : null));
  $effect(() => view?.setPath(path));
  /** A tidally locked world: its star stays above the map's centre (longitude 180°, the equator). */
  const locked = $derived(meta?.star?.tidally_locked ?? false);
  $effect(() => {
    const id = world.systemId;
    untrack(() => {
      if (id === orbitWorld) return;
      orbitWorld = id;
      orbitDays = orbitLink ?? 0;
      orbitsPlaying = false;
    });
  });
  // A planetary system (docs/system-spec.md): every body placed from the
  // orbits and spins its file stores (lib/systemSky.ts), on a clock of
  // days: the season of the shown body's planet and the orbit time past
  // it; the Sun longitude slider sets the hour of the body shown.
  const sys = $derived(world.system);
  const sysBody = $derived(world.shownSystemBody);
  /**
   * The original resolution of a body of the system shown (`WxH`, its
   * params' size): always on disk once it is generated, so the other
   * bodies' maps (sky, star view) need no listing of their resolutions.
   */
  const originalSize = (body: string): string | null => {
    const params = sys?.bodies.find((b) => b.id === body)?.params;
    return params ? sizeName(params.width, params.height) : null;
  };
  /** The planet of the body shown (itself, or the planet of a moon). */
  const sysPlanet = $derived(sys && sysBody ? (sys.bodies.find((b) => b.id === (sysBody.parent ?? sysBody.id)) ?? null) : null);
  /** Whether the body shown is a moon, its host planet then in its sky: the switches then have a part for each. */
  const hasHost = $derived(Boolean(sysBody?.parent));
  /** The local group of the body shown: its planet and the moons of its planet (itself left out). */
  const localIds = $derived(sys && sysBody ? localGroup(sys.bodies, sysBody.id) : new Set<string>());
  /** The system-wide switches' parts: the squares show only when there is a local group to tell from the rest. */
  const hasLocalGroup = $derived(localIds.size > 0);
  const labelScope = $derived<ScopePair>({ local: skyLabelsLocal, stellar: skyLabels });
  const orbitScope = $derived<ScopePair>({ local: orbitLinesLocal, stellar: orbitLines });
  const smallScope = $derived<ScopePair>({ local: smallBodiesLocal, stellar: smallBodies });
  /** The star view of a planetary system (lib/systemStar.ts). */
  /**
   * The planet picked in the star view (a click on it, or its row), by id:
   * none when a system opens; a second click on it lets it go.
   */
  let starPick = $state<string | null>(null);
  $effect(() => {
    void world.systemId;
    untrack(() => (starPick = null));
  });
  const pickedPlanet = $derived(sys && starPick ? (sys.bodies.find((b) => b.id === starPick && b.kind === 'planet') ?? null) : null);
  /** The planet the star view's orbit facts describe: the one picked, else any (nothing is then selected). */
  const starBase = $derived(pickedPlanet ?? sysPlanet ?? sys?.bodies.find((b) => b.kind === 'planet') ?? null);
  const sysStar = $derived(
    sys && starBase ? systemStarView(sys, starBase, world.systemNames, t.viewer.bodyTypes as Record<string, string>) : null,
  );
  /** Picks planet `index` of the star view, or lets it go if it is the one picked: its card pinned or unpinned with it. */
  function togglePick(index: number): void {
    const id = sysStar?.ids[index];
    if (!id) return;
    if (starPick === id) {
      starPick = null;
      world.pinnedPlanets = world.pinnedPlanets.filter((p) => p !== id);
    } else {
      starPick = id;
      if (!world.pinnedPlanets.includes(id)) world.pinnedPlanets = [...world.pinnedPlanets, id];
    }
  }
  /** Opens planet `index` of the star view (in sysStar's order): its maps, or its generation. */
  const openSystemPlanet = (index: number) => {
    const id = sysStar?.ids[index];
    if (!id) return;
    // The travelling: the camera flies to the planet, then its view opens.
    const open = () => {
      world.subject = 'planet';
      world.showSystemBody(id);
    };
    if (world.subject === 'star' && starPane) starPane.flyToPlanet(index, open);
    else open();
  };
  /** The planet the star view flies out from when it opens again (the travelling back), by index. */
  let flyBack = $state<number | null>(null);
  /** Shows the star view, flying out from the planet shown. */
  function toStar(): void {
    // Back from a planet: it is the one picked, and the camera flies out from it.
    starPick = sysPlanet?.id ?? null;
    const index = sysStar && sysPlanet ? sysStar.ids.indexOf(sysPlanet.id) : -1;
    flyBack = index >= 0 ? index : null;
    world.starMode = 'system';
    world.subject = 'star';
  }
  /**
   * The maps of each generated planet of the star view, for its marker: the
   * small copies of its colour and its relief normals, from its original
   * resolution, at the system's revision.
   */
  const sysMaps = $derived(
    sysStar && world.systemId
      ? sysStar.planets.map((p): SurfaceMaps | null => {
          const original = originalSize(p.id);
          if (!p.generated || !original) return null;
          const id = systemBodyId(world.systemId!, p.id);
          return {
            // Seen from the star view, not opened: a peek.
            colour: peeked(worldFile(id, original, 'raw_data/colour_small.png', revision)),
            normal: peeked(worldFile(id, original, 'raw_data/normal_small.png', revision)),
          };
        })
      : null,
  );
  /** What the star view shows: the system round the planet shown (or the moon's), the Sun's before one loads. */
  const system = $derived(
    sysStar
      ? { star: sysStar.star, omega: sysStar.omega, giants: sysStar.giants }
      : { star: withOrbitDefaults(SUN_SYSTEM), omega: 0, giants: null },
  );
  /** The moons of the planet shown, for the star view. */
  const sysMoons = $derived(
    sys && sysPlanet
      ? sys.bodies
          .filter((b) => b.parent === sysPlanet.id)
          .map((b) => ({
            radius_km: b.physical.radius_km,
            inclination_deg: b.orbit.inclination_deg,
            period_days: b.orbit.period_days,
            barycentre_km: 0,
            semi_major_axis_km: b.orbit.semi_major_axis_km,
          }))
      : null,
  );
  /**
   * The clock, days from the system's epoch: the planet's place on its orbit
   * at the season (the quarters are its equinoxes and solstices, see
   * lib/seasonClock.ts), and the orbit time past it.
   */
  const sysDays = $derived(sysPlanet ? clockDays(sysPlanet.orbit, season, orbitDays) : orbitDays);
  /** The day of the planet's year the clock shows: days since its northern spring equinox. */
  const yearDay = $derived(sysPlanet ? yearDayAt(sysPlanet.orbit, sysDays) : null);
  /** The maps of a body of the system, for the sky: its original resolution's. Peeks: the body is not opened. */
  const sysTextures = (id: string): SurfaceMaps | null => {
    const original = originalSize(id);
    if (!world.systemId || !original) return null;
    const body = systemBodyId(world.systemId, id);
    return {
      colour: peeked(worldFile(body, original, 'raw_data/colour.png')),
      normal: peeked(worldFile(body, original, 'raw_data/normal.png')),
    };
  };
  /**
   * The hour: the sun at the slider's longitude over the body shown at the
   * season's start, then turning on itself as the orbit time goes.
   */
  const skySpin = $derived.by(() => {
    if (!sys || !sysBody || !sysPlanet || sysBody.spin.locked) return 0;
    const start = systemSky(sys, sysBody.id, sysDays - orbitDays, sysTextures);
    const longitude = Math.atan2(start.sun[0], start.sun[2]);
    return longitude - (sunLongitude * Math.PI) / 180;
  });
  /** Points round each orbit line in the sky, by quality (see QualityProfile.orbitSamples). */
  const orbitSamples = currentProfile().orbitSamples;
  const sysSky = $derived.by(() => {
    if (!sys || !sysBody || !sysPlanet) return null;
    return systemSky(sys, sysBody.id, sysDays, sysTextures, skySpin, orbitSamples);
  });
  /** The trojan swarms and belts in that sky, while Small bodies is on. */
  const smallSky = $derived.by(() => {
    if (!scopeOn(smallScope) || !sys || !sysBody || !sysPlanet) return null;
    const all = smallBodiesSky(sys, sysBody.id, sysDays, skySpin);
    // The planet's own trojans are local; the belts and the other planets' trojans are the star system's.
    return {
      swarms: all.swarms.filter((swarm) => (swarm.planet === sysPlanet.id ? smallScope.local : smallScope.stellar)),
      belts: smallScope.stellar ? all.belts : [],
    };
  });
  $effect(() => view?.setSkyBelts(smallSky ? smallSky.belts.flatMap((b) => b.edges) : []));
  /** The rotation dial round the body shown, while its switch is on (see lib/rotationDial). */
  const dial = $derived(dialShown && sys && sysBody && sysSky ? rotationDial(sys, sysBody.id, sysDays, skySpin, sysSky) : null);
  /**
   * The bodies of the sky chosen for the dial, by system and id: the host
   * by default, the others when ticked in the Light and season panel.
   */
  let dialChoices = $state<Record<string, boolean>>({});
  const dialChosen = (id: string, host: boolean) => dialChoices[`${world.systemId ?? ''}/${id}`] ?? host;
  function chooseDialBody(id: string, on: boolean): void {
    dialChoices = { ...dialChoices, [`${world.systemId ?? ''}/${id}`]: on };
  }
  /** The dial as drawn: with the chosen bodies only. */
  const dialDrawn = $derived(dial ? { ...dial.state, bodies: dial.state.bodies.filter((b) => dialChosen(b.id, b.host)) } : null);
  $effect(() => view?.setRotationDial(dialDrawn));
  /** The bodies the dial can show, with their names, for the panel's ticks. */
  const dialChoiceList = $derived(
    dial && sys
      ? dial.state.bodies.map((b) => {
          const body = sys.bodies.find((x) => x.id === b.id);
          return { id: b.id, host: b.host, name: world.systemNames[b.id] ?? body?.designation ?? b.id, on: dialChosen(b.id, b.host) };
        })
      : [],
  );
  /** The dial's names and readings, laid out with the other labels of the sky (see SkyLabels). */
  const dialLabels = $derived.by((): { at: { x: number; y: number } | null; label: SkyLabel }[] => {
    const at = skyPositions?.dial;
    if (!dialDrawn || !at || !sys) return [];
    const degrees = (radians: number) => formatDialAngle(radians, true, formatNumber);
    const out: { at: { x: number; y: number } | null; label: SkyLabel }[] = [
      {
        at: at.pointer,
        label: { name: dialReading, detail: dialDrawn.locked ? t.viewer.dialPointerLocked : t.viewer.dialPointerFree },
      },
    ];
    if (dialDrawn.sun) {
      out.push({
        at: at.sun,
        label: {
          name: world.systemNames['star'] ?? sys.star.designation,
          detail: t.viewer.dialHeight({ angle: degrees(dialDrawn.sunAltitude) }),
        },
      });
    }
    // The host's name and distance already hang on its line with the spin
    // axes: its marker then only gives its height.
    const hostNamed = hostLink && skyPositions?.guides.host;
    dialDrawn.bodies.forEach((b, k) => {
      const name = dialChoiceList.find((c) => c.id === b.id)?.name ?? b.id;
      const height = t.viewer.dialHeight({ angle: degrees(b.altitude) });
      out.push({ at: at.bodies[k] ?? null, label: b.host && hostNamed ? { name: height, detail: '' } : { name, detail: height } });
    });
    return out;
  });
  /** The pointer's reading on the ring: whole degrees, signed tenths for a locked body. */
  const dialReading = $derived(dial ? formatDialAngle(dial.state.reading, dial.state.locked, formatNumber) : '');
  /**
   * The host planet's compass, while its switch is on: the host's own
   * rotation dial (see lib/rotationDial), drawn round it in the moon's sky,
   * with the moon shown as the only body on it.
   */
  const hostCompass = $derived.by(() => {
    if (!hostCompassShown || !sys || !sysBody?.parent || !sysSky) return null;
    const index = sysSky.ids.indexOf(sysBody.parent);
    const dial = index < 0 ? null : hostCompassOf(sys, sysBody.id, sysDays);
    return dial ? { index, dial } : null;
  });
  $effect(() => view?.setHostCompass(hostCompass ? { index: hostCompass.index, state: hostCompass.dial.state } : null));
  /** The compass's names and readings: the pointer's, the star's and the moon's. */
  const hostCompassLabels = $derived.by((): { at: { x: number; y: number } | null; label: SkyLabel }[] => {
    const at = skyPositions?.hostDial;
    if (!hostCompass || !at || !sys || !sysBody) return [];
    const { state } = hostCompass.dial;
    const degrees = (radians: number) => formatDialAngle(radians, true, formatNumber);
    const out: { at: { x: number; y: number } | null; label: SkyLabel }[] = [
      {
        at: at.pointer,
        label: {
          name: formatDialAngle(state.reading, state.locked, formatNumber),
          detail: state.locked ? t.viewer.dialPointerLocked : t.viewer.dialPointerFree,
        },
      },
    ];
    if (state.sun) {
      out.push({
        at: at.sun,
        label: { name: world.systemNames['star'] ?? sys.star.designation, detail: t.viewer.dialHeight({ angle: degrees(state.sunAltitude) }) },
      });
    }
    state.bodies.forEach((b, k) => {
      out.push({
        at: at.bodies[k] ?? null,
        label: { name: world.systemNames[sysBody.id] ?? sysBody.designation, detail: t.viewer.dialHeight({ angle: degrees(b.altitude) }) },
      });
    });
    return out;
  });
  /** The dial's figures, two lines for the Light and season panel. */
  const dialLines = $derived.by((): [string, string] | null => {
    if (!dial) return null;
    const units = { hour: t.viewer.hourUnit, minute: t.viewer.minuteUnit, day: t.viewer.dayUnit };
    const text = dialFigureText(dial, units, { space: t.format.unitSpace, number: formatNumber });
    return text.locked
      ? [t.viewer.dialLocked(text), t.viewer.dialLockedTime(text)]
      : [text.resonant ? t.viewer.dialResonant(text) : t.viewer.dialFree(text), t.viewer.dialFreeDays(text)];
  });
  /** The glossary of the dial's figures, opened by its `?` button. */
  let dialGlossary = $state(false);
  // Followed on the screen: each swarm's centre, each belt's edges (every
  // third point is plenty to put its label).
  $effect(() =>
    view?.setSkyMarks(
      smallSky
        ? [
            ...smallSky.swarms.map((s) => ({ candidates: [s.position] })),
            ...smallSky.belts.map((b, k) => ({
              candidates: [...b.edges[0], ...b.edges[1]].filter((_, i) => i % 3 === 0),
              prefer: [0.2 + (0.6 * (k + 0.5)) / smallSky.belts.length, 0.5] as const,
            })),
          ]
        : [],
    ),
  );
  /** Their labels, in the order of the marks. */
  const smallLabels = $derived.by((): SkyLabel[] => {
    if (!smallSky || !sys) return [];
    const nameOf = (id: string) => {
      const body = sys.bodies.find((b) => b.id === id);
      return world.systemNames[id] ?? body?.designation ?? id;
    };
    // Two significant figures: the draw's counts are estimates.
    const about = (n: number) => {
      const digits = Math.max(0, Math.floor(Math.log10(Math.max(n, 1))) - 1);
      return formatInt(Math.round(n / 10 ** digits) * 10 ** digits);
    };
    const au = (v: number) => formatUpTo(v, v < 10 ? 2 : 0);
    return [
      ...smallSky.swarms.map((s) => ({
        name: t.viewer.trojanSwarm({ point: s.point, planet: nameOf(s.planet) }),
        detail: t.viewer.trojanFacts({ count: about(s.count), n: s.count }),
      })),
      ...smallSky.belts.map((b) => {
        const belt = sys.belts![b.index]!;
        return {
          name: t.viewer.beltNames[belt.kind],
          detail: t.viewer.beltFacts({ inner: au(belt.inner_au), outer: au(belt.outer_au), inside: b.inside }),
        };
      }),
    ];
  });
  // Its preview, while it has no maps.
  $effect(() => view?.setPreviewBody(sysBody && !sysBody.generated ? sysBody.preview : null));
  /**
   * The generation of the body shown and its neighbourhood, when it is not
   * generated yet (`POST /api/systems/{id}/bodies/{body}/generate`): queued
   * as soon as it is shown, followed until done, then the system reloaded.
   * Or, with `size`, one more resolution of a generated body, asked for
   * from its chips (`POST …/sizes/{W}x{H}`), shown once done.
   */
  let generation = $state<{ body: string; size: string | null; status: JobStatus | null; error: string | null } | null>(null);
  /** A running status of a system job: the body being made and its place in the job. */
  type RunningBody = { body?: string | null; body_step?: number | null; bodies?: number | null; stage: Stage; step: number; steps: number };
  let stopFollowing: (() => void) | null = null;
  /** The jobs this tab has followed: a listing may name one for a moment after it ends. */
  const followedJobs = new Set<string>();
  /** The width of a size name, as the chips and messages name a size. */
  const sizeWidth = (size: string) => String(parseSize(size)?.[0] ?? size);
  async function generateShown(body: string, size?: string): Promise<void> {
    const systemId = world.systemId;
    if (!systemId) return;
    stopFollowing?.();
    stopFollowing = null;
    const wanted = size ?? null;
    generation = { body, size: wanted, status: null, error: null };
    // Still this generation: another one (another body, another size) replaces it.
    const mine = () => generation?.body === body && generation.size === wanted;
    const id = systemBodyId(systemId, body);
    /** Shows the resolution made, when its body is still the one shown. */
    const showSize = async (made: string) => {
      if (world.id !== id) return;
      await world.refreshSizes();
      if (world.id === id) await world.switchSize(made);
    };
    try {
      let jobId: string | null;
      if (wanted !== null) {
        // Being generated already (from another tab): follow its job.
        const pending = world.id === id ? world.sizes?.pending.find((p) => sizeName(p.width, p.height) === wanted) : undefined;
        jobId = pending ? pending.job_id : (await api.generateSize(id, wanted)).job_id;
      } else {
        // Read before the request: the system may be listed again while it
        // is in flight (a job ending), and no longer name the body.
        const wasEvicted = world.systemEvicted.includes(body);
        const started = await api.generateBody(systemId, body);
        jobId = started.job_id;
        // Bodies that lost their files, made by another generator (the one
        // asked for, and its family: a planet's job makes its moons): the
        // user hears it once each. The body opened, if it lost its files,
        // is told of once as it is made again; when it is also made by an
        // older generator, one toast says both.
        for (const { kind, body: named } of remadeToasts(systemId, body, wasEvicted, started.regenerated)) {
          const title = bodyTitle(sys, world.systemNames, named);
          if (kind === 'evictedRemade') toasts.show(t.app.evictedRemade({ body: title }), { ms: 5000 });
          else toasts.show(kind === 'evictedRemadeOlder' ? t.app.evictedRemadeOlder({ body: title }) : t.app.olderWorld({ body: title }), { ms: 12000 });
        }
      }
      if (jobId !== null) followedJobs.add(jobId);
      if (jobId === null) {
        if (!mine()) return;
        generation = null;
        if (wanted !== null) await showSize(wanted);
        else world.loadSystem(systemId);
        return;
      }
      const stream = followJob(jobId, (status) => {
        if (mine()) generation = { body, size: wanted, status, error: null };
      });
      stopFollowing = stream.cancel;
      const final = await stream.done;
      if (!mine()) return;
      if (final.status === 'done') {
        generation = null;
        // Made again: a later eviction of these bodies tells again.
        forgetTold(systemId);
        if (wanted !== null) await showSize(wanted);
        else world.loadSystem(systemId);
      } else {
        generation = { body, size: wanted, status: final, error: final.status === 'failed' ? final.error : null };
      }
    } catch (e) {
      if (mine()) generation = { body, size: wanted, status: null, error: errorText(e) };
      else if (wanted !== null) toasts.error(t.viewer.sizeFailed({ size: sizeWidth(wanted), error: errorText(e) }));
    }
  }
  // A size of the body shown being generated by a job this tab does not
  // follow (another tab's, or one started before a reload): follow it, with
  // the same bar, and show the size once done.
  $effect(() => {
    const sizes = world.sizes;
    const body = sysBody;
    const systemId = world.systemId;
    if (!sizes || !body?.generated || !systemId || world.subject !== 'planet' || worldId !== systemBodyId(systemId, body.id)) return;
    untrack(() => {
      const job = pendingToFollow(sizes, generation !== null && !generation.error, followedJobs);
      if (job) void generateShown(body.id, sizeName(job.width, job.height));
    });
  });
  /** Deletes resolution `size` of the body shown (its chip's ×), then lists the sizes again. */
  async function deleteShownSize(size: string): Promise<void> {
    const id = world.id;
    if (!id) return;
    try {
      await api.deleteSize(id, size);
    } catch (e) {
      toasts.error(errorText(e));
    }
    // Deleted or not, the listing tells: the body loads at its displayed
    // size when the one shown is gone.
    await world.refreshSizes().catch((e: unknown) => console.warn('Could not list the resolutions', e));
  }
  /** Shows resolution `size` of the body shown (a chip on disk). */
  function pickSize(size: string): void {
    world.switchSize(size).catch((e: unknown) => toasts.error(errorText(e)));
  }
  /** The resolution chips under the planet, from the icon bar's Resolution switch. */
  let sizesOpen = $state(false);
  /** A generated body has resolutions to show: its listing is in. */
  const sizesShown = $derived(world.subject === 'planet' && worldId !== null && world.sizes !== null);
  /** The resolution this view is generating for the body shown, `WxH`. */
  const sizeGenerating = $derived(
    generation && generation.size !== null && !generation.error && worldId === systemBodyId(world.systemId ?? '', generation.body)
      ? generation.size
      : null,
  );
  $effect(() => {
    // Only the body looked at is generated: the star view of a system
    // (its default) leaves its planets as spheres until one is opened.
    const body = sysBody;
    if (!body || body.generated || world.subject !== 'planet') return;
    untrack(() => {
      if (generation?.body !== body.id) void generateShown(body.id);
    });
  });
  onDestroy(() => stopFollowing?.());
  /** The line of the generation in progress. */
  const generationText = $derived.by(() => {
    if (!generation || !sys) return null;
    const name = (id: string) => world.systemNames[id] ?? sys.bodies.find((b) => b.id === id)?.designation ?? id;
    if (generation.error) {
      return generation.size !== null
        ? t.viewer.sizeFailed({ size: sizeWidth(generation.size), error: generation.error })
        : t.viewer.generationFailed({ body: name(generation.body), error: generation.error });
    }
    const job = generation.status;
    if (generation.size !== null) {
      const width = sizeWidth(generation.size);
      return !job || job.status === 'queued' || job.status === 'running' ? t.viewer.sizeGenerating({ size: width }) : null;
    }
    if (!job || job.status === 'queued') return t.viewer.generationQueued({ body: name(generation.body) });
    if (job.status !== 'running') return null;
    const status = job as unknown as RunningBody;
    return t.viewer.generationRunning({
      body: name(status.body ?? generation.body),
      step: status.body_step ?? 1,
      bodies: status.bodies ?? 1,
      stage: status.stage ? t.stages[status.stage] : '',
      stageStep: status.step ?? 0,
      stageSteps: status.steps ?? 0,
    });
  });
  /** Its share done, 0 to 1. */
  const generationShare = $derived.by(() => {
    const job = generation?.status;
    if (!job || job.status !== 'running') return 0;
    const status = job as unknown as RunningBody;
    const bodies = Math.max(status.bodies ?? 1, 1);
    const within = (status.step ?? 0) / Math.max(status.steps ?? 1, 1);
    return ((status.body_step ?? 1) - 1 + within) / bodies;
  });
  /** The period the orbit slider spans: a moon's orbit; a planet's innermost moon's, else ten of its days. */
  const orbitPeriod = $derived.by(() => {
    if (!sys || !sysBody) return null;
    if (sysBody.parent) return sysBody.orbit.period_days;
    const moon = sys.bodies.find((b) => b.parent === sysBody.id);
    return moon?.orbit.period_days ?? (10 * sysBody.spin.period_hours) / 24;
  });
  /** The orbit the Light and season panel states: the moon's own, or the planet's year (not the slider's span). */
  const orbitClock = $derived(sysBody ? orbitReadout({ parent: sysBody.parent, orbitPeriodDays: sysBody.orbit.period_days }, yearDay ?? 0, orbitDays) : null);
  /**
   * The slowest orbit speed: ORBIT_SPEED_MIN, or slower for a short orbit,
   * down to real time (the power of two at or just below it).
   */
  const orbitSpeedMin = $derived(
    orbitPeriod ? Math.min(ORBIT_SPEED_MIN, Math.floor(Math.log2(ORBIT_SECONDS / (orbitPeriod * 86400)))) : ORBIT_SPEED_MIN,
  );
  // A speed slower than the body shown allows (kept from a shorter orbit) comes up to its slowest.
  $effect(() => {
    if (orbitSpeed < orbitSpeedMin) orbitSpeed = orbitSpeedMin;
  });
  /** The sun's direction over the body shown, from its system: longitude and latitude, degrees. */
  const sunPlace = $derived.by(() => {
    const dir = sysSky?.sun;
    if (!dir) return null;
    return {
      longitudeDeg: (Math.atan2(dir[0], dir[2]) * 180) / Math.PI,
      latitudeDeg: (Math.asin(Math.max(-1, Math.min(1, dir[1]))) * 180) / Math.PI,
    };
  });
  // The latitude under the sun, as the system places it, or from the tilt
  // and the season on a world without a system.
  const declination = $derived(
    sunPlace?.latitudeDeg ?? (locked ? 0 : sunDeclinationDeg(meta?.axial_tilt_deg ?? 23.44, season)),
  );
  $effect(() => {
    view?.setLighting(
      !sun
        ? { kind: 'headlight' }
        : sunPlace
          ? { kind: 'sun', season, longitudeDeg: sunPlace.longitudeDeg, axialTiltDeg: 0, latitudeDeg: sunPlace.latitudeDeg }
          : locked
            ? { kind: 'sun', season: 0, longitudeDeg: 180, axialTiltDeg: 0 }
            : { kind: 'sun', season, longitudeDeg: sunLongitude, axialTiltDeg: meta?.axial_tilt_deg ?? 23.44 },
    );
  });
  // The stars turn against the body shown once per day of its own (once
  // per orbit for a body locked to what it orbits).
  const starsTurn = $derived.by(() => {
    if (!sysBody) return 0;
    const period = sysBody.spin.locked ? sysBody.orbit.period_days : sysBody.spin.period_hours / 24;
    // Not wrapped, so that the camera follows a fast turn the right way.
    return (-2 * Math.PI * sysDays) / Math.max(period, 1e-6);
  });
  // While the orbits play, the camera turns with the stars: it stays still
  // against the sky, the star and the orbits, round the globe's centre or
  // the point it looks at, and sees the globe turn on itself under it; or,
  // with "Camera turns with the body" (on by default for a body locked to
  // what it orbits), it stays fixed on the body, always seeing the same
  // face, the sky turning round it.
  $effect(() => view?.setStarsTurn(starsTurn, orbitsPlaying && !cameraTurnsWithBody));
  // The gizmo shows the axes of the frame the camera stays in.
  $effect(() => view?.setGizmoOnSky(Boolean(sysBody) && !cameraTurnsWithBody));
  // The star drawn at its place, at the centre of the planets' orbits.
  $effect(() => view?.setStarPlace(sysSky?.star ?? null));
  // Its living surface at the system's day, the clock's speed for the fades.
  $effect(() =>
    view?.setHostStarSurface(
      sys && sysSky
        ? {
            info: sys.star.info,
            activity: sys.star.activity ?? null,
            days: sysDays,
            daysPerSecond: orbitsPlaying && orbitPeriod ? (orbitPeriod * 2 ** orbitSpeed) / ORBIT_SECONDS : 0,
            axes: sysSky.star.axes,
          }
        : null,
    ),
  );
  // Far enough out to see the moons of the shown body's planet whole.
  const reach = $derived(sys && sysBody ? systemReach(sys, sysBody.id) : null);
  // Not aborted when the bodies move (every frame of the seasons): a map
  // still loading is kept, and a newer set of bodies drops an older one.
  /**
   * The rings of the giant in its moon's sky: as its system drew them,
   * else as the moon's world stores its giant (systems drawn before).
   */
  const hostRings = $derived.by(() => {
    const parent = sysBody?.parent ? sys?.bodies.find((b) => b.id === sysBody.parent) : null;
    if (!parent) return null;
    const given = parent.params.given_rings;
    return given ? given.rings : (meta?.host?.giant.rings ?? null);
  });
  /** Whether this view draws the rocks of the rings round a ring moon (high quality only). */
  const drawsRingRocks = currentProfile().ringRocks;
  /** The ring moon shown's rocks at the system's time, while they are asked for. */
  const ringRockWindow = $derived.by(() => {
    if (!drawsRingRocks || ringRockScale === 0 || !sys || !sysBody?.ring_role || !sysSky || !hostRings) return null;
    const k = sysSky.ids.indexOf(sysBody.parent ?? '');
    const giant = sysSky.bodies[k];
    const giantBody = sys.bodies.find((b) => b.id === sysBody.parent);
    if (!giant || !giantBody) return null;
    return ringRocks(sysBody, giant, giantBody.physical.radius_km, hostRings, sysDays);
  });
  $effect(() => view?.setRingRocks(ringRockWindow, ringRockScale));
  /**
   * The host planet's air and clouds in its moon's sky, from the planet's
   * own world.json (null until it loads, or when it is not generated).
   */
  let hostLook = $state.raw<{ system: string; id: string; look: PlanetLook } | null>(null);
  $effect(() => {
    const parent = sysBody?.parent ? sys?.bodies.find((b) => b.id === sysBody.parent) : null;
    const system = world.systemId;
    if (!parent?.generated || !system) {
      hostLook = null;
      return;
    }
    const id = systemBodyId(system, parent.id);
    const original = sizeName(parent.params.width, parent.params.height);
    const controller = new AbortController();
    // The host planet in the moon's sky: a peek, the planet is not opened.
    api
      .peekMetadata(id, original, controller.signal)
      .then((host) => {
        if (!controller.signal.aborted) hostLook = { system, id: parent.id, look: planetLook(host, (file) => peeked(worldFile(id, original, `raw_data/${file}`))) };
      })
      .catch((e: unknown) => {
        if (!controller.signal.aborted) console.warn("Could not load the host planet's world", e);
      });
    return () => controller.abort();
  });
  $effect(() => {
    // The planet in its moon's sky, by its own part of each switch.
    const parent = sysBody?.parent ?? null;
    const planetTilt = sysPlanet?.spin.axial_tilt_deg ?? 0;
    // The host by its own part of each switch; the other bodies of the sky
    // (a planet's moons, a moon's siblings) by the host's, else the body's.
    const hostShow: SkyBodyShow = {
      atmosphere: hostAtmosphere,
      clouds: hostClouds,
      drift: hostCloudDrift,
      grid: hostGrid,
      tropics: hostTropics,
      rings: hostRingsShown,
      axis: hostAxes,
      spin: hostSpinDirection,
    };
    const otherShow: SkyBodyShow = {
      atmosphere: true,
      clouds: true,
      drift: false,
      grid: false,
      tropics: false,
      rings: true,
      axis: parent ? hostAxes : axesShown,
      spin: parent ? hostSpinDirection : spinDirection,
    };
    const bodies = (sysSky?.bodies ?? []).map((body, k) =>
      sysSky?.ids[k] === parent
        ? { ...body, ...(hostLook?.id === parent && hostLook.system === world.systemId ? hostLook.look : {}), show: hostShow, axialTiltDeg: planetTilt, rings: hostRings }
        : { ...body, show: otherShow },
    );
    view?.setSkyBodies(bodies);
    // What lights the night side: the light these bodies reflect (lib/nightLight.ts).
    view?.setNightSources(nightSources(bodies, sysSky?.sun ?? [0, 0, 1]));
    view?.setShadowCasters(sysSky?.casters ?? []);
  });
  // The moons' orbits in the sky, seen from the body shown.
  const orbitKept = $derived(
    (sysSky?.orbitBodies ?? []).flatMap((o, i) => ((o.local ? orbitScope.local : orbitScope.stellar) ? [i] : [])),
  );
  const skyOrbitLoops = $derived(sysSky ? orbitKept.map((i) => sysSky.orbits[i]!) : []);
  // The Hill sphere: the body's own, or its host planet's from a moon (round
  // the planet as it stands in the moon's sky).
  const hill = $derived(sys && sysBody ? hillSphere(sys, sysBody.id) : null);
  const hillDrawn = $derived.by(() => {
    const k = hill && sysBody?.parent && sysSky ? sysSky.ids.indexOf(hill.id) : -1;
    const centre: [number, number, number] | null = !hill ? null : k >= 0 ? (sysSky?.bodies[k]?.position ?? null) : sysBody?.parent ? null : [0, 0, 0];
    return hillOn && hill && sysBody && centre ? { centre, radius: hill.radiusKm / Math.max(sysBody.physical.radius_km, 1e-6) } : null;
  });
  $effect(() => view?.setHillSphere(hillDrawn));
  // With the Hill sphere on, the camera may go out to see it whole.
  $effect(() => view?.setGlobeReach(Math.max(reach ?? 0, hillDrawn ? hillReach(hillDrawn.centre, hillDrawn.radius) : 0)));
  // The body shown's own orbit dashed: the body slides along it.
  $effect(() => view?.setSkyOrbits(skyOrbitLoops, orbitKept.map((k) => k === sysSky?.ownOrbit)));
  // A moon goes round its orbit 2^speed times every ORBIT_SECONDS while
  // the orbits play (see orbitPeriod): its dashes lengthen to match.
  $effect(() => view?.setSkyOrbitPace(orbitsPlaying && sysBody?.parent ? 2 ** orbitSpeed / ORBIT_SECONDS : 0));
  // Follow the bodies of the sky on the screen while their labels show.
  $effect(() => {
    const current = view;
    if (!current) return;
    // The guides' labels (spin, host) follow with the spin axes, the dial's with the dial.
    const follow = scopeOn(labelScope) || spinDirection || hostLink || hostSpinDirection || scopeOn(smallScope) || dial !== null || hostCompass !== null;
    current.onSkyPositions = follow ? (positions) => (skyPositions = positions) : null;
    if (!follow) skyPositions = null;
    return () => {
      current.onSkyPositions = null;
    };
  });
  /**
   * The names and facts of the bodies of the sky, in the order `sky` gives
   * them: the moons from their planet; from a moon, its planet then the
   * other moons; from a giant's moon world, the giant then its other moons.
   */
  const skyLabelList = $derived.by((): (SkyLabel | null)[] => {
    if (!sys || !sysSky) return [];
    const km = (v: number) => withUnit(formatInt(v), 'km');
    return sysSky.ids.map((id, k) => {
      const body = sys.bodies.find((b) => b.id === id);
      if (!body) return null;
      // From a moon, its planet: how far it is now, as its line says.
      const position = id === sysBody?.parent ? sysSky.bodies[k]?.position : undefined;
      if (position && sysBody) {
        return {
          name: world.systemNames[id] ?? body.designation,
          detail: t.viewer.skyDistanceFacts({
            radius: km(body.physical.radius_km),
            distance: km(Math.hypot(...position) * sysBody.physical.radius_km),
          }),
        };
      }
      return {
        name: world.systemNames[id] ?? body.designation,
        detail: t.viewer.skyOrbitFacts({
          radius: km(body.physical.radius_km),
          orbit: body.parent
            ? km(body.orbit.semi_major_axis_km)
            : withUnit(formatUpTo(body.orbit.semi_major_axis_au, 2), t.viewer.au),
        }),
      };
    });
  });
  const starLabel = $derived.by((): SkyLabel | null => {
    if (!sys || !sysPlanet) return null;
    return {
      name: world.systemNames['star'] ?? `${sys.star.designation} · ${sys.star.type}`,
      detail: t.viewer.skyStarFacts({ distance: withUnit(formatUpTo(sysPlanet.orbit.semi_major_axis_au, 2), t.viewer.au) }),
    };
  });

  /**
   * The body shown's spin, for its arrow: locked to what it orbits (a
   * ring) or on its own; and in a system, its spin speed at the equator.
   */
  const spinLocked = $derived(sysBody ? sysBody.spin.locked : (meta?.star?.tidally_locked ?? false));
  $effect(() => view?.setSpinLocked(meta || sysBody ? spinLocked : null));
  /** "Camera turns with the body" for the body shown: its own setting for a locked body. */
  const cameraTurnsWithBody = $derived(spinLocked ? cameraOnLockedBody : cameraOnBody);
  function setCameraTurnsWithBody(on: boolean): void {
    if (spinLocked) cameraOnLockedBody = on;
    else cameraOnBody = on;
  }
  /** A moon's host planet: where it is, and the moon's direction of motion round it (its own orbit's tangent). */
  const hostGuide = $derived.by(() => {
    if (!sys || !sysBody?.parent || !sysSky) return null;
    const index = sysSky.ids.indexOf(sysBody.parent);
    const host = sysSky.bodies[index];
    if (!host?.position) return null;
    const loop = sysSky.orbits[sysSky.ownOrbit] ?? null;
    let velocity: [number, number, number] | null = null;
    if (loop && loop.length > 2) {
      // The point of the loop at the body (the origin), and its neighbours.
      let k = 0;
      loop.forEach((p, i) => {
        if (Math.hypot(...p) < Math.hypot(...loop[k]!)) k = i;
      });
      const next = loop[(k + 1) % loop.length]!;
      const prev = loop[(k - 1 + loop.length) % loop.length]!;
      velocity = [next[0] - prev[0], next[1] - prev[1], next[2] - prev[2]];
    }
    return { position: host.position, velocity };
  });
  $effect(() => view?.setHostGuide(hostGuide));
  /** The host planet in a moon's sky, whose spin is drawn in 3D round it (Spin axes). */
  const hostBody = $derived(sys && sysBody?.parent ? (sys.bodies.find((b) => b.id === sysBody.parent) ?? null) : null);
  $effect(() => {
    const index = hostBody && sysSky ? sysSky.ids.indexOf(hostBody.id) : -1;
    view?.setHostSpin(hostBody && index >= 0 ? { index, locked: hostBody.spin.locked } : null);
  });
  /** The guides that carry a label. */
  type GuideKey = 'spin' | 'host' | 'pull' | 'velocity' | 'hostSpin';
  /** The guides' labels: the spin, the host's distance, its pull, the body's speed and the host's own spin. */
  const guideLabels = $derived.by((): { key: GuideKey; label: SkyLabel }[] => {
    const labels: { key: GuideKey; label: SkyLabel }[] = [];
    if (sysBody) {
      const hours = sysBody.spin.period_hours;
      labels.push({
        key: 'spin',
        label: spinLocked
          ? { name: t.viewer.spinLockedName, detail: t.viewer.spinLockedFacts({ period: formatYear(sysBody.orbit.period_days) }) }
          : {
              name: t.viewer.spinName,
              detail: t.viewer.spinFacts({
                speed: withUnit(formatInt((2 * Math.PI * sysBody.physical.radius_km) / Math.max(hours, 1e-6)), 'km/h'),
                period: withUnit(formatUpTo(hours, 1), 'h'),
              }),
            },
      });
    }
    const host = sys && sysBody?.parent ? sys.bodies.find((b) => b.id === sysBody.parent) : null;
    if (sysBody && host) {
      const G = 6.674e-11;
      const EARTH_KG = 5.972e24;
      const r = sysBody.orbit.semi_major_axis_km;
      const hostKm = hostGuide ? Math.hypot(...hostGuide.position) * sysBody.physical.radius_km : r;
      const mu = G * (host.physical.mass_earth + sysBody.physical.mass_earth) * EARTH_KG;
      const pull = (G * host.physical.mass_earth * EARTH_KG) / (hostKm * 1000) ** 2;
      const speed = Math.sqrt(Math.max(mu * (2 / (hostKm * 1000) - 1 / (r * 1000)), 0)) / 1000;
      const name = world.systemNames[host.id] ?? host.designation;
      // The line to the host and the pull of the host point the same way:
      // one label for both on the line (see the template), the pull's own
      // only when the line is hidden.
      const pullText = withUnit(formatUpTo(pull, pull < 0.01 ? 4 : 3), 'm/s²');
      labels.push({
        key: 'host',
        label: { name, detail: t.viewer.hostLineFacts({ distance: withUnit(formatInt(hostKm), 'km'), pull: pullText }) },
      });
      labels.push({ key: 'pull', label: { name: t.viewer.hostPull, detail: pullText } });
      labels.push({ key: 'velocity', label: { name: t.viewer.hostOrbitSpeed, detail: withUnit(formatUpTo(speed, 2), 'km/s') } });
      // Its own spin, drawn round it.
      const hostHours = host.spin.period_hours;
      labels.push({
        key: 'hostSpin',
        label: {
          name: t.viewer.hostSpinName({ name }),
          detail: host.spin.locked
            ? t.viewer.hostSpinLockedFacts({ period: formatYear(host.orbit.period_days) })
            : t.viewer.spinFacts({
                speed: withUnit(formatInt((2 * Math.PI * host.physical.radius_km) / Math.max(hostHours, 1e-6)), 'km/h'),
                period: withUnit(formatUpTo(hostHours, 1), 'h'),
              }),
        },
      });
    }
    return labels;
  });

  /** The world's axes on screen, for the planet view's gizmo. */
  let planetAxes = $state<ScreenAxis[]>([]);
  /**
   * The playback bar's clock (see docs/superpowers/specs/2026-11-05-
   * interface-redesign-design.md): in a system, one clock in days moves
   * the moons and the seasons together (the orbit animation, sysDays); a
   * body without a system plays its seasons.
   */
  const clockPlaying = $derived(sysSky && orbitPeriod ? orbitsPlaying : playing);
  function setClockPlaying(on: boolean): void {
    // Time needs the sun.
    if (on && !sun) sun = true;
    if (sysSky && orbitPeriod) orbitsPlaying = on;
    else playing = on;
  }
  /** The span of the time bar, days: a year of the planet (the seasons). */
  const clockYear = $derived(sysPlanet ? sysPlanet.orbit.period_days : null);
  /** Moves the clock to day `day` of the year: the season there, the orbit time from it. */
  function setClockDay(day: number): void {
    if (!clockYear) return;
    season = sysPlanet ? seasonOfYearDay(sysPlanet.orbit, ((day % clockYear) + clockYear) % clockYear) : 0;
    orbitDays = 0;
  }
  /**
   * A switch on something the body shown and its host planet both have:
   * from a moon, a square beside it for each, and the main button sets both
   * to the opposite of the moon's (see lib/switches.ts); else a plain switch.
   * `hostDisabled`: the host has nothing of it to draw.
   */
  function perBody(
    base: Omit<Toggle, 'on' | 'set' | 'subs'>,
    body: boolean,
    host: boolean,
    setBody: (on: boolean) => void,
    setHost: (on: boolean) => void,
    hostDisabled = false,
  ): Toggle {
    if (!hasHost) return { ...base, on: body, set: setBody };
    return {
      ...base,
      on: body,
      set: () => {
        const next = pressBody({ body, host });
        setBody(next.body);
        setHost(next.host);
      },
      subs: [
        { key: `${base.key}-moon`, label: t.viewer.squareMoon({ what: base.label }), icon: 'moon', on: body, set: setBody },
        { key: `${base.key}-host`, label: t.viewer.squareHost({ what: base.label }), icon: 'planet', on: host, set: setHost, disabled: hostDisabled },
      ],
    };
  }
  /**
   * A system-wide switch: when the body shown has a local group (its planet
   * and their moons), a square for it and one for the rest of the star
   * system, and the main button sets both; else a plain switch.
   */
  function scoped(base: Omit<Toggle, 'on' | 'set' | 'subs'>, scope: ScopePair, setScope: (scope: ScopePair) => void): Toggle {
    const main = { ...base, on: scopeOn(scope), set: () => setScope(pressScope(scope)) };
    if (!hasLocalGroup) return main;
    return {
      ...main,
      subs: [
        { key: `${base.key}-local`, label: t.viewer.squareLocal({ what: base.label }), icon: 'host', on: scope.local, set: (on) => setScope({ ...scope, local: on }) },
        { key: `${base.key}-stellar`, label: t.viewer.squareStellar({ what: base.label }), icon: 'star', on: scope.stellar, set: (on) => setScope({ ...scope, stellar: on }) },
      ],
    };
  }
  /** The drift row's reset button: the bands, storms and clouds back to where they were generated. */
  const driftReset = $derived({ label: t.viewer.resetDrift, icon: 'start' as const, run: () => view?.resetDrift() });
  /** Whether the host planet has an atmosphere halo, and clouds, to draw in a moon's sky. */
  const hostHasAir = $derived(Boolean(hostLook?.look.halo));
  const hostHasClouds = $derived(Boolean(hostLook?.look.clouds));
  /** The display switches, as the icon bar groups them: light, surface, sky. */
  const toggleGroups = $derived<Toggle[][]>([
    [
      { key: 'sun', label: t.viewer.sunlight, icon: 'sun', on: sun, set: (v) => (sun = v) },
      { key: 'starLight', label: t.viewer.starLight, help: t.viewer.starLightHelp, icon: 'starlight', on: starLight, set: (v) => (starLight = v) },
      perBody({ key: 'atmosphere', label: t.viewer.atmosphere, icon: 'atmosphere' }, atmosphere, hostAtmosphere, (v) => (atmosphere = v), (v) => (hostAtmosphere = v), !hostHasAir),
      { key: 'stars', label: t.viewer.stars, help: t.viewer.starsHelp, icon: 'sparkles', on: stars, set: (v) => (stars = v) },
    ],
    [
      ...(giant
        ? [{ ...perBody({ key: 'drift', label: t.viewer.bandDrift, help: t.viewer.bandDriftHelp, icon: 'drift' }, cloudDrift, hostCloudDrift, (v) => (cloudDrift = v), (v) => (hostCloudDrift = v), !hostHasClouds), action: driftReset }]
        : [
            perBody({ key: 'clouds', label: t.viewer.clouds, icon: 'clouds' }, clouds, hostClouds, (v) => (clouds = v), (v) => (hostClouds = v), !hostHasClouds),
            { ...perBody({ key: 'drift', label: t.viewer.drift, icon: 'drift' }, cloudDrift, hostCloudDrift, (v) => (cloudDrift = v), (v) => (hostCloudDrift = v), !hostHasClouds), action: driftReset },
          ]),
      // The host's rings: the body shown has no ring switch, so no squares.
      ...(hasHost && hostRings
        ? [{ key: 'hostRings', label: t.viewer.hostRings, help: t.viewer.hostRingsHelp, icon: 'giants' as const, on: hostRingsShown, set: (v: boolean) => (hostRingsShown = v) }]
        : []),
      perBody({ key: 'grid', label: t.viewer.grid, icon: 'graticule' }, graticule, hostGrid, (v) => (graticule = v), (v) => (hostGrid = v)),
      perBody({ key: 'tropics', label: t.viewer.tropics, help: t.viewer.tropicsHelp, icon: 'tropics' }, referenceLines, hostTropics, (v) => (referenceLines = v), (v) => (hostTropics = v)),
      // Contours come from the body's elevation data, which the host's
      // view of it does not have: no square for the host.
      { key: 'sea', label: t.viewer.depthContours, help: t.viewer.depthContoursHelp, icon: 'depth', on: seaContours, set: (v) => dataReady && (seaContours = v) },
      { key: 'land', label: t.viewer.elevationContours, help: t.viewer.elevationContoursHelp, icon: 'contours', on: landContours, set: (v) => dataReady && (landContours = v) },
    ],
    [
      scoped({ key: 'labels', label: t.viewer.skyLabels, help: t.viewer.skyLabelsHelp, icon: 'tag' }, labelScope, (s) => {
        skyLabelsLocal = s.local;
        skyLabels = s.stellar;
      }),
      ...(sys && sys.bodies.length > 1
        ? [
            scoped({ key: 'orbits', label: t.viewer.orbitLines, help: t.viewer.orbitLinesHelp, icon: 'orbit' }, orbitScope, (s) => {
              orbitLinesLocal = s.local;
              orbitLines = s.stellar;
            }),
          ]
        : []),
      ...(hill && sysBody
        ? [
            {
              key: 'hill',
              label: t.viewer.hillSphere,
              help: t.viewer.hillSphereHelp({
                host: world.systemNames[hill.id] ?? sys?.bodies.find((b) => b.id === hill.id)?.designation ?? hill.id,
                radius: withUnit(formatInt(hill.radiusKm), 'km'),
                radii: formatInt(hill.radiusInHostRadii),
              }),
              icon: 'orbit' as const,
              on: hillOn,
              set: (v: boolean) => (hillOn = v),
            },
          ]
        : []),
      ...(sys && ((sys.belts?.length ?? 0) > 0 || sys.bodies.some((b) => b.trojans))
        ? [
            scoped({ key: 'small', label: t.viewer.smallBodies, help: t.viewer.smallBodiesHelp, icon: 'trojans' }, smallScope, (s) => {
              smallBodiesLocal = s.local;
              smallBodies = s.stellar;
            }),
          ]
        : []),
      perBody({ key: 'axes', label: t.viewer.spinAxes, help: t.viewer.spinAxesHelp, icon: 'axis' }, axesShown, hostAxes, (v) => (axesShown = v), (v) => (hostAxes = v)),
      perBody({ key: 'spin', label: t.viewer.spinDirection, help: t.viewer.spinDirectionHelp, icon: 'spin' }, spinDirection, hostSpinDirection, (v) => (spinDirection = v), (v) => (hostSpinDirection = v)),
      ...(hasHost
        ? [{ key: 'hostLink', label: t.viewer.hostLink, help: t.viewer.hostLinkHelp, icon: 'links' as const, on: hostLink, set: (v: boolean) => (hostLink = v) }]
        : []),
      // The body shown's rotation dial (the moon's square) and the host's compass (the host's).
      ...(sys && sysBody
        ? [perBody({ key: 'dial', label: t.viewer.compass, help: t.viewer.compassHelp, icon: 'dial' }, dialShown, hostCompassShown, (v) => (dialShown = v), (v) => (hostCompassShown = v))]
        : []),
    ],
    // The resolutions of a generated body: chips under the planet.
    ...(sizesShown ? [[{ key: 'sizes', label: t.viewer.sizes, help: t.viewer.sizesHelp, icon: 'resolution' as const, on: sizesOpen, set: (on: boolean) => (sizesOpen = on) }]] : []),
  ]);
  /** The panels of the planet's dock. */
  const dockPanels = $derived<DockPanel[]>([
    ...(info ? [{ id: 'info', title: t.viewer.infoPanel, icon: 'info' as const, content: info }] : []),
    { id: 'map', title: t.viewer.mapPanel, icon: 'planet', content: mapPanel },
    { id: 'light', title: t.viewer.lightPanel, icon: 'sun', content: lightPanel },
    ...(epochs && epochStops.length > 1 ? [{ id: 'epoch', title: t.epochs.slider, icon: 'clock' as const, content: epochPanel }] : []),
    ...(drift ? [{ id: 'drift', title: t.drift.title, icon: 'clock' as const, content: drift }] : []),
    { id: 'hex', title: t.viewer.hexes, icon: 'hex', content: hexPanel },
  ]);

  // The orbit animation: one orbit of the slider's period every
  // ORBIT_SECONDS, paused while the canvas is hidden.
  $effect(() => {
    const period = orbitPeriod;
    if (!orbitsPlaying || !sun || !canvasVisible || !period) {
      return;
    }
    let last = performance.now();
    let frame = requestAnimationFrame(function step(now) {
      orbitDays = (orbitDays + ((now - last) / 1000 / ORBIT_SECONDS) * period * 2 ** orbitSpeed) % (period * 1000);
      last = now;
      frame = requestAnimationFrame(step);
    });
    return () => cancelAnimationFrame(frame);
  });

  // The season animation: one year every YEAR_SECONDS, paused while the
  // canvas is hidden (it resumes from the same day).
  $effect(() => {
    if (!playing || !sun || !canvasVisible || locked) {
      return;
    }
    const orbit = sysPlanet?.orbit;
    const year = clockYear;
    let last = performance.now();
    let frame = requestAnimationFrame(function step(now) {
      const fraction = (now - last) / 1000 / YEAR_SECONDS;
      // In a system a year of time passes in YEAR_SECONDS, not a year of angle:
      // the seasons of an eccentric orbit are unequal.
      season = orbit && year ? seasonOfYearDay(orbit, ((yearDayAt(orbit, clockDays(orbit, season)) + fraction * year) % year)) : (season + fraction) % 1;
      last = now;
      frame = requestAnimationFrame(step);
    });
    return () => cancelAnimationFrame(frame);
  });

  // The hex grid is computed by the server on demand, for the chosen column
  // count and river threshold; slider moves are debounced.
  $effect(() => {
    const current = view;
    const id = worldId;
    const shownSize = size;
    const columns = hexColumns;
    const riverFlow = Math.round(10 ** hexRiverLog);
    if (!current || !id || !shownSize || !hexOn) {
      current?.setHex(null);
      hexMap = null;
      hexBusy = false;
      hexServerBusy = false;
      return;
    }
    // A newer request (another slider value, another world, hexes turned
    // off) aborts this one: its result and its errors are dropped.
    const controller = new AbortController();
    const timer = setTimeout(() => {
      hexBusy = true;
      retryWhileBusy((signal) => api.hex(id, shownSize, { columns, river_flow_km2: riverFlow }, signal), {
        signal: controller.signal,
        // The server answers 503 while it generates a world.
        isBusy: (e) => e instanceof ApiFailure && e.status === 503,
        delayMs: HEX_RETRY_MS,
        onBusy: () => (hexServerBusy = true),
      })
        .then((map) => {
          current.setHex(map);
          hexMap = map;
        })
        .catch((e: unknown) => {
          if (!controller.signal.aborted) {
            toasts.error(t.viewer.hexFailed({ error: errorText(e) }));
          }
        })
        .finally(() => {
          if (!controller.signal.aborted) {
            hexBusy = false;
            hexServerBusy = false;
          }
        });
    }, 300);
    return () => {
      controller.abort();
      clearTimeout(timer);
    };
  });

  /** Back to the whole planet, north up (or to the whole system, in the star view). */
  export function resetView(): void {
    if (world.subject === 'star') {
      if (world.starMode === 'star') starSurfacePane?.resetView();
      else starPane?.resetView();
    } else {
      view?.resetView();
    }
  }

  /** Shows the same place as another viewer (once the view exists, if it does not yet). */
  export function applyCamera(state: CameraState): void {
    mode = state.mode;
    if (view) {
      view.setCamera(state);
    } else {
      pendingCamera = state;
    }
  }

  /** The camera, layer, relief and sun shown, for a link to this view. */
  export function viewState(): ViewLink | null {
    if (!view) {
      return null;
    }
    return {
      camera: view.getCamera(),
      layer: shown,
      relief,
      sun: sun ? { on: true, season, longitudeDeg: sunLongitude } : { on: false },
      ...(orbitDays === 0 || orbitPeriod === null ? {} : { orbitDays: orbitDays % orbitPeriod }),
      ...(epoch === 0 ? {} : { epoch }),
    };
  }

  /**
   * Opens the view of a link: the parts it has replace the current ones.
   * A layer this world lacks is kept as the choice, and the biomes shown
   * meanwhile (as when picked by hand).
   */
  export function applyView(link: ViewLink): void {
    if (link.layer) layer = link.layer;
    if (link.relief !== undefined) relief = link.relief;
    if (link.sun) {
      sun = link.sun.on;
      if (link.sun.on) {
        season = link.sun.season;
        sunLongitude = link.sun.longitudeDeg;
      }
    }
    if (link.epoch !== undefined) epochChoice = link.epoch;
    if (link.orbitDays !== undefined) {
      orbitDays = link.orbitDays;
      // Kept for the world the link opens, which may load just after.
      orbitLink = link.orbitDays;
      setTimeout(() => (orbitLink = null));
    }
    if (link.camera) {
      applyCamera(link.camera);
    }
  }

  /** Saves the current view as a PNG. */
  export async function screenshot(): Promise<void> {
    const blob = await view?.screenshot();
    if (blob && worldId) {
      saveBlob(blob, `${bodyFileName(worldId)}-${shown}-${mode}.png`);
    } else {
      toasts.error(t.viewer.screenshotFailed);
    }
  }

  function downloadHex(): void {
    if (hexMap && worldId) {
      downloadJson(hexMap, `${bodyFileName(worldId)}-hex-${hexMap.columns}.json`);
    }
  }

  function selectLayer(index: number): void {
    const option = LAYERS[index];
    if (option && available(option.needs, option.id)) {
      layer = option.id;
    }
  }

  function handleKey(event: KeyboardEvent): void {
    if (!keyboard || event.ctrlKey || event.metaKey || (event.altKey && !altAllowed(event.key))) {
      return;
    }
    const target = event.target as HTMLElement | null;
    if (target && (target.isContentEditable || ['INPUT', 'SELECT', 'TEXTAREA'].includes(target.tagName))) {
      return;
    }
    const shortcut = shortcutFor(event.key);
    if (!shortcut) {
      return;
    }
    event.preventDefault();
    switch (shortcut.action) {
      case 'layer':
        selectLayer(shortcut.index);
        break;
      case 'globe':
        mode = 'globe';
        break;
      case 'map':
        mode = 'map';
        break;
      case 'reset':
        resetView();
        break;
      case 'hexes':
        if (worldId) hexOn = !hexOn;
        break;
      case 'sunlight':
        sun = !sun;
        break;
      case 'seasons':
        // The playback bar's clock (it turns the sun on to play).
        setClockPlaying(!clockPlaying);
        break;
      case 'atmosphere':
        atmosphere = !atmosphere;
        break;
      case 'stars':
        stars = !stars;
        break;
      case 'starView':
        if (worldId) {
          if (world.subject === 'star') world.subject = 'planet';
          else toStar();
        }
        break;
      case 'moonView':
        stepBody();
        break;
      case 'clouds':
        clouds = !clouds;
        break;
      case 'drift':
        // Drifting needs the clouds: show them and drift.
        if (!clouds) {
          clouds = true;
          cloudDrift = true;
        } else {
          cloudDrift = !cloudDrift;
        }
        break;
      case 'grid':
        graticule = !graticule;
        break;
      case 'tropics':
        referenceLines = !referenceLines;
        break;
      case 'depthContours':
        seaContours = !seaContours;
        break;
      case 'elevationContours':
        landContours = !landContours;
        break;
      case 'measure':
        if (worldId) onToggleMeasure();
        break;
      case 'screenshot':
        if (worldId) void screenshot();
        break;
      case 'help':
        showHelp = !showHelp;
        break;
      case 'escape':
        if (showHelp) {
          showHelp = false;
        } else {
          pinned = null;
          onEscape();
        }
        break;
      case 'previousWorld':
        onStepWorld?.(-1);
        break;
      case 'nextWorld':
        onStepWorld?.(1);
        break;
      case 'reliefUp':
        if (dataReady) relief = stepRelief(relief, 1);
        break;
      case 'epochOlder':
        stepEpoch(1);
        break;
      case 'epochYounger':
        stepEpoch(-1);
        break;
      case 'epochOlderFar':
        stepEpoch(10);
        break;
      case 'epochYoungerFar':
        stepEpoch(-10);
        break;
      case 'reliefDown':
        if (dataReady) relief = stepRelief(relief, -1);
        break;
    }
  }

  // The help takes the focus when it opens, and gives it back when it
  // closes, so a keyboard user lands where they were.
  let helpDialog = $state<HTMLDivElement | null>(null);
  $effect(() => {
    if (!showHelp || !helpDialog) {
      return;
    }
    const previous = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    helpDialog.focus();
    return () => {
      if (previous?.isConnected) {
        previous.focus();
      }
    };
  });

  function helpKey(event: KeyboardEvent): void {
    // Also in a compare pane, which has no other shortcuts.
    if (event.key === 'Escape') {
      event.preventDefault();
      event.stopPropagation();
      showHelp = false;
    }
  }
</script>

<svelte:window onkeydown={handleKey} />

{#snippet mapPanel()}
  <div class="panel-body">
    <div class="segmented">
        <button class:active={mode === 'globe'} onclick={() => (mode = 'globe')} title={t.viewer.globeHelp}>{t.viewer.globe}</button>
        <button class:active={mode === 'map'} onclick={() => (mode = 'map')} title={t.viewer.mapHelp}>{t.viewer.map}</button>
    </div>
      <label>
        <span>{t.viewer.layer}</span>
        <select bind:value={layer} disabled={!dataReady}>
          {#each LAYERS as option, i (option.id)}
            <option value={option.id} disabled={dataReady && !available(option.needs, option.id)}
              >{i + 1}. {layerName(option.id)}{!dataReady || available(option.needs, option.id) ? '' : t.viewer.notInWorld}</option
            >
          {/each}
        </select>
      </label>
      <label title={t.viewer.reliefHelp}>
        <span>{t.viewer.relief}</span>
        <span class="relief-track">
          <input type="range" min="0" max={RELIEF_MAX} step="0.1" bind:value={relief} disabled={!dataReady} />
          <!-- The true relief's point on the track (see lib/relief.ts). -->
          <span class="true-mark" style:--at={RELIEF_DEFAULT / RELIEF_MAX} title={t.viewer.reliefTrue}></span>
        </span>
        <span class="relief-value">×{formatNumber(relief, 1)}</span>
      </label>
      <button
        class="small relief-reset"
        onclick={() => (relief = RELIEF_DEFAULT)}
        disabled={!dataReady || relief === RELIEF_DEFAULT}
        title={t.viewer.reliefResetHelp}>↺ {t.viewer.reliefTrue}</button
      >
    {#if sysBody?.ring_role && drawsRingRocks}
      <!-- The rocks of the rings round a ring moon, high quality only. -->
      <label class="ring-rocks" title={t.viewer.ringRocksHelp}>
        {t.viewer.ringRocks}
        <select bind:value={ringRockScale}>
          {#each RING_ROCK_SCALES as scale (scale)}
            <option value={scale}>{t.viewer.ringRockScales[scale] ?? scale}</option>
          {/each}
        </select>
      </label>
    {/if}
  </div>
{/snippet}
{#snippet lightPanel()}
  <div class="panel-body">
    {#if !sun}
      <p class="muted">{t.viewer.sunlightOff}</p>
    {/if}
      {#if sun && locked}
        <p class="muted">{t.viewer.lockedSun}</p>
      {:else if sun}
        <label>
          <span title={t.viewer.seasonHelp}>{t.viewer.season}</span>
          <input type="range" min="0" max="1" step="0.005" bind:value={season} />
        </label>
        <p class="muted">
          {t.viewer.sunOver({
            season: seasonLabel,
            latitude: formatNumber(Math.abs(declination), 1),
            north: declination >= 0,
          })}
        </p>
        <label>
          <span>{t.viewer.sunLongitude}</span>
          <input type="range" min="0" max="360" step="1" bind:value={sunLongitude} />
          <small class="value">{sunLongitude}°</small>
        </label>
      {/if}
    {#if sun && orbitClock && sysSky}
        <p class="muted">
          {t.viewer.orbitDay({
            days: formatNumber(orbitClock.day, orbitClock.period < 10 ? 2 : 1),
            period: formatNumber(orbitClock.period, orbitClock.period < 10 ? 2 : 1),
          })}{#if sysBody?.kind === 'moon'}<br />{sysBody.spin.locked
              ? t.viewer.moonSun
              : t.viewer.moonSunSpin({ hours: formatNumber(sysBody.spin.period_hours, 1) })}{/if}
        </p>
    {/if}
    {#if dialLines}
      <div class="dial-figures">
        <p class="muted">{dialLines[0]}<br />{dialLines[1]}</p>
        <button
          class="small"
          onclick={() => (dialGlossary = !dialGlossary)}
          aria-expanded={dialGlossary}
          aria-label={t.viewer.dialGlossaryHelp}
          title={t.viewer.dialGlossaryHelp}>?</button
        >
      </div>
      {#if dialGlossary}
        <dl class="dial-glossary">
          {#each t.viewer.dialGlossary as entry (entry.term)}
            <div><dt>{entry.term}</dt> <dd>{entry.text}</dd></div>
          {/each}
        </dl>
      {/if}
      {#if dialChoiceList.length > 0}
        <fieldset class="dial-bodies" title={t.viewer.dialBodiesHelp}>
          <legend>{t.viewer.dialBodies}</legend>
          {#each dialChoiceList as choice (choice.id)}
            <label class="check">
              <input type="checkbox" checked={choice.on} onchange={(e) => chooseDialBody(choice.id, e.currentTarget.checked)} />
              {choice.name}
            </label>
          {/each}
        </fieldset>
      {/if}
    {/if}
    {#if orbitPeriod && sysSky}
      <label class="check" title={t.viewer.cameraOnBodyHelp}>
        <input type="checkbox" checked={cameraTurnsWithBody} onchange={(e) => setCameraTurnsWithBody(e.currentTarget.checked)} />
        {t.viewer.cameraOnBody}
      </label>
    {/if}
  </div>
{/snippet}
{#snippet hexPanel()}
  <div class="panel-body">
    <label class="check">
      <input
        type="checkbox"
        checked={hexOn}
        disabled={!worldId || sysBody?.kind !== 'planet'}
        onchange={(e) => (hexOn = e.currentTarget.checked)}
      />
      {t.viewer.hexes}
    </label>
    {#if !hexOn}
      <p class="muted">{t.viewer.hexesOff}</p>
    {/if}
        {#if hexServerBusy}
          <small class="muted" title={t.viewer.serverBusyHelp}
            >{t.viewer.serverBusy}</small
          >
        {:else if hexBusy}
          <small class="muted">{t.viewer.computing}</small>
        {/if}
      {#if hexOn}
        <label>
          <span>{t.viewer.columns}</span>
          <input type="range" min="16" max="256" step="4" bind:value={hexColumns} />
          <small class="value">{hexColumns}</small>
        </label>
        <label>
          <span title={t.viewer.riverFromHelp}>{t.viewer.riverFrom}</span>
          <input type="range" min="3" max="7" step="0.1" bind:value={hexRiverLog} />
          <small class="value">{formatArea(10 ** hexRiverLog)}</small>
        </label>
        <button class="small" onclick={downloadHex} disabled={!hexMap}>{t.viewer.downloadHex}</button>
      {/if}
  </div>
{/snippet}
{#snippet epochPanel()}
  <div class="panel-body">
      <div class="epoch" role="group" aria-label={t.epochs.slider} title={`${t.epochs.sliderHelp}. ${t.epochs.presentOnly}`}>
        <span class="epoch-label unit-help" title={unitTooltip('Ma', 'Ga')} aria-live="polite">{formatEpoch(epochShown)}</span>
        <input
          type="range"
          min={0}
          max={epochStops.length - 1}
          step={1}
          value={epochStops.indexOf(epochShown)}
          oninput={(e) => (epochPreview = epochStops[Number(e.currentTarget.value)] ?? 0)}
          onchange={(e) => {
            epochChoice = epochStops[Number(e.currentTarget.value)] ?? 0;
            epochPreview = null;
          }}
          aria-label={t.epochs.slider}
          aria-valuetext={formatEpoch(epochShown)}
        />
        <button onclick={() => (epochChoice = 0)} disabled={epoch === 0 && epochPreview === null}>{t.epochs.backToPresent}</button>
        {#if epochMapLoading || world.epochLoading}
          <span class="epoch-busy" title={t.epochs.loading}>…</span>
        {/if}
      </div>
  </div>
{/snippet}

<div class="viewer">
  <!-- Hidden (no size) in the star view: the planet view pauses meanwhile. -->
  <canvas bind:this={canvas} class:measuring class:hidden={world.subject === 'star'}></canvas>
  {#if world.subject === 'planet' && skyPositions}
    {@const guides = guideLabels.filter((g) => (g.key === 'spin' ? spinDirection : g.key === 'hostSpin' ? hostSpinDirection : hostLink) && (g.key !== 'pull' || !skyPositions!.guides.host))}
    {@const marks = smallSky ? skyPositions.marks : []}
    <!-- A dashed ring round each trojan swarm, too faint to see. -->
    {#each marks.slice(0, smallSky?.swarms.length ?? 0) as mark, k (k)}
      {#if mark}
        <span class="swarm-ring" style:left="{mark.x}px" style:top="{mark.y}px"></span>
      {/if}
    {/each}
    <SkyLabels
      positions={[
        ...skyPositions.bodies.map((at, k) => (inScope(labelScope, localIds, sysSky?.ids[k] ?? '') ? at : null)),
        ...guides.map((g) => skyPositions!.guides[g.key]),
        ...marks,
        ...dialLabels.map((d) => d.at),
        ...hostCompassLabels.map((d) => d.at),
      ]}
      labels={[
        ...skyLabelList,
        ...guides.map((g) => g.label),
        ...smallLabels.slice(0, marks.length),
        ...dialLabels.map((d) => d.label),
        ...hostCompassLabels.map((d) => d.label),
      ]}
      star={labelScope.stellar ? skyPositions.star : null}
      {starLabel}
    />
    {#if dial && skyPositions.dial}
      <DialLabels ticks={skyPositions.dial.ticks} arcTicks={skyPositions.dial.arcTicks} />
    {/if}
    {#if hostCompass && skyPositions.hostDial}
      <DialLabels ticks={skyPositions.hostDial.ticks} arcTicks={skyPositions.hostDial.arcTicks} />
    {/if}
  {/if}
  {#if world.subject === 'star' && (worldId || sys)}
    <StarPane
      kinds={sysStar ? sysStar.planets.map((p) => p.typeLabel) : null}
      selected={sysStar ? (pickedPlanet ? sysStar.selected : -1) : null}
      planets={sysStar ? sysStar.planets : null}
      pinned={world.pinnedPlanets}
      onPin={(id: string) => {
        if (!world.pinnedPlanets.includes(id)) world.pinnedPlanets = [...world.pinnedPlanets, id];
      }}
      onUnpin={(id: string) => (world.pinnedPlanets = world.pinnedPlanets.filter((p) => p !== id))}
      onUnpinAll={() => (world.pinnedPlanets = [])}
      onOpenMoon={(id: string) => {
        world.subject = 'planet';
        world.showSystemBody(id);
      }}
      onPickPlanet={sysStar ? togglePick : null}
      onOpenPlanet={sysStar ? openSystemPlanet : null}
      flyFrom={flyBack}
      onFlown={() => (flyBack = null)}
      onSelectPlanet={sysStar ? togglePick : null}
      maps={sysMaps}
      bind:this={starPane}
      planetName={sysPlanet ? (world.systemNames[sysPlanet.id] ?? sysPlanet.designation) : null}
      star={system.star}
      activity={sysStar ? (sys?.star.activity ?? null) : null}
      omega={system.omega}
      moons={(sysMoons ?? []) as unknown as Moon[]}
      binary={false}
      planetRadiusKm={sysPlanet?.physical.radius_km ?? 6371}
      bind:animated={starAnimated}
      bind:distances={starDistances}
      bind:orbit={starOrbit}
      bind:zone={starZone}
      bind:prominences={starProminences}
      giantSystem={system.giants}
      worldGiant={meta?.giant ?? null}
      worldTiltDeg={meta?.axial_tilt_deg ?? 0}
      host={null}
      bind:giants={starGiants}
      bind:links={starLinks}
      bind:velocities={starVelocities}
      bind:pulls={starPulls}
      bind:frost={starFrost}
      bind:hills={starHills}
      bind:realScale={starRealScale}
      belts={sysStar?.belts ?? []}
      bind:showBelts={starBelts}
      bind:showTrojans={starTrojans}
      bind:showLagrange={starLagrange}
      hidden={world.starMode === 'star'}
      bind:clockDays={starDays}
      onOpenStar={sys ? () => (world.starMode = 'star') : null}
    />
  {/if}
  {#if world.subject === 'star' && world.starMode === 'star' && sys}
    <StarSurfacePane
      bind:this={starSurfacePane}
      star={sys.star.info}
      activity={sys.star.activity ?? null}
      bind:clockDays={starDays}
      bind:animated={starAnimated}
      bind:loops={starProminences}
      bind:filter={starFilter}
      onOpenSystem={() => (world.starMode = 'system')}
    />
  {/if}

  <div class="toolbar" role="toolbar" aria-label={t.viewer.tools}>
    <select
      class="body-pick"
      aria-label={t.viewer.body}
      title={t.viewer.bodyHelp}
      value={world.subject === 'star' ? (world.starMode === 'star' ? 'star' : 'system') : (world.systemBody ?? '')}
      onchange={(e) => pickBody(e.currentTarget.value)}
      disabled={!sys}
    >
      {#if sys}
        <option value="system"
          >{t.viewer.bodyOption({ name: world.systemNames['star'] ?? sys.star.designation, role: t.viewer.roleSystem })}</option
        >
        <option value="star"
          >{t.viewer.bodyOption({ name: world.systemNames['star'] ?? sys.star.designation, role: t.viewer.roleStar })}</option
        >
        {#each sys.bodies as body (body.id)}
          <option value={body.id}
            >{body.parent ? '\u2003' : ''}{t.viewer.systemBodyOption({
              name: world.systemNames[body.id] ?? body.designation,
              type: body.ring_role
                ? t.viewer.ringMoonTypes[body.ring_role]
                : (t.viewer.bodyTypes[body.type as keyof typeof t.viewer.bodyTypes] ?? body.type),
              generated: body.generated,
            })}</option
          >
        {/each}
      {/if}
    </select>
    <button onclick={resetView} disabled={!worldId} title={t.viewer.resetViewHelp}>{t.viewer.resetView}</button>
    {#if world.subject === 'planet'}
      <button
        onclick={onToggleMeasure}
        disabled={!worldId}
        class:active={measuring}
        aria-pressed={measuring}
        title={t.viewer.measureHelp}>{t.viewer.measure}</button
      >
      <button onclick={screenshot} disabled={!worldId} title={t.viewer.screenshotHelp}>{t.viewer.screenshot}</button>
    {/if}
    <button onclick={() => (showHelp = !showHelp)} aria-expanded={showHelp} title={t.viewer.shortcutsHelp}>?</button>
  </div>

  {#if world.subject === 'planet'}
  <div class="planet-chrome">
    <div class="gizmo"><AxesGizmo axes={planetAxes} /></div>
    <div class="toggles">
      <ToggleBar label={t.viewer.display} groups={toggleGroups} bind:expanded={barExpanded} />
    </div>
    <Dock kind="planet" panels={dockPanels} defaultActive="map" />
    <PlaybackBar
      bind:playing={() => clockPlaying, (v) => setClockPlaying(v)}
      speed={sysSky && orbitPeriod
        ? {
            value: orbitSpeed,
            min: orbitSpeedMin,
            max: ORBIT_SPEED_MAX,
            text: t.viewer.orbitSpeedRate({ time: simulatedSecond((orbitPeriod / ORBIT_SECONDS) * 2 ** orbitSpeed) }),
            help: t.viewer.orbitSpeedHelp,
            set: (v) => (orbitSpeed = v),
          }
        : null}
      time={clockYear
        ? {
            value: yearDay ?? 0,
            max: clockYear,
            text: t.playback.dayOfYear({ day: formatInt(Math.floor(yearDay ?? 0) + 1), year: formatInt(Math.round(clockYear)) }),
            help: t.playback.yearHelp,
            set: setClockDay,
          }
        : null}
      clock={locked ? t.viewer.lockedSun : ''}
      jumps={clockYear ? [{ key: 'start', label: t.playback.toStart, icon: 'start', run: () => setClockDay(0) }] : []}
    />
    {#if sizesOpen && sizesShown && world.sizes}
      <div class="sizes">
        <SizeChips
          sizes={world.sizes}
          shown={size ?? ''}
          generating={sizeGenerating}
          onPick={pickSize}
          onGenerate={(s) => sysBody && generateShown(sysBody.id, s)}
          onDelete={(s) => void deleteShownSize(s)}
        />
      </div>
    {/if}
  </div>

  {#if dataReady && shown !== 'biomes'}
    {@const windShown = giant !== null && shown === 'moisture'}
    {@const topsShown = giant !== null && shown === 'elevation'}
    {@const ramp = windShown ? t.legends.wind : topsShown ? t.legends.cloudTops : t.legends[shown]}
    {@const range = shown === 'temperature' ? tempRange : null}
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="legend" class:bar-wide={barExpanded} role="region" onpointerdown={dragLegend} onclickcapture={legendClick} ondblclick={legendHome} title={t.viewer.legendDrag} style:left={legendAt ? `clamp(8px, ${legendAt.left}px, calc(100% - 248px))` : null} style:bottom={legendAt ? `clamp(8px, ${legendAt.bottom}px, calc(100% - 48px))` : null} aria-label={t.viewer.legend}>
      <strong>{giant ? layerName(shown) : ramp.label}</strong>
      <div class="bar" style:background={cssGradient(windShown ? 'wind' : topsShown ? 'cloudTops' : shown)}></div>
      <div class="ends">
        <span
          >{windShown && giant
            ? t.viewer.windEnd({ speed: formatInt(Math.max(giant.wind_east, giant.wind_west)), side: ramp.min })
            : range
              ? withUnit(formatInt(range[0]), '°C')
              : ramp.min}</span
        >
        <span
          >{windShown && giant
            ? t.viewer.windEnd({ speed: formatInt(Math.max(giant.wind_east, giant.wind_west)), side: ramp.max })
            : range
              ? withUnit(formatInt(range[1]), '°C')
              : shown === 'coast' && layers?.extra
                ? t.viewer.coastCap({ km: formatKm(layers.extra.coastMaxKm) })
                : ramp.max}</span
        >
      </div>
    </div>
  {:else if shown === 'biomes' && world.stats}
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="legend biomes" class:bar-wide={barExpanded} role="region" onpointerdown={dragLegend} onclickcapture={legendClick} ondblclick={legendHome} title={t.viewer.legendDrag} style:left={legendAt ? `clamp(8px, ${legendAt.left}px, calc(100% - 248px))` : null} style:bottom={legendAt ? `clamp(8px, ${legendAt.bottom}px, calc(100% - 48px))` : null} aria-label={t.viewer.biomeLegend}>
      <button class="legend-title" onclick={() => (legendOpen = !legendOpen)} aria-expanded={legendOpen}>
        <strong>{t.viewer.biomes}</strong>
        <small class="muted">{legendOpen ? t.viewer.legendHide : t.viewer.legendShow({ count: world.stats.shares.length })}</small>
      </button>
      {#if legendOpen}
        <ul onpointerleave={() => (hovered = null)}>
          {#each world.stats.shares as entry (entry.biome.id)}
            <li>
              <button
                class:pinned={pinned === entry.biome.id}
                onpointerenter={() => (hovered = entry.biome.id)}
                onfocus={() => (hovered = entry.biome.id)}
                onblur={() => (hovered = null)}
                onclick={() => (pinned = pinned === entry.biome.id ? null : entry.biome.id)}
                aria-pressed={pinned === entry.biome.id}
                title={t.viewer.highlightHelp}
              >
                <span class="swatch" style:background={legendColour(entry.biome)}></span>
                <span class="name">{biomeName(entry.biome)}</span>
                <span class="value">{formatShare(entry.share)}</span>
              </button>
            </li>
          {/each}
        </ul>
      {/if}
    </div>
  {/if}

  {/if}
  {#if showHelp}
    <div
      class="help"
      role="dialog"
      aria-label={t.viewer.shortcuts}
      tabindex="-1"
      bind:this={helpDialog}
      onkeydown={helpKey}
    >
      <header>
        <strong>{t.viewer.shortcuts}</strong>
        <button class="close" onclick={() => (showHelp = false)} aria-label={t.viewer.close}>×</button>
      </header>
      <dl>
        {#each shortcutHelp() as [key, action] (key)}
          <dt><kbd>{key}</kbd></dt>
          <dd>{action}</dd>
        {/each}
      </dl>
      <strong class="units">{t.units.title}</strong>
      <dl>
        {#each UNITS as unit (unit)}
          <dt><abbr>{unit}</abbr></dt>
          <dd>{t.units[unit].name}<br /><small>{t.units[unit].examples}</small></dd>
        {/each}
      </dl>
    </div>
  {/if}

  {#if generationText}
    <div class="chip generation" role="status">
      <p>{generationText}</p>
      <div
        class="generation-bar"
        role="progressbar"
        aria-valuemin={0}
        aria-valuemax={100}
        aria-valuenow={Math.round(generationShare * 100)}
      >
        <span style:width={`${generationShare * 100}%`}></span>
      </div>
      {#if generation?.error}
        <button onclick={() => generation && generateShown(generation.body, generation.size ?? undefined)}>{t.viewer.generationRetry}</button>
      {/if}
    </div>
  {/if}
  {#if !worldId && !sysBody}
    <p class="overlay">{t.viewer.noWorld}</p>
  {:else if world.subject === 'star' || sysBody?.generated === false}
    <!-- The star view, or a body not generated yet: no maps to wait for. -->
  {:else if loading}
    <div class="overlay">
      <div class="loading">
        <p>{t.viewer.loading}</p>
        {#if mapProgress}<LoadingBar progress={mapProgress} />{/if}
      </div>
    </div>
  {:else if !dataReady && !world.error}
    <div class="chip">
      <p>{t.viewer.loadingData}</p>
      {#if world.progress}<LoadingBar progress={world.progress} />{/if}
    </div>
  {/if}
  {#if error || world.error}
    <p class="overlay error">{error ?? world.error}</p>
  {/if}
</div>

<style>
  .swarm-ring {
    position: absolute;
    width: 34px;
    height: 34px;
    margin: -17px 0 0 -17px;
    border: 1.5px dashed var(--overlay-line, rgba(220, 200, 160, 0.8));
    border-radius: 50%;
    pointer-events: none;
    z-index: 1;
  }
  .ring-rocks {
    margin-top: 8px;
    display: grid;
    gap: 4px;
    font-size: 0.85rem;
  }
  .viewer {
    position: relative;
    height: 100%;
    min-height: 0;
  }
  canvas {
    display: block;
    width: 100%;
    height: 100%;
    cursor: grab;
  }
  canvas:active {
    cursor: grabbing;
  }
  canvas.hidden {
    display: none;
  }
  canvas.measuring {
    cursor: crosshair;
  }
  .toolbar {
    position: absolute;
    top: 12px;
    left: 12px;
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    /* Leave room for the display panel on the right. */
    max-width: calc(100% - 280px);
  }
  .toolbar button {
    /* Framed as the theme frames panels (see app.css and lib/themes.ts). */
    background: var(--frame-deco), var(--panel);
    clip-path: var(--frame-clip);
    box-shadow: var(--glow);
    padding: 4px 10px;
  }
  .toolbar button.active {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--accent-text);
  }
  .planet-chrome {
    position: absolute;
    inset: 0;
    pointer-events: none;
  }
  .toggles {
    position: absolute;
    left: 12px;
    top: 72px;
    /* Clear of the playback bar: a long bar (a moon's switches) scrolls. */
    bottom: 64px;
    overflow-y: auto;
    scrollbar-width: thin;
    pointer-events: auto;
  }
  .gizmo {
    position: absolute;
    top: 4px;
    right: 12px;
  }
  /* The resolution chips: under the planet, clear of the playback bar. */
  .sizes {
    position: absolute;
    left: 50%;
    bottom: 64px;
    transform: translateX(-50%);
    max-width: calc(100% - 24px);
  }
  .panel-body {
    display: grid;
    gap: 8px;
  }

  .panel-body label {
    display: grid;
    /* As wide as the label's words in the language, never more than 40 %. */
    grid-template-columns: fit-content(40%) minmax(0, 1fr) auto;
    align-items: center;
    gap: 6px;
  }
  .panel-body label > select {
    min-width: 0;
  }
  .relief-track {
    position: relative;
    display: grid;
    align-items: center;
  }
  .relief-track input {
    width: 100%;
    margin: 0;
  }
  /* A tick under the track at the true relief, the thumb's centre there
     (the thumb, about 16 px wide, stops half its width from each end). */
  .true-mark {
    position: absolute;
    left: calc(8px + (100% - 16px) * var(--at));
    bottom: -7px;
    width: 2px;
    height: 7px;
    margin-left: -1px;
    background: var(--accent);
    pointer-events: auto;
  }
  .relief-value {
    min-width: 3.2em;
    text-align: right;
    font-variant-numeric: tabular-nums;
  }
  .relief-reset {
    justify-self: end;
  }
  .panel-body label.check {
    display: flex;
    gap: 6px;
  }

  .small {
    padding: 2px 8px;
    font-size: 0.85rem;
  }
  .segmented {
    display: flex;
    border: 1px solid var(--border);
    border-radius: 6px;
    overflow: hidden;
  }
  .segmented button {
    border: none;
    border-radius: 0;
  }
  .segmented button.active {
    background: var(--accent);
    color: var(--accent-text);
  }
  .muted {
    margin: 0;
    color: var(--muted);
    font-size: 0.85rem;
  }
  .dial-figures {
    display: flex;
    align-items: flex-start;
    gap: 6px;
  }
  .dial-figures .muted {
    flex: 1;
    font-variant-numeric: tabular-nums;
  }
  .dial-glossary {
    display: grid;
    gap: 4px;
    margin: 0;
    font-size: 0.8rem;
    color: var(--muted);
  }
  .dial-glossary dt {
    display: inline;
    font-weight: 600;
    color: var(--text);
  }
  .dial-glossary dd {
    display: inline;
    margin: 0;
  }
  .dial-bodies {
    display: grid;
    gap: 2px;
    margin: 0;
    padding: 4px 8px 6px;
    border: 1px solid var(--border);
    border-radius: 4px;
  }
  .dial-bodies legend {
    padding: 0 4px;
    font-size: 0.8rem;
    color: var(--muted);
  }
  .value {
    font-variant-numeric: tabular-nums;
    color: var(--muted);
  }
  .legend {
    position: absolute;
    /* Bottom left, above the playback bar and right of the icon bar (12 px
       from the edge, 40 px wide), so that a long biome list never covers
       it; the dock holds the right. */
    left: 80px;
    bottom: 76px;
    width: 240px;
    /* Under the toolbar, whatever the list: it scrolls inside (see ul). */
    max-height: calc(100% - 172px);
    overflow: hidden;
    /* Framed as the theme frames panels (see app.css and lib/themes.ts). */
    background: var(--frame-deco), var(--panel);
    clip-path: var(--frame-clip);
    box-shadow: var(--glow);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 8px 10px;
    display: grid;
    gap: 4px;
    /* Dragged from anywhere on it (see dragLegend): no text selection. */
    cursor: grab;
    user-select: none;
    touch-action: none;
  }
  .legend.biomes {
    grid-template-rows: auto minmax(0, 1fr);
  }
  /* The icon bar showing its names is wider: the legend steps right of it. */
  .legend.bar-wide {
    left: 208px;
  }
  .legend ul {
    touch-action: pan-y;
  }  /* The playback bar wraps to two lines on narrow views. */
  @media (max-width: 720px) {
    .legend {
      bottom: 132px;
    }
  }

  .legend-title {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    border: none;
    background: none;
    padding: 0;
  }
  .legend ul {
    touch-action: pan-y;
    list-style: none;
    margin: 0;
    padding: 0;
    overflow-y: auto;
    /* Twelve biomes or so, then a scroll: the view stays visible. */
    max-height: min(35vh, 300px);
  }
  .legend li button {
    display: grid;
    grid-template-columns: 14px minmax(0, 1fr) auto;
    gap: 8px;
    align-items: center;
    width: 100%;
    border: 1px solid transparent;
    background: none;
    padding: 2px 4px;
    text-align: left;
  }
  .legend li button.pinned {
    border-color: var(--accent);
  }
  .swatch {
    width: 14px;
    height: 14px;
    border-radius: 3px;
    border: 1px solid var(--border);
  }
  .name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .bar {
    height: 10px;
    border-radius: 3px;
  }
  .ends {
    display: flex;
    justify-content: space-between;
    color: var(--muted);
    font-size: 0.8rem;
  }
  .help {
    position: absolute;
    top: 52px;
    left: 12px;
    /* Wide, in columns, rather than tall: it stays above the playback bar. */
    width: min(860px, calc(100% - 24px));
    max-height: calc(100% - 140px);
    overflow-y: auto;
    /* Framed as the theme frames panels (see app.css and lib/themes.ts). */
    background: var(--frame-deco), var(--panel-solid);
    clip-path: var(--frame-clip);
    box-shadow: var(--glow);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 10px 14px;
    z-index: 2;
  }
  .help:focus-visible {
    outline: 2px solid var(--accent);
  }
  .help header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 6px;
  }
  .help dl {
    display: grid;
    grid-template-columns: repeat(auto-fill, 7em minmax(13em, 1fr));
    gap: 4px 12px;
    margin: 0;
  }
  .help dd {
    margin: 0;
  }
  .help .units {
    display: block;
    margin: 10px 0 6px;
  }
  .help small {
    color: var(--muted);
  }
  kbd {
    font-family: ui-monospace, monospace;
    font-size: 0.85rem;
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 0 5px;
  }
  .close {
    border: none;
    background: none;
    font-size: 1.2rem;
    padding: 0 4px;
  }
  .overlay {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    margin: 0;
    pointer-events: none;
    color: var(--muted);
  }
  .loading {
    display: grid;
    justify-items: center;
    gap: 8px;
  }
  .loading p,
  .chip p {
    margin: 0;
  }
  .chip:has(:global(.loading-bar)) {
    border-radius: 10px;
    padding: 6px 12px;
    text-align: center;
  }
  .chip :global(.loading-bar) {
    margin: 4px 0 2px;
  }
  .chip.generation {
    bottom: auto;
    top: 96px;
    pointer-events: auto;
    border-radius: 10px;
    padding: 6px 12px;
    text-align: center;
    display: grid;
    gap: 4px;
    justify-items: center;
  }
  .generation-bar {
    width: 240px;
    height: 6px;
    border-radius: 3px;
    background: var(--border);
    overflow: hidden;
  }
  .generation-bar span {
    display: block;
    height: 100%;
    background: var(--accent);
    transition: width 0.3s;
  }
  .overlay.error {
    color: var(--danger);
    padding: 0 24px;
    text-align: center;
  }
  /* The time slider: a row of its own under the tools, wherever they are. */
  .epoch {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
  }

  .epoch input[type='range'] {
    flex: 1;
    min-width: 0;
  }
  .epoch-label {
    min-width: 8em;
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }
  .epoch-busy {
    color: var(--muted);
  }
  .chip {
    position: absolute;
    left: 50%;
    bottom: 12px;
    transform: translateX(-50%);
    margin: 0;
    padding: 4px 12px;
    border-radius: 999px;
    /* Framed as the theme frames panels (see app.css and lib/themes.ts). */
    background: var(--frame-deco), var(--panel);
    clip-path: var(--frame-clip);
    box-shadow: var(--glow);
    border: 1px solid var(--border);
    color: var(--muted);
    font-size: 0.85rem;
    pointer-events: none;
  }
  /* Narrow viewer: the tools go in a row under the display panel (which
     covers them when unfolded). */
  @container (max-width: 580px) {
    .toolbar {
      top: 64px;
      right: 12px;
      max-width: none;
    }
    .toolbar button {
      padding: 2px 8px;
      font-size: 0.85rem;
    }
  }
</style>
