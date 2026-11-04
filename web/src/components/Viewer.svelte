<script lang="ts">
  import { errorText } from '../lib/i18n/errors';
  import { t } from '../lib/i18n/index.svelte';
  import { biomeName } from '../lib/i18n/names.svelte';
  import { onDestroy, onMount, untrack } from 'svelte';
  import { ApiFailure, api, worldFile } from '../lib/api/client';
  import type { HexMap } from '../lib/api/types/HexMap';
  import { linkedChanges, type LinkedDisplay } from '../lib/display';
  import { downloadJson } from '../lib/download';
  import { formatArea, formatKm, formatNumber, formatShare, saveBlob, surfaceColour } from '../lib/format';
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
  import { periapsisAngle, SUN_SYSTEM } from '../lib/viewer/starSystem';
  import { type CameraState, PlanetView, SUN_STAR, type ViewMode } from '../lib/viewer/PlanetView';
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
  let starPane = $state<ReturnType<typeof StarPane> | null>(null);
  let starLight = $state(saved.starLight);
  let clouds = $state(saved.clouds);
  let cloudDrift = $state(saved.cloudDrift);
  let sun = $state(saved.sun);
  let season = $state(saved.season);
  let sunLongitude = $state(saved.sunLongitude);
  let playing = $state(false);
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
  const epochs = $derived(meta ? epochRange(meta) : null);
  /** The slider's stops, from the past (left) to the future (right). */
  const epochStops = $derived(epochs ? epochSteps(epochs).reverse() : []);
  const epoch = $derived(epochs ? nearestStep(epochStops, epochChoice) : 0);
  const epochShown = $derived(epochPreview ?? epoch);
  const layers = $derived(world.layers);
  const dataReady = $derived(layers !== null);
  function available(needs: Needs): boolean {
    return (
      needs === 'none' ||
      (needs === 'extra' && layers?.extra != null) ||
      (needs === 'floodplain' && layers?.extra != null && meta?.layers.floodplain != null)
    );
  }
  /** The layer drawn: the chosen one when this world has it, the biomes otherwise. */
  const shown = $derived.by((): Layer => {
    const option = LAYERS.find((l) => l.id === layer);
    return dataReady && option && available(option.needs) ? layer : 'biomes';
  });
  const declination = $derived((meta?.axial_tilt_deg ?? 23.44) * Math.sin(2 * Math.PI * season));
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
  $effect(() => view?.setCloudsVisible(clouds));
  $effect(() => view?.setCloudsDrifting(cloudDrift));
  $effect(() => {
    // No detail while the slider moves: the reduced preview shows through.
    view?.setDetail(
      worldId && meta && revision !== null && epochPreview === null
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
  $effect(() => view?.setSeaContours(seaContours));
  $effect(() => view?.setLandContours(landContours));
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
    view?.setLighting(
      !sun
        ? { kind: 'headlight' }
        : locked
          ? { kind: 'sun', season: 0, longitudeDeg: 180, axialTiltDeg: 0 }
          : { kind: 'sun', season, longitudeDeg: sunLongitude, axialTiltDeg: meta?.axial_tilt_deg ?? 23.44 },
    );
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
    if (link.camera) {
      applyCamera(link.camera);
    }
  }

  /** Saves the current view as a PNG. */
  export async function screenshot(): Promise<void> {
    const blob = await view?.screenshot();
    if (blob && worldId) {
      saveBlob(blob, `${worldId}-${shown}-${mode}.png`);
    } else {
      toasts.error(t.viewer.screenshotFailed);
    }
  }

  function downloadHex(): void {
    if (hexMap && worldId) {
      downloadJson(hexMap, `${worldId}-hex-${hexMap.columns}.json`);
    }
  }

  function selectLayer(index: number): void {
    const option = LAYERS[index];
    if (option && available(option.needs)) {
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
  {#if subject === 'star' && worldId}
    <StarPane
      bind:this={starPane}
      star={meta?.star ?? SUN_SYSTEM}
      omega={periapsisAngle(meta?.seed ?? 0)}
      bind:animated={starAnimated}
      bind:distances={starDistances}
    />
  {/if}

  <div class="toolbar" role="toolbar" aria-label={t.viewer.tools}>
    <div class="segmented" role="group" aria-label={t.viewer.target}>
      <button
        class:active={subject === 'planet'}
        aria-pressed={subject === 'planet'}
        onclick={() => (subject = 'planet')}
        title={t.viewer.planetViewHelp}>{t.viewer.planetView}</button
      >
      <button
        class:active={subject === 'star'}
        aria-pressed={subject === 'star'}
        onclick={() => (subject = 'star')}
        disabled={!worldId}
        title={t.viewer.starViewHelp}>{t.viewer.starView}</button
      >
    </div>
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
            <option value={option.id} disabled={dataReady && !available(option.needs)}
              >{i + 1}. {t.viewer.layers[option.id]}{!dataReady || available(option.needs) ? '' : t.viewer.notInWorld}</option
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
      <label class="check"><input type="checkbox" bind:checked={atmosphere} /> {t.viewer.atmosphere}</label>
      <label class="check" title={t.viewer.starsHelp}><input type="checkbox" bind:checked={stars} /> {t.viewer.stars}</label>
      <label class="check"><input type="checkbox" bind:checked={clouds} /> {t.viewer.clouds}</label>
      {#if clouds}
        <label class="check sub"><input type="checkbox" bind:checked={cloudDrift} /> {t.viewer.drift}</label>
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
      <label class="check">
        <input type="checkbox" bind:checked={hexOn} disabled={!worldId} />
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
    {@const ramp = t.legends[shown]}
    <div class="legend" role="region" aria-label={t.viewer.legend}>
      <strong>{ramp.label}</strong>
      <div class="bar" style:background={cssGradient(shown)}></div>
      <div class="ends">
        <span>{ramp.min}</span>
        <span>{shown === 'coast' && layers?.extra ? t.viewer.coastCap({ km: formatKm(layers.extra.coastMaxKm) }) : ramp.max}</span>
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
