<script lang="ts">
  import { errorText } from '../lib/i18n/errors';
  import { t } from '../lib/i18n/index.svelte';
  import { biomeName } from '../lib/i18n/names.svelte';
  import { onDestroy, onMount, untrack } from 'svelte';
  import { ApiFailure, api, worldFile } from '../lib/api/client';
  import type { HexMap } from '../lib/api/types/HexMap';
  import { linkedChanges, type LinkedDisplay } from '../lib/display';
  import { downloadJson } from '../lib/download';
  import { formatArea, formatInt, formatKm, formatNumber, formatShare, formatUpTo, saveBlob, surfaceColour, withUnit } from '../lib/format';
  import type { LonLat } from '../lib/geo';
  import type { ViewLink } from '../lib/link';
  import type { LoadProgress } from '../lib/progress';
  import { coastline } from '../lib/sample';
  import { seaIsLiquid } from '../lib/sea';
  import { retryWhileBusy } from '../lib/retry';
  import { debouncedSetting, loadSetting } from '../lib/settings';
  import { UNITS, unitTooltip } from '../lib/units';
  import { epochFile, epochRange, epochSteps, formatEpoch, nearestStep } from '../lib/epochs';
  import { altAllowed, shortcutFor, shortcutHelp, stepRelief } from '../lib/shortcuts';
  import { cloudTint, skyFromWorld } from '../lib/sky';
  import { toasts } from '../lib/toasts.svelte';
  import { cssGradient } from '../lib/viewer/colormaps';
  import { vegetationShares } from '../lib/vegetation';
  import StarPane from './StarPane.svelte';
  import SkyLabels, { type SkyLabel } from './SkyLabels.svelte';
  import { presetName } from '../lib/i18n/names.svelte';
  import MoonPane from './MoonPane.svelte';
  import { bodyFileName, bodyKey, moonsOf, parseBodyKey, systemBodies, type Body } from '../lib/moons';
  import {
    bodyTurn,
    giantSystemSky,
    halo,
    joinSkies,
    moonSky,
    moonSpinHours,
    moonSystemSky,
    orbitLoops,
    planetSky,
    siblingOrbitLoops,
    siblingSky,
    spinAxes,
    sunOnMoon,
  } from '../lib/skyBodies';
  import type { GiantSystem } from '../lib/api/types/GiantSystem';
  import type { StarInfo } from '../lib/api/types/StarInfo';
  import { periapsisAngle, SUN_SYSTEM, withOrbitDefaults } from '../lib/viewer/starSystem';
  import { type CameraState, giantTemperatureRange, PlanetView, SUN_STAR, type ViewMode } from '../lib/viewer/PlanetView';
  import type { Layer } from '../lib/viewer/shaders';
  import type { LoadedWorld } from '../lib/world.svelte';
  import type { WorldNames } from '../lib/api/types/WorldNames';
  import { moonName, worldName } from '../lib/names';
  import LoadingBar from './LoadingBar.svelte';

  interface Props {
    world: LoadedWorld;
    /** Its revision (modification time), which keys the cached detail tiles. */
    revision: number | null;
    /** The names of the world's system, from its summary. */
    names?: WorldNames | null;
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
  }

  let {
    world,
    revision,
    names = null,
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
  /** mapgen::hex::DEFAULT_RIVER_FLOW_KM2, on the log10 slider. */
  const DEFAULT_RIVER_LOG = Math.log10(500_000);
  /** Wait before asking a server busy generating for the hex grid again, ms. */
  const HEX_RETRY_MS = 5000;

  /** Display preferences kept between visits. */
  const saved = loadSetting('display', {
    mode: 'globe' as string,
    layer: 'biomes' as string,
    relief: 0,
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
  let relief = $state(saved.relief);
  let graticule = $state(saved.graticule);
  let referenceLines = $state(saved.referenceLines);
  // `contours` keeps its saved name: it was the only (depth) contour toggle.
  let seaContours = $state(saved.contours);
  let landContours = $state(saved.landContours);
  let atmosphere = $state(saved.atmosphere);
  let stars = $state(saved.stars);
  /** What the viewer shows: the planet, or its star and system. */
  let subject = $state<'planet' | 'star'>('planet');
  let starAnimated = $state(false);
  let starDistances = $state(false);
  let starOrbit = $state(true);
  let starZone = $state(true);
  let starProminences = $state(true);
  let starGiants = $state(true);
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
  let axesShown = $state(saved.spinAxes);
  $effect(() => view?.setSpinAxes(axesShown));
  const hostShow = $derived({ atmosphere: hostAtmosphere, clouds: hostClouds, grid: hostGrid, tropics: hostTropics });
  /** Labels on the bodies of the sky, and where those are on the screen. */
  let skyLabels = $state(saved.skyLabels);
  let skyPositions = $state<{ bodies: ({ x: number; y: number } | null)[]; star: { x: number; y: number } | null } | null>(null);
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
  /** The moons of the world loaded (whichever body is shown), nearest first. */
  const moons = $derived(moonsOf(world.worldMeta));
  const moonSystem = $derived(world.worldMeta?.moons ?? null);
  /** The moon shown, or null for the world itself. */
  const shownMoon = $derived(world.moon === null ? null : (moons[world.moon - 1] ?? null));
  /** The bodies of the world's system the planet view can show, in the system's order. */
  const bodies = $derived(systemBodies(world.worldMeta));
  /** Whether the body shown is the giant or one of its other moons, not the world's own system. */
  const onHostSystem = $derived(world.body.kind === 'host' || world.body.kind === 'sibling');
  /** Shows a body of the world's system in the planet view. */
  function showBody(body: Body): void {
    subject = 'planet';
    world.showBody(body);
  }
  /** `U`: each body of the system in turn (the star aside, on `O`). */
  function stepBody(): void {
    if (bodies.length < 2) return;
    const at = bodies.findIndex((b) => bodyKey(b) === bodyKey(world.body));
    showBody(bodies[(at + 1) % bodies.length]!);
  }
  /** The body picker's choice: the star, or a body of the system. */
  function pickBody(key: string): void {
    if (key === 'star') {
      if (worldId) subject = 'star';
      return;
    }
    const body = parseBodyKey(key);
    if (body) showBody(body);
  }
  /** A body's line in the picker: its name, its role, and a moon's size. */
  function bodyOption(body: Body): string {
    const km = (r: number) => `${formatUpTo(r, r < 100 ? 1 : 0)}${t.format.unitSpace}km`;
    const host = world.worldMeta?.host;
    switch (body.kind) {
      case 'world':
        return t.viewer.bodyOption({ name: names ? worldName(names) : (world.worldId ?? ''), role: t.viewer.roleWorld });
      case 'moon': {
        const moon = moons[body.n - 1];
        const name = (names && moonName(names, body.n)) ?? moon?.name ?? '';
        return moon ? t.moons.option({ name, size: km(moon.radius_km) }) : name;
      }
      case 'host':
        return t.viewer.bodyOption({
          name: names?.host ?? t.starView.giantKinds[host?.giant.kind ?? 'gas'] ?? '',
          role: t.viewer.roleGiant,
        });
      case 'sibling': {
        const sibling = host?.siblings?.[body.n - 1];
        const name = names?.siblings[body.n - 1] ?? t.viewer.skySibling({ index: body.n });
        return sibling ? t.moons.option({ name, size: km(sibling.radius_km) }) : name;
      }
    }
  }
  // Only the world has a hex grid (the server resamples worlds only).
  $effect(() => {
    if (world.body.kind !== 'world') hexOn = false;
  });
  /**
   * The system the star view shows: the previous world's while the next
   * one's metadata loads, so the view does not flash the Sun's in between.
   */
  let shownSystem: { star: StarInfo; omega: number; giants: GiantSystem | null } | null = null;
  const system = $derived.by(() => {
    // The world's, also while one of its moons is shown.
    const planet = world.worldMeta ?? (world.body.kind === 'world' ? meta : null);
    if (planet) {
      shownSystem = {
        star: withOrbitDefaults(planet.star ?? SUN_SYSTEM),
        omega: periapsisAngle(planet.seed),
        giants: planet.giants ?? null,
      };
    }
    return shownSystem ?? { star: withOrbitDefaults(SUN_SYSTEM), omega: 0, giants: null };
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
      view = new PlanetView(canvas);
      if (pendingCamera) {
        view.setCamera(pendingCamera);
        pendingCamera = null;
      }
      view.onPick = (point) => onPick(point);
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
    view?.setData(layers, meta ? coastline(meta) : 0, meta ? seaIsLiquid(meta) : true);
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
  /**
   * The clock of the moons: their planet's year, the time on their
   * orbits (the season, its star's place in the planet's sky as the
   * planet view has it, and the days past it), in moonDirection's terms.
   */
  /**
   * The planet's own day, hours: its rotation, or its orbit round the
   * giant it is locked to.
   */
  const planetDayHours = $derived.by(() => {
    const planet = world.worldMeta;
    if (planet?.host) return planet.host.period_days * 24;
    return planet?.moons?.planet_day_hours ?? 24;
  });
  /**
   * The longitude under the sun on the planet, degrees: the Sun longitude
   * slider, the planet turning on itself from there at its own day as the
   * orbit time goes (the sun going west), so that its day passes while the
   * moons go round; still on a planet locked to its star.
   */
  const planetHour = $derived.by(() => {
    const planet = world.worldMeta;
    if (planet?.star?.tidally_locked) return 180;
    const yearDays = planet?.star?.year_days ?? 365.25;
    const turned = sunLongitude - (bodyTurn(orbitDays, planetDayHours, yearDays) * 180) / Math.PI;
    return ((turned % 360) + 360) % 360;
  });
  const moonClock = $derived.by(() => {
    const planet = world.worldMeta;
    const yearDays = planet?.star?.year_days ?? 365.25;
    const planetLocked = planet?.star?.tidally_locked ?? false;
    return {
      yearDays,
      season: (planetLocked ? 0 : season) + orbitDays / yearDays,
      longitude: (planetHour * Math.PI) / 180,
    };
  });
  $effect(() => {
    const id = world.worldId;
    untrack(() => {
      if (id === orbitWorld) return;
      orbitWorld = id;
      orbitDays = orbitLink ?? 0;
      orbitsPlaying = false;
    });
  });
  /** The period the orbit slider spans: of the moon shown, or of the largest moon. */
  const orbitPeriod = $derived.by(() => {
    if (shownMoon) return shownMoon.period_days;
    const largest = moons.reduce<(typeof moons)[number] | null>(
      (best, m) => (m.migration !== 'locked' && (!best || m.radius_km > best.radius_km) ? m : best),
      null,
    );
    return largest?.period_days ?? null;
  });
  // On a moon, the sun stands where the moon's place on its orbit puts it,
  // at the latitude the planet's season and tilt give it (the moon's orbit
  // inclined to the planet's equator), not the moon's own tilt.
  const moonSun = $derived(
    shownMoon && world.moon !== null
      ? sunOnMoon(
          shownMoon,
          world.moon - 1,
          moonClock.season,
          moonClock.yearDays,
          moonClock.longitude,
          world.worldMeta?.star?.tidally_locked ? 0 : (world.worldMeta?.axial_tilt_deg ?? 0),
        )
      : null,
  );
  // The latitude under the sun: on a moon, where its orbit puts it (see moonSun).
  const declination = $derived(
    moonSun?.latitudeDeg ?? (meta?.axial_tilt_deg ?? 23.44) * Math.sin(2 * Math.PI * season),
  );
  $effect(() => {
    view?.setLighting(
      !sun
        ? { kind: 'headlight' }
        : moonSun !== null
          ? {
              kind: 'sun',
              season,
              longitudeDeg: moonSun.longitudeDeg,
              axialTiltDeg: 0,
              latitudeDeg: moonSun.latitudeDeg,
            }
          : locked
            ? { kind: 'sun', season: 0, longitudeDeg: 180, axialTiltDeg: 0 }
            : {
                kind: 'sun',
                season,
                // On a body of the planet's own: the planet's hour; else (the
                // giant, its other moons) the slider's.
                longitudeDeg: onHostSystem ? sunLongitude : planetHour,
                axialTiltDeg: meta?.axial_tilt_deg ?? 23.44,
              },
    );
  });
  // The other bodies of the system in the sky, with their phases, and
  // their shadows on the ground (eclipses): the moons from their planet,
  // the planet and its other moons from one of them, each with its map.
  // The sky of the giant a world orbits, or of one of its other moons:
  // the world and the giant's moons round it, and their orbits.
  const giantSky = $derived.by(() => {
    const planet = world.worldMeta;
    const host = planet?.host;
    const id = world.worldId;
    const body = world.body;
    if (!planet || !host || !id || (body.kind !== 'host' && body.kind !== 'sibling')) return null;
    return giantSystemSky(
      host,
      planet.circumference_km / (2 * Math.PI),
      (host.giant.radius_earth * 40075) / (2 * Math.PI),
      body.kind === 'host' ? 'host' : body.n - 1,
      moonClock.season,
      moonClock.yearDays,
      bodyTurn(orbitDays, host.giant.rotation_hours, host.period_days),
      (k) =>
        worldFile(
          id,
          k === 'world'
            ? 'raw_data/surface.png'
            : k === 'host'
              ? 'host/raw_data/surface.png'
              : `host/siblings/${k + 1}/raw_data/surface.png`,
          revision,
        ),
      { show: hostShow, axialTiltDeg: planet.axial_tilt_deg },
    );
  });
  const sky = $derived.by(() => {
    const planet = world.worldMeta;
    if (onHostSystem) return giantSky;
    if (!planet) return null;
    const planetRadiusKm = planet.circumference_km / (2 * Math.PI);
    const planetId = world.worldId;
    const moonMap = (k: number) =>
      planetId ? worldFile(planetId, `moons/${k + 1}/raw_data/surface.png`, revision) : null;
    const { yearDays, season: moonSeason, longitude } = moonClock;
    if (shownMoon && world.moon !== null) {
      const map = planetId ? worldFile(planetId, 'raw_data/surface.png', revision) : null;
      // The planet with its clouds and its air, a grid and its tropics,
      // by its own switches (Host planet), not the moon's.
      const cloudFile = planet.layers.clouds?.file;
      const look = {
        clouds: planetId && cloudFile ? worldFile(planetId, `raw_data/${cloudFile}`, revision) : null,
        cloudTint: cloudTint(planet),
        halo: halo(skyFromWorld(planet)),
        show: hostShow,
        axialTiltDeg: planet.axial_tilt_deg,
      };
      // The planet turns on itself under its moon as its hour goes (see
      // planetHour: its frame turns under the moons; a binary's moon turns
      // with it).
      return moonSystemSky(moons, world.moon - 1, planetRadiusKm, map, moonMap, moonSeason, yearDays, longitude, look);
    }
    // The moon of a giant: the giant hangs over the face the world keeps
    // towards it (longitude 180°), its bands from its own world in host/.
    if (world.moon === null && planet.host && world.worldId) {
      const host = planet.host;
      const map = worldFile(world.worldId, 'host/raw_data/surface.png', revision);
      const giant = moonSky(
        { radius_km: planetRadiusKm, semi_major_axis_km: host.distance_km },
        (host.giant.radius_earth * 40075) / (2 * Math.PI),
        map,
        { show: hostShow, axialTiltDeg: planet.axial_tilt_deg },
        [0, 0, -1],
        // It turns on itself (its storms with it) as the orbit slider plays.
        spinAxes(bodyTurn(orbitDays, host.giant.rotation_hours, host.period_days)),
      );
      // The giant's other regular moons beside it, with their phases.
      const siblings = siblingSky(
        host.siblings ?? [],
        planetRadiusKm,
        host.distance_km,
        host.period_days,
        // The orbit slider moves them too.
        moonClock.season,
        planet.star?.year_days ?? 365.25,
        // Each a world of its own (none in worlds made before them: plain disks).
        (k) => (world.worldId ? worldFile(world.worldId, `host/siblings/${k + 1}/raw_data/surface.png`, revision) : null),
      );
      return joinSkies(giant, siblings);
    }
    if (world.moon !== null || moons.length === 0) return null;
    return planetSky(moons, planetRadiusKm, moonSeason, yearDays, longitude, moonMap);
  });
  // A moon locked to its planet (or a world locked to its giant) turns
  // once per orbit against the stars, its planet fixed in its sky: the star
  // field turns with its orbit, so the other bodies are seen to move against
  // the stars, not to circle the moon. A moon that is not locked turns once
  // per day of its own, its planet rising and setting.
  const starsTurn = $derived.by(() => {
    const { yearDays, season: moonSeason } = moonClock;
    const spin = shownMoon && world.moon !== null ? moonSpinHours(shownMoon) : null;
    const host = world.worldMeta?.host;
    const body = world.body;
    // The giant turns once per day of its own, its other moons once per orbit.
    const period =
      body.kind === 'host'
        ? (host?.giant.rotation_hours ?? 0) / 24
        : body.kind === 'sibling'
          ? (host?.siblings?.[body.n - 1]?.period_days ?? null)
          : spin !== null
            ? spin / 24
            : shownMoon && world.moon !== null
              ? shownMoon.period_days
              : (host?.period_days ?? null);
    // The planet itself (not a giant's moon): once per day of its own
    // against the stars, from the hour of its view (see planetHour).
    if (body.kind === 'world' && !host) {
      if (world.worldMeta?.star?.tidally_locked) return 0;
      const turns = (orbitDays * 24) / Math.max(planetDayHours, 1e-3);
      return -2 * Math.PI * (turns - Math.floor(turns));
    }
    if (period === null || period <= 0) return 0;
    const turns = (moonSeason * yearDays) / period;
    return -2 * Math.PI * (turns - Math.floor(turns));
  });
  // While the orbits play, the camera turns with the stars, so that the
  // globe is seen to turn on itself under a still sky.
  $effect(() => view?.setStarsTurn(starsTurn, orbitsPlaying));
  // Not aborted when the bodies move (every frame of the seasons): a map
  // still loading is kept, and a newer set of bodies drops an older one.
  $effect(() => {
    view?.setSkyBodies(sky?.bodies ?? []);
    view?.setShadowCasters(sky?.casters ?? []);
  });
  // The moons' orbits in the sky, seen from the body shown.
  const skyOrbitLoops = $derived.by(() => {
    const planet = world.worldMeta;
    if (!orbitLines || !planet) return [];
    if (onHostSystem) return giantSky?.orbits ?? [];
    if (world.moon === null && planet.host) {
      return siblingOrbitLoops(
        planet.host.siblings ?? [],
        planet.host.distance_km,
        planet.circumference_km / (2 * Math.PI),
      );
    }
    return orbitLoops(
      moons,
      world.moon === null ? null : world.moon - 1,
      moonClock.season,
      moonClock.yearDays,
      moonClock.longitude,
      planet.circumference_km / (2 * Math.PI),
    );
  });
  $effect(() => view?.setSkyOrbits(skyOrbitLoops));
  // Follow the bodies of the sky on the screen while their labels show.
  $effect(() => {
    const current = view;
    if (!current) return;
    current.onSkyPositions = skyLabels ? (positions) => (skyPositions = positions) : null;
    if (!skyLabels) skyPositions = null;
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
    const planet = world.worldMeta;
    if (!planet || !sky) return [];
    const km = (v: number) => withUnit(formatInt(v), 'km');
    const moonLabel = (m: (typeof moons)[number], k: number): SkyLabel => ({
      name: (names && moonName(names, k + 1)) ?? t.viewer.skyMoon({ name: m.name }),
      detail: t.viewer.skyOrbitFacts({ radius: km(m.radius_km), orbit: km(m.semi_major_axis_km) }),
    });
    if (shownMoon && world.moon !== null) {
      const self = shownMoon;
      const planetLabel: SkyLabel = {
        name: names ? worldName(names) : presetName(planet.preset),
        detail: t.viewer.skyFacts({
          radius: km(planet.circumference_km / (2 * Math.PI)),
          distance: km(self.semi_major_axis_km),
        }),
      };
      return [
        planetLabel,
        ...moons.flatMap((m, k) => (m === self ? [] : [moonLabel(m, k)])),
      ];
    }
    // From the giant or one of its other moons, in giantSystemSky's
    // order: the giant (from a moon), the world, then the siblings.
    const body = world.body;
    if (planet.host && (body.kind === 'host' || body.kind === 'sibling')) {
      const host = planet.host;
      const giantLabel: SkyLabel = {
        name: names?.host ?? t.starView.giantKinds[host.giant.kind] ?? '',
        detail: t.viewer.skyFacts({
          radius: withUnit(formatUpTo(host.giant.mass_earth / 317.8, 2), 'MJ'),
          distance: km(body.kind === 'sibling' ? (host.siblings?.[body.n - 1]?.distance_km ?? 0) : 0),
        }),
      };
      const worldLabel: SkyLabel = {
        name: names ? worldName(names) : presetName(planet.preset),
        detail: t.viewer.skyOrbitFacts({ radius: km(planet.circumference_km / (2 * Math.PI)), orbit: km(host.distance_km) }),
      };
      const siblingLabels = (host.siblings ?? []).flatMap((s, k) =>
        body.kind === 'sibling' && body.n - 1 === k
          ? []
          : [
              {
                name: names?.siblings[k] ?? t.viewer.skySibling({ index: k + 1 }),
                detail: t.viewer.skyOrbitFacts({ radius: km(s.radius_km), orbit: km(s.distance_km) }),
              },
            ],
      );
      return [...(body.kind === 'sibling' ? [giantLabel] : []), worldLabel, ...siblingLabels];
    }
    if (world.moon === null && planet.host) {
      const host = planet.host;
      return [
        {
          name: names?.host ?? t.starView.giantKinds[host.giant.kind] ?? '',
          detail: t.viewer.skyFacts({
            radius: withUnit(formatUpTo(host.giant.mass_earth / 317.8, 2), 'MJ'),
            distance: km(host.distance_km),
          }),
        },
        ...(host.siblings ?? []).map((s, k) => ({
          name: names?.siblings[k] ?? t.viewer.skySibling({ index: k + 1 }),
          detail: t.viewer.skyOrbitFacts({ radius: km(s.radius_km), orbit: km(s.distance_km) }),
        })),
      ];
    }
    return moons.map(moonLabel);
  });
  /** The heading of the moon panel: the moon's name, and its designation beside a given one. */
  const moonTitle = $derived.by((): string | null => {
    const n = world.moon;
    const moon = n === null ? undefined : names?.moons[n - 1];
    if (!moon) return null;
    return moon.name ? `${moon.name} · ${moon.designation}` : moon.designation;
  });
  const starLabel = $derived.by((): SkyLabel | null => {
    const star = world.worldMeta?.star;
    if (!star) return null;
    return {
      name: names ? `${names.star} · ${star.type}` : t.viewer.skyStar({ type: star.type }),
      detail: t.viewer.skyStarFacts({ distance: withUnit(formatUpTo(star.orbit_au, 2), t.viewer.au) }),
    };
  });

  // The orbit animation: one orbit of the slider's period every
  // ORBIT_SECONDS, paused while the canvas is hidden.
  $effect(() => {
    const period = orbitPeriod;
    if (!orbitsPlaying || !sun || !canvasVisible || !period) {
      return;
    }
    let last = performance.now();
    let frame = requestAnimationFrame(function step(now) {
      orbitDays = (orbitDays + ((now - last) / 1000 / ORBIT_SECONDS) * period) % (period * 1000);
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
    if (subject === 'star') {
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
        // The seasons need the sun: turn it on and play.
        if (!sun) {
          sun = true;
          playing = true;
        } else {
          playing = !playing;
        }
        break;
      case 'atmosphere':
        atmosphere = !atmosphere;
        break;
      case 'stars':
        stars = !stars;
        break;
      case 'starView':
        if (worldId) subject = subject === 'star' ? 'planet' : 'star';
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

<div class="viewer">
  <!-- Hidden (no size) in the star view: the planet view pauses meanwhile. -->
  <canvas bind:this={canvas} class:measuring class:hidden={subject === 'star'}></canvas>
  {#if subject === 'planet' && skyLabels && skyPositions}
    <SkyLabels positions={skyPositions.bodies} labels={skyLabelList} star={skyPositions.star} {starLabel} />
  {/if}
  {#if subject === 'planet' && shownMoon && moonSystem}
    <MoonPane moon={shownMoon} system={moonSystem} world={world.worldId ?? ''} title={moonTitle} />
  {/if}
  {#if subject === 'star' && worldId}
    <StarPane
      bind:this={starPane}
      star={system.star}
      {names}
      omega={system.omega}
      {moons}
      binary={moonSystem?.binary ?? false}
      planetRadiusKm={(world.worldMeta?.circumference_km ?? 40075) / (2 * Math.PI)}
      bind:animated={starAnimated}
      bind:distances={starDistances}
      bind:orbit={starOrbit}
      bind:zone={starZone}
      bind:prominences={starProminences}
      giantSystem={system.giants}
      worldGiant={world.worldMeta?.giant ?? world.worldMeta?.host?.giant ?? null}
      host={world.worldMeta?.host ?? null}
      bind:giants={starGiants}
    />
  {/if}

  <div class="toolbar" role="toolbar" aria-label={t.viewer.tools}>
    <select
      class="body-pick"
      aria-label={t.viewer.body}
      title={t.viewer.bodyHelp}
      value={subject === 'star' ? 'star' : bodyKey(world.body)}
      onchange={(e) => pickBody(e.currentTarget.value)}
      disabled={!worldId}
    >
      <option value="star"
        >{t.viewer.bodyOption({ name: names?.star ?? system.star.type, role: t.viewer.roleStar })}</option
      >
      {#each bodies as body (bodyKey(body))}
        <option value={bodyKey(body)}>{bodyOption(body)}</option>
      {/each}
    </select>
    <button onclick={resetView} disabled={!worldId} title={t.viewer.resetViewHelp}>{t.viewer.resetView}</button>
    {#if subject === 'planet'}
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
    {#if subject === 'planet' && epochs && epochStops.length > 1}
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
    {/if}
  </div>

  {#if subject === 'planet'}
  <div class="panel" role="region" aria-label={t.viewer.display}>
    <div class="row">
      <div class="segmented">
        <button class:active={mode === 'globe'} onclick={() => (mode = 'globe')} title={t.viewer.globeHelp}>{t.viewer.globe}</button>
        <button class:active={mode === 'map'} onclick={() => (mode = 'map')} title={t.viewer.mapHelp}>{t.viewer.map}</button>
      </div>
      <button class="fold" onclick={() => (panelOpen = !panelOpen)} aria-expanded={panelOpen}>
        {panelOpen ? t.viewer.hide : t.viewer.display}
      </button>
    </div>

    {#if panelOpen}
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
      <label>
        <span>{t.viewer.relief}</span>
        <input type="range" min="0" max="1" step="0.05" bind:value={relief} disabled={!dataReady} />
      </label>

      <label class="check"><input type="checkbox" bind:checked={sun} /> {t.viewer.sunlight}</label>
      <label class="check" title={t.viewer.starLightHelp}
        ><input type="checkbox" bind:checked={starLight} /> {t.viewer.starLight}</label
      >
      {#if sun && locked}
        <p class="muted">{t.viewer.lockedSun}</p>
      {:else if sun}
        <label>
          <span title={t.viewer.seasonHelp}>{t.viewer.season}</span>
          <input type="range" min="0" max="1" step="0.005" bind:value={season} />
          <button class="small" onclick={() => (playing = !playing)} aria-pressed={playing} title={t.viewer.playHelp}
            >{playing ? t.viewer.pause : t.viewer.play}</button
          >
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
      {#if sun && orbitPeriod && sky}
        <label>
          <span title={t.viewer.orbitsHelp}>{t.viewer.orbits}</span>
          <input
            type="range"
            min="0"
            max={orbitPeriod}
            step={orbitPeriod / 400}
            value={orbitDays % orbitPeriod}
            oninput={(e) => {
              const day = Number(e.currentTarget.value);
              orbitDays = Math.floor(orbitDays / orbitPeriod) * orbitPeriod + day;
            }}
          />
          <button
            class="small"
            onclick={() => (orbitsPlaying = !orbitsPlaying)}
            aria-pressed={orbitsPlaying}
            title={t.viewer.playOrbitsHelp}>{orbitsPlaying ? t.viewer.pause : t.viewer.play}</button
          >
        </label>
        <p class="muted">
          {t.viewer.orbitDay({
            days: formatNumber(orbitDays % orbitPeriod, orbitPeriod < 10 ? 2 : 1),
            period: formatNumber(orbitPeriod, orbitPeriod < 10 ? 2 : 1),
          })}{#if shownMoon}<br />{moonSpinHours(shownMoon) === null
              ? t.viewer.moonSun
              : t.viewer.moonSunSpin({ hours: formatNumber(moonSpinHours(shownMoon) ?? 0, 1) })}{/if}
        </p>
      {/if}
      <label class="check"><input type="checkbox" bind:checked={atmosphere} /> {t.viewer.atmosphere}</label>
      <label class="check" title={t.viewer.starsHelp}><input type="checkbox" bind:checked={stars} /> {t.viewer.stars}</label>
      {#if giant}
        <label class="check" title={t.viewer.bandDriftHelp}><input type="checkbox" bind:checked={cloudDrift} /> {t.viewer.bandDrift}</label>
      {:else}
        <label class="check"><input type="checkbox" bind:checked={clouds} /> {t.viewer.clouds}</label>
        {#if clouds}
          <label class="check sub"><input type="checkbox" bind:checked={cloudDrift} /> {t.viewer.drift}</label>
        {/if}
      {/if}
      <label class="check"><input type="checkbox" bind:checked={graticule} /> {t.viewer.grid}</label>
      <label class="check" title={t.viewer.tropicsHelp}>
        <input type="checkbox" bind:checked={referenceLines} /> {t.viewer.tropics}
      </label>
      <label class="check" title={t.viewer.depthContoursHelp}>
        <input type="checkbox" bind:checked={seaContours} disabled={!dataReady} /> {t.viewer.depthContours}
      </label>
      <label class="check" title={t.viewer.elevationContoursHelp}>
        <input type="checkbox" bind:checked={landContours} disabled={!dataReady} /> {t.viewer.elevationContours}
      </label>
      <label class="check" title={t.viewer.skyLabelsHelp}><input type="checkbox" bind:checked={skyLabels} /> {t.viewer.skyLabels}</label>
      {#if (shownMoon && world.moon !== null) || (world.worldMeta?.host && world.body.kind !== 'host')}
        <!-- The planet in this body's sky has switches of its own. -->
        <fieldset class="host" title={t.viewer.hostHelp}>
          <legend>{t.viewer.host}</legend>
          <label class="check"><input type="checkbox" bind:checked={hostAtmosphere} /> {t.viewer.atmosphere}</label>
          <label class="check"><input type="checkbox" bind:checked={hostClouds} /> {t.viewer.clouds}</label>
          <label class="check"><input type="checkbox" bind:checked={hostGrid} /> {t.viewer.grid}</label>
          <label class="check"><input type="checkbox" bind:checked={hostTropics} /> {t.viewer.tropics}</label>
        </fieldset>
      {/if}
      {#if moons.length > 0 || onHostSystem || (world.moon === null && (world.worldMeta?.host?.siblings?.length ?? 0) > 0)}
        <label class="check" title={t.viewer.orbitLinesHelp}>
          <input type="checkbox" bind:checked={orbitLines} /> {t.viewer.orbitLines}
        </label>
      {/if}
      <label class="check" title={t.viewer.spinAxesHelp}>
        <input type="checkbox" bind:checked={axesShown} /> {t.viewer.spinAxes}
      </label>
      <label class="check">
        <input type="checkbox" bind:checked={hexOn} disabled={!worldId || world.body.kind !== 'world'} />
        {t.viewer.hexes}
        {#if hexServerBusy}
          <small class="muted" title={t.viewer.serverBusyHelp}
            >{t.viewer.serverBusy}</small
          >
        {:else if hexBusy}
          <small class="muted">{t.viewer.computing}</small>
        {/if}
      </label>
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
    {/if}
  </div>

  {#if dataReady && shown !== 'biomes'}
    {@const windShown = giant !== null && shown === 'moisture'}
    {@const topsShown = giant !== null && shown === 'elevation'}
    {@const ramp = windShown ? t.legends.wind : topsShown ? t.legends.cloudTops : t.legends[shown]}
    {@const range = giant && shown === 'temperature' ? giantTemperatureRange(giant) : null}
    <div class="legend" role="region" aria-label={t.viewer.legend}>
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
    <div class="legend biomes" role="region" aria-label={t.viewer.biomeLegend}>
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

  {#if !worldId}
    <p class="overlay">{t.viewer.noWorld}</p>
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
    background: var(--panel);
    padding: 4px 10px;
  }
  .toolbar button.active {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--accent-text);
  }
  .panel {
    position: absolute;
    top: 12px;
    right: 12px;
    width: 240px;
    /* Scroll inside the panel rather than overflow a short viewer. */
    max-height: calc(100% - 24px);
    overflow-y: auto;
    display: grid;
    align-content: start;
    gap: 8px;
    background: var(--panel);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 10px;
    z-index: 1;
  }
  .row {
    display: flex;
    justify-content: space-between;
    gap: 8px;
  }
  .panel label {
    display: grid;
    grid-template-columns: 92px minmax(0, 1fr) auto;
    align-items: center;
    gap: 6px;
  }
  .panel label > select {
    min-width: 0;
  }
  .panel label.check {
    display: flex;
    gap: 6px;
  }
  .panel label.check.sub {
    padding-left: 18px;
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
    right: 12px;
    bottom: 12px;
    width: 240px;
    background: var(--panel);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 8px 10px;
    display: grid;
    gap: 4px;
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
    list-style: none;
    margin: 0;
    padding: 0;
    overflow-y: auto;
    max-height: min(40vh, 360px);
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
    width: min(320px, calc(100% - 24px));
    background: var(--panel-solid);
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
    grid-template-columns: auto 1fr;
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
  .overlay.error {
    color: var(--danger);
    padding: 0 24px;
    text-align: center;
  }
  /* The time slider: a row of its own under the tools, wherever they are. */
  .epoch {
    flex-basis: 100%;
    max-width: 460px;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 4px 10px;
    background: var(--panel);
    border: 1px solid var(--border);
    border-radius: 8px;
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
    background: var(--panel);
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
