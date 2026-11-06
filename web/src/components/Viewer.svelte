<script lang="ts">
  import { errorText } from '../lib/i18n/errors';
  import { t } from '../lib/i18n/index.svelte';
  import { biomeName } from '../lib/i18n/names.svelte';
  import { onDestroy, onMount, untrack, type Snippet } from 'svelte';
  import Dock, { type DockPanel } from './dock/Dock.svelte';
  import PlaybackBar from './dock/PlaybackBar.svelte';
  import ToggleBar, { type Toggle } from './dock/ToggleBar.svelte';
  import AxesGizmo from './AxesGizmo.svelte';
  import type { ScreenAxis } from '../lib/viewer/axes';
  import { ApiFailure, api, followJob, systemBodyId, worldFile, worldThumbnail } from '../lib/api/client';
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
  import { cloudTint, skyFromWorld } from '../lib/sky';
  import { toasts } from '../lib/toasts.svelte';
  import { cssGradient } from '../lib/viewer/colormaps';
  import { vegetationShares } from '../lib/vegetation';
  import StarPane from './StarPane.svelte';
  import SkyLabels, { type SkyLabel } from './SkyLabels.svelte';
  import { bodyFileName } from '../lib/moons';
  import type { Moon } from '../lib/api/types/Moon';
  import { smallBodiesSky, systemReach, systemSky } from '../lib/systemSky';
  import { systemStarView } from '../lib/systemStar';
  import { SUN_SYSTEM, withOrbitDefaults } from '../lib/viewer/starSystem';
  import { type CameraState, giantTemperatureRange, PlanetView, SUN_STAR, type ViewMode } from '../lib/viewer/PlanetView';
  import { currentProfile } from '../lib/viewer/quality';
  import type { Layer } from '../lib/viewer/shaders';
  import type { LoadedWorld } from '../lib/world.svelte';
  import LoadingBar from './LoadingBar.svelte';

  interface Props {
    world: LoadedWorld;
    /** Its revision (modification time), which keys the cached detail tiles. */
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
  /** The orbit animation's slowest and fastest speeds, powers of two (see orbitSpeed). */
  const ORBIT_SPEED_MIN = -8;
  // Up to a year of seasons in seconds: the playback bar's one clock moves them too.
  const ORBIT_SPEED_MAX = 12;
  /** A time in days as the speed's rate shows it: in minutes, hours or days. */
  const simulatedSecond = (days: number): string =>
    days < 1 / 24
      ? withUnit(formatUpTo(days * 1440, 1), t.viewer.minuteUnit)
      : days < 1
        ? withUnit(formatUpTo(days * 24, 1), t.viewer.hourUnit)
        : withUnit(formatUpTo(days, 1), t.viewer.dayUnit);
  /** mapgen::hex::DEFAULT_RIVER_FLOW_KM2, on the log10 slider. */
  const DEFAULT_RIVER_LOG = Math.log10(500_000);
  /** Wait before asking a server busy generating for the hex grid again, ms. */
  const HEX_RETRY_MS = 5000;

  /** Display preferences kept between visits. */
  const saved = loadSetting('display', {
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
    // The planet in a moon's sky, by its own switches.
    hostAtmosphere: true,
    hostClouds: true,
    hostGrid: false,
    hostTropics: false,
    orbitLines: false,
    // The trojan swarms and belts ringed in the sky.
    smallBodies: false,
    // The spin axes of the globe and the bodies of the sky.
    spinAxes: false,
    // Names tied to the bodies of the sky.
    skyLabels: true,
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
  let starLight = $state(saved.starLight);
  let clouds = $state(saved.clouds);
  /** The planet seen in a moon's sky (or the giant in its moon world's), drawn by its own switches. */
  let hostAtmosphere = $state(saved.hostAtmosphere);
  let hostClouds = $state(saved.hostClouds);
  let hostGrid = $state(saved.hostGrid);
  let hostTropics = $state(saved.hostTropics);
  /** The orbits of the moons (or of a giant's other moons) drawn in the sky. */
  let orbitLines = $state(saved.orbitLines);
  let smallBodies = $state(saved.smallBodies);
  let axesShown = $state(saved.spinAxes);
  $effect(() => view?.setSpinAxes(axesShown));
  const hostShow = $derived({ atmosphere: hostAtmosphere, clouds: hostClouds, grid: hostGrid, tropics: hostTropics });
  /** Labels on the bodies of the sky, and where those are on the screen. */
  let skyLabels = $state(saved.skyLabels);
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
  /** The body picker's choice: the star, or a body of the system. */
  function pickBody(key: string): void {
    if (key === 'star') {
      if (sys) toStar();
      return;
    }
    // From the star view, the camera flies to the planet (a moon's planet) first.
    const body = sys?.bodies.find((b) => b.id === key);
    const index = sysStar && body ? sysStar.ids.indexOf(body.parent ?? body.id) : -1;
    const open = () => {
      world.subject = 'planet';
      world.showSystemBody(key);
    };
    if (world.subject === 'star' && starPane && index >= 0) starPane.flyToPlanet(index, open);
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
  const seasonLabel = $derived.by(() => {
    const quarter = season * 4;
    const nearest = Math.round(quarter) % 4;
    return Math.abs(quarter - Math.round(quarter)) < 0.12
      ? (t.viewer.seasons[nearest] ?? '')
      : t.viewer.dayOfYear({ day: Math.round(season * 365) % 365 });
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
      hostGrid,
      hostTropics,
      orbitLines,
      smallBodies,
      spinAxes: axesShown,
      skyLabels,
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
    const rev = revision;
    // No revision: the world left the list, typically while it is being
    // regenerated in place. Keep its map until the new one is listed.
    if (rev === null && id !== null && id === textureOf) {
      loading = false;
      return;
    }
    pinned = null;
    hovered = null;
    if (!current || !id) {
      textureOf = null;
      loading = false;
      return;
    }
    textureOf = id;
    loading = true;
    mapProgress = null;
    error = null;
    hexOn = false;
    const controller = new AbortController();
    const progress = (p: LoadProgress) => {
      if (!controller.signal.aborted) mapProgress = p;
    };
    // The render without lines (format version 6), which the reference
    // lines and contours are drawn over; world.png, which has them baked
    // in, for older worlds. At another epoch, its render (a world shown at
    // the same epoch as the one before).
    const epochMa = untrack(() => epoch);
    textureEpoch = { id, epochMa, scale: 1 };
    const surface = epochMa === 0 ? worldFile(id, 'raw_data/surface.png', rev) : epochSurface(id, epochMa, rev, 1);
    current
      .setTexture(surface, controller.signal, progress)
      .catch((e: unknown) => {
        if (controller.signal.aborted) throw e;
        return current.setTexture(worldFile(id, 'world.png', rev), controller.signal, progress);
      })
      .catch((e: unknown) => {
        if (!controller.signal.aborted) error = t.viewer.mapFailed({ error: String(e) });
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
    const rev = revision;
    // Untracked: the world's own epoch state must not re-run this (a failed
    // load would be retried at once, again and again).
    untrack(() => world.setEpoch(epochMa, rev));
  });
  $effect(() => {
    if (world.epochError) toasts.error(world.epochError);
  });

  /** The base map shown: world, epoch and reduction. */
  let textureEpoch = { id: null as string | null, epochMa: 0, scale: 1 };
  /** Whether the base map of another epoch is loading. */
  let epochMapLoading = $state(false);

  /** URL of the base map of a world at an epoch (the render without lines, as raw_data/surface.png). */
  function epochSurface(id: string, epochMa: number, rev: number | null, scale: 1 | 4): string {
    return epochFile(id, epochMa, 'surface.png', rev, scale);
  }

  // The base map of another epoch of the world shown: reduced while the
  // slider moves (after a short pause), full once it is released.
  $effect(() => {
    const current = view;
    const id = worldId;
    const rev = revision;
    const epochMa = epochShown;
    const scale: 1 | 4 = epochPreview === null ? 1 : 4;
    if (!current || !id || id !== textureOf || loading) {
      return;
    }
    if (textureEpoch.id === id && textureEpoch.epochMa === epochMa && textureEpoch.scale === scale) {
      return;
    }
    const controller = new AbortController();
    const show = () => {
      epochMapLoading = true;
      const url =
        epochMa === 0 ? worldFile(id, 'raw_data/surface.png', rev) : epochSurface(id, epochMa, rev, scale);
      current
        .setTexture(url, controller.signal)
        .then(() => {
          textureEpoch = { id, epochMa, scale };
        })
        .catch((e: unknown) => {
          if (!controller.signal.aborted) toasts.error(t.epochs.failed({ error: String(e) }));
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
  // The cloud layer of worlds that have one (format version 5 and later).
  $effect(() => {
    const current = view;
    const id = worldId;
    const file = meta?.layers.clouds?.file;
    if (!current) {
      return;
    }
    const controller = new AbortController();
    const url = id && file ? worldFile(id, `raw_data/${file}`, revision) : null;
    current
      .setClouds(url, meta ? cloudTint(meta) : [1, 1, 1], controller.signal)
      .catch((e: unknown) => {
        if (!controller.signal.aborted) console.warn('Could not load the clouds', e);
      });
    return () => controller.abort();
  });
  // An irregular moon's shape, once per body.
  $effect(() => {
    const id = worldId;
    const shape = meta?.layers.shape;
    if (!view) return;
    const controller = new AbortController();
    view
      .setShape(id && shape ? { url: worldFile(id, `raw_data/${shape.file}`, revision), min: shape.min, max: shape.max } : null, controller.signal)
      .catch((e: unknown) => {
        if (!controller.signal.aborted) console.warn('Could not load the shape', e);
      });
    return () => controller.abort();
  });
  $effect(() => view?.setCloudsVisible(clouds));
  $effect(() => view?.setCloudsDrifting(cloudDrift));
  $effect(() => view?.setGiant(giant));
  // The dust rings its system drew for the giant shown.
  $effect(() => view?.setDustRings(giant && sysBody ? (sysBody.params.given_rings?.dust ?? []) : []));
  $effect(() => {
    // No detail while the slider moves: the reduced preview shows through.
    // A giant has no ground to detail.
    view?.setDetail(
      worldId && meta && !giant && revision !== null && epochPreview === null
        ? { id: worldId, revision, baseWidth: meta.width, epochMa: epoch }
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
  /** The planet of the body shown (itself, or the planet of a moon). */
  const sysPlanet = $derived(sys && sysBody ? (sys.bodies.find((b) => b.id === (sysBody.parent ?? sysBody.id)) ?? null) : null);
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
    world.subject = 'star';
  }
  /** The map of each generated planet of the star view, for its marker: its thumbnail, at the system's revision. */
  const sysMaps = $derived(
    sysStar && world.systemId
      ? sysStar.planets.map((p) => (p.generated ? worldThumbnail(systemBodyId(world.systemId!, p.id), revision) : null))
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
  const sysDays = $derived((sysPlanet ? season * sysPlanet.orbit.period_days : 0) + orbitDays);
  const sysTextures = (id: string) => (world.systemId ? worldFile(systemBodyId(world.systemId, id), 'raw_data/surface.png') : null);
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
  const sysSky = $derived.by(() => {
    if (!sys || !sysBody || !sysPlanet) return null;
    return systemSky(sys, sysBody.id, sysDays, sysTextures, skySpin);
  });
  /** The trojan swarms and belts in that sky, while Small bodies is on. */
  const smallSky = $derived(smallBodies && sys && sysBody && sysPlanet ? smallBodiesSky(sys, sysBody.id, sysDays, skySpin) : null);
  $effect(() => view?.setSkyBelts(smallSky ? smallSky.belts.flatMap((b) => b.edges) : []));
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
   */
  let generation = $state<{ body: string; status: JobStatus | null; error: string | null } | null>(null);
  /** A running status of a system job: the body being made and its place in the job. */
  type RunningBody = { body?: string | null; body_step?: number | null; bodies?: number | null; stage: Stage; step: number; steps: number };
  let stopFollowing: (() => void) | null = null;
  async function generateShown(body: string): Promise<void> {
    const systemId = world.systemId;
    if (!systemId) return;
    stopFollowing?.();
    stopFollowing = null;
    generation = { body, status: null, error: null };
    try {
      const response = await fetch(
        `/api/systems/${encodeURIComponent(systemId)}/bodies/${encodeURIComponent(body)}/generate`,
        { method: 'POST' },
      );
      if (!response.ok) throw new Error(`${response.status} ${await response.text()}`);
      const { job_id: jobId } = (await response.json()) as { job_id: string | null };
      if (jobId === null) {
        world.loadSystem(systemId);
        return;
      }
      const stream = followJob(jobId, (status) => {
        if (generation?.body === body) generation = { body, status, error: null };
      });
      stopFollowing = stream.cancel;
      const final = await stream.done;
      if (generation?.body !== body) return;
      if (final.status === 'done') {
        generation = null;
        world.loadSystem(systemId);
      } else {
        generation = { body, status: final, error: final.status === 'failed' ? final.error : null };
      }
    } catch (e) {
      if (generation?.body === body) generation = { body, status: null, error: String(e) };
    }
  }
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
    if (generation.error) return t.viewer.generationFailed({ body: name(generation.body), error: generation.error });
    const job = generation.status;
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
  /** The sun's direction over the body shown, from its system: longitude and latitude, degrees. */
  const sunPlace = $derived.by(() => {
    const dir = sysSky?.sun;
    if (!dir) return null;
    return {
      longitudeDeg: (Math.atan2(dir[0], dir[2]) * 180) / Math.PI,
      latitudeDeg: (Math.asin(Math.max(-1, Math.min(1, dir[1]))) * 180) / Math.PI,
    };
  });
  // The latitude under the sun, as the system places it.
  const declination = $derived(sunPlace?.latitudeDeg ?? 0);
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
  // against the sky, the star and the orbits, and the globe turns on itself.
  $effect(() => view?.setStarsTurn(starsTurn, orbitsPlaying));
  // The star drawn at its place, at the centre of the planets' orbits.
  $effect(() => view?.setStarPlace(sysSky?.star ?? null));
  // Far enough out to see the moons of the shown body's planet whole.
  const reach = $derived(sys && sysBody ? systemReach(sys, sysBody.id) : null);
  $effect(() => view?.setGlobeReach(reach ?? 0));
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
  $effect(() => {
    // The planet in its moon's sky, by its own switches (Host planet).
    const parent = sysBody?.parent ?? null;
    const planetTilt = sysPlanet?.spin.axial_tilt_deg ?? 0;
    view?.setSkyBodies(
      (sysSky?.bodies ?? []).map((body, k) =>
        sysSky?.ids[k] === parent ? { ...body, show: hostShow, axialTiltDeg: planetTilt, rings: hostRings } : body,
      ),
    );
    view?.setShadowCasters(sysSky?.casters ?? []);
  });
  // The moons' orbits in the sky, seen from the body shown.
  const skyOrbitLoops = $derived(orbitLines && sysSky ? sysSky.orbits : []);
  // The body shown's own orbit dashed: the body slides along it.
  $effect(() => view?.setSkyOrbits(skyOrbitLoops, skyOrbitLoops.map((_, i) => i === sysSky?.ownOrbit)));
  // Follow the bodies of the sky on the screen while their labels show.
  $effect(() => {
    const current = view;
    if (!current) return;
    // The guides' labels (spin, host) follow with the spin axes.
    const follow = skyLabels || axesShown || smallBodies;
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
    return sysSky.ids.map((id) => {
      const body = sys.bodies.find((b) => b.id === id);
      if (!body) return null;
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
  /** The guides' labels: the spin, the host's distance, its pull and the body's speed. */
  const guideLabels = $derived.by((): { key: 'spin' | 'host' | 'pull' | 'velocity'; label: SkyLabel }[] => {
    const labels: { key: 'spin' | 'host' | 'pull' | 'velocity'; label: SkyLabel }[] = [];
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
      labels.push({ key: 'host', label: { name, detail: withUnit(formatInt(hostKm), 'km') } });
      labels.push({ key: 'pull', label: { name: t.viewer.hostPull, detail: withUnit(formatUpTo(pull, pull < 0.01 ? 4 : 3), 'm/s²') } });
      labels.push({ key: 'velocity', label: { name: t.viewer.hostOrbitSpeed, detail: withUnit(formatUpTo(speed, 2), 'km/s') } });
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
    season = ((day % clockYear) + clockYear) % clockYear / clockYear;
    orbitDays = 0;
  }
  /** The display switches, as the icon bar groups them: light, surface, sky. */
  const toggleGroups = $derived<Toggle[][]>([
    [
      { key: 'sun', label: t.viewer.sunlight, icon: 'sun', on: sun, set: (v) => (sun = v) },
      { key: 'starLight', label: t.viewer.starLight, help: t.viewer.starLightHelp, icon: 'starlight', on: starLight, set: (v) => (starLight = v) },
      { key: 'atmosphere', label: t.viewer.atmosphere, icon: 'atmosphere', on: atmosphere, set: (v) => (atmosphere = v) },
      { key: 'stars', label: t.viewer.stars, help: t.viewer.starsHelp, icon: 'sparkles', on: stars, set: (v) => (stars = v) },
    ],
    [
      ...(giant
        ? [{ key: 'drift', label: t.viewer.bandDrift, help: t.viewer.bandDriftHelp, icon: 'drift' as const, on: cloudDrift, set: (v: boolean) => (cloudDrift = v) }]
        : [
            { key: 'clouds', label: t.viewer.clouds, icon: 'clouds' as const, on: clouds, set: (v: boolean) => (clouds = v) },
            { key: 'drift', label: t.viewer.drift, icon: 'drift' as const, on: cloudDrift, set: (v: boolean) => (cloudDrift = v) },
          ]),
      { key: 'grid', label: t.viewer.grid, icon: 'graticule', on: graticule, set: (v) => (graticule = v) },
      { key: 'tropics', label: t.viewer.tropics, help: t.viewer.tropicsHelp, icon: 'tropics', on: referenceLines, set: (v) => (referenceLines = v) },
      { key: 'sea', label: t.viewer.depthContours, help: t.viewer.depthContoursHelp, icon: 'depth', on: seaContours, set: (v) => dataReady && (seaContours = v) },
      { key: 'land', label: t.viewer.elevationContours, help: t.viewer.elevationContoursHelp, icon: 'contours', on: landContours, set: (v) => dataReady && (landContours = v) },
    ],
    [
      { key: 'labels', label: t.viewer.skyLabels, help: t.viewer.skyLabelsHelp, icon: 'tag', on: skyLabels, set: (v) => (skyLabels = v) },
      ...(sys && sys.bodies.length > 1
        ? [{ key: 'orbits', label: t.viewer.orbitLines, help: t.viewer.orbitLinesHelp, icon: 'orbit' as const, on: orbitLines, set: (v: boolean) => (orbitLines = v) }]
        : []),
      ...(sys && ((sys.belts?.length ?? 0) > 0 || sys.bodies.some((b) => b.trojans))
        ? [{ key: 'small', label: t.viewer.smallBodies, help: t.viewer.smallBodiesHelp, icon: 'trojans' as const, on: smallBodies, set: (v: boolean) => (smallBodies = v) }]
        : []),
      { key: 'axes', label: t.viewer.spinAxes, help: t.viewer.spinAxesHelp, icon: 'axis', on: axesShown, set: (v) => (axesShown = v) },
      {
        key: 'hexes',
        label: t.viewer.hexes,
        icon: 'hex',
        on: hexOn,
        set: (v) => {
          if (worldId && sysBody?.kind === 'planet') hexOn = v;
        },
      },
    ],
  ]);
  /** The panels of the planet's dock. */
  const dockPanels = $derived<DockPanel[]>([
    ...(info ? [{ id: 'info', title: t.viewer.infoPanel, icon: 'info' as const, content: info }] : []),
    { id: 'map', title: t.viewer.mapPanel, icon: 'planet', content: mapPanel },
    { id: 'light', title: t.viewer.lightPanel, icon: 'sun', content: lightPanel },
    ...(epochs && epochStops.length > 1 ? [{ id: 'epoch', title: t.epochs.slider, icon: 'clock' as const, content: epochPanel }] : []),
    ...(sysBody?.parent ? [{ id: 'host', title: t.viewer.host, icon: 'host' as const, content: hostPanel }] : []),
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
    let last = performance.now();
    let frame = requestAnimationFrame(function step(now) {
      season = (season + (now - last) / 1000 / YEAR_SECONDS) % 1;
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
    const columns = hexColumns;
    const riverFlow = Math.round(10 ** hexRiverLog);
    if (!current || !id || !hexOn) {
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
      retryWhileBusy((signal) => api.hex(id, { columns, river_flow_km2: riverFlow }, signal), {
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
      starPane?.resetView();
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
    {#if sun && orbitPeriod && sysSky}
        <p class="muted">
          {t.viewer.orbitDay({
            days: formatNumber(orbitDays % orbitPeriod, orbitPeriod < 10 ? 2 : 1),
            period: formatNumber(orbitPeriod, orbitPeriod < 10 ? 2 : 1),
          })}{#if sysBody?.kind === 'moon'}<br />{sysBody.spin.locked
              ? t.viewer.moonSun
              : t.viewer.moonSunSpin({ hours: formatNumber(sysBody.spin.period_hours, 1) })}{/if}
        </p>
    {/if}
  </div>
{/snippet}
{#snippet hexPanel()}
  <div class="panel-body">
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
{#snippet hostPanel()}
  <div class="panel-body">
        <!-- The planet in this body's sky has switches of its own. -->
        <fieldset class="host" title={t.viewer.hostHelp}>
          <legend>{t.viewer.host}</legend>
          <label class="check"><input type="checkbox" bind:checked={hostAtmosphere} /> {t.viewer.atmosphere}</label>
          <label class="check"><input type="checkbox" bind:checked={hostClouds} /> {t.viewer.clouds}</label>
          <label class="check"><input type="checkbox" bind:checked={hostGrid} /> {t.viewer.grid}</label>
          <label class="check"><input type="checkbox" bind:checked={hostTropics} /> {t.viewer.tropics}</label>
        </fieldset>
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
    {@const guides = axesShown ? guideLabels : []}
    {@const marks = smallSky ? skyPositions.marks : []}
    <!-- A dashed ring round each trojan swarm, too faint to see. -->
    {#each marks.slice(0, smallSky?.swarms.length ?? 0) as mark, k (k)}
      {#if mark}
        <span class="swarm-ring" style:left="{mark.x}px" style:top="{mark.y}px"></span>
      {/if}
    {/each}
    <SkyLabels
      positions={[...(skyLabels ? skyPositions.bodies : []), ...guides.map((g) => skyPositions!.guides[g.key]), ...marks]}
      labels={[...(skyLabels ? skyLabelList : []), ...guides.map((g) => g.label), ...smallLabels.slice(0, marks.length)]}
      star={skyLabels ? skyPositions.star : null}
      {starLabel}
    />
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
    />
  {/if}

  <div class="toolbar" role="toolbar" aria-label={t.viewer.tools}>
    <select
      class="body-pick"
      aria-label={t.viewer.body}
      title={t.viewer.bodyHelp}
      value={world.subject === 'star' ? 'star' : (world.systemBody ?? '')}
      onchange={(e) => pickBody(e.currentTarget.value)}
      disabled={!sys}
    >
      {#if sys}
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
      <ToggleBar label={t.viewer.display} groups={toggleGroups} />
    </div>
    <Dock kind="planet" panels={dockPanels} defaultActive="map" />
    <PlaybackBar
      bind:playing={() => clockPlaying, (v) => setClockPlaying(v)}
      speed={sysSky && orbitPeriod
        ? {
            value: orbitSpeed,
            min: ORBIT_SPEED_MIN,
            max: ORBIT_SPEED_MAX,
            text: t.viewer.orbitSpeedRate({ time: simulatedSecond((orbitPeriod / ORBIT_SECONDS) * 2 ** orbitSpeed) }),
            help: t.viewer.orbitSpeedHelp,
            set: (v) => (orbitSpeed = v),
          }
        : null}
      time={clockYear
        ? {
            value: sysDays % clockYear,
            max: clockYear,
            text: t.playback.dayOfYear({ day: formatInt(Math.floor(sysDays % clockYear) + 1), year: formatInt(Math.round(clockYear)) }),
            help: t.playback.yearHelp,
            set: setClockDay,
          }
        : null}
      clock={locked ? t.viewer.lockedSun : ''}
      jumps={clockYear ? [{ key: 'start', label: t.playback.toStart, icon: 'start', run: () => setClockDay(0) }] : []}
    />
  </div>

  {#if dataReady && shown !== 'biomes'}
    {@const windShown = giant !== null && shown === 'moisture'}
    {@const topsShown = giant !== null && shown === 'elevation'}
    {@const ramp = windShown ? t.legends.wind : topsShown ? t.legends.cloudTops : t.legends[shown]}
    {@const range = giant && shown === 'temperature' ? giantTemperatureRange(giant) : null}
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="legend" role="region" onpointerdown={dragLegend} onclickcapture={legendClick} ondblclick={legendHome} title={t.viewer.legendDrag} style:left={legendAt ? `clamp(8px, ${legendAt.left}px, calc(100% - 248px))` : null} style:bottom={legendAt ? `clamp(8px, ${legendAt.bottom}px, calc(100% - 48px))` : null} aria-label={t.viewer.legend}>
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
    <div class="legend biomes" role="region" onpointerdown={dragLegend} onclickcapture={legendClick} ondblclick={legendHome} title={t.viewer.legendDrag} style:left={legendAt ? `clamp(8px, ${legendAt.left}px, calc(100% - 248px))` : null} style:bottom={legendAt ? `clamp(8px, ${legendAt.bottom}px, calc(100% - 48px))` : null} aria-label={t.viewer.biomeLegend}>
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
                <span class="swatch" style:background={hexOn ? entry.biome.color : surfaceColour(entry.biome)}></span>
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
        <button onclick={() => generation && generateShown(generation.body)}>{t.viewer.generationRetry}</button>
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
  fieldset.host {
    margin: 4px 0 0;
    padding: 4px 8px 6px;
    border: 1px solid var(--border);
    border-radius: 6px;
    display: grid;
    gap: 2px;
  }
  fieldset.host legend {
    font-size: 0.8rem;
    color: var(--muted);
    padding: 0 4px;
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
    pointer-events: auto;
  }
  .gizmo {
    position: absolute;
    top: 4px;
    right: 12px;
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
  .value {
    font-variant-numeric: tabular-nums;
    color: var(--muted);
  }
  .legend {
    position: absolute;
    /* Bottom left, above the playback bar and right of the icon bar (12 px
       from the edge, 40 px wide), so that a long biome list never covers
       it; the dock holds the right. */
    left: 64px;
    bottom: 76px;
    width: 240px;
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
