// One world shown in a viewer pane: its metadata, decoded layers, surface
// statistics and rivers, loaded in that order (each step can take a while
// on large maps, and the earlier ones are useful on their own).
import { t } from './i18n/index.svelte';
import { api, peeked, sizeName, systemBodyId, worldFile } from './api/client';
import type { BodySizes } from './api/types/BodySizes';
import type { PresentSize } from './api/types/PresentSize';
import type { System } from './api/types/System';
import type { WorldMetadata } from './api/types/WorldMetadata';
import { epochFile } from './epochs';
import { MIN_FORMAT_VERSION } from './formatVersion';
import { loadEpochLayers, loadLayers, loadPeekedStats, type WorldLayers } from './layers';
import type { LoadProgress } from './progress';
import { RiverIndex } from './rivers';
import type { WorldStats } from './stats';

export class LoadedWorld {
  /**
   * The body shown, when it is generated: `systems/<system>/bodies/<body>`
   * (see systemBodyId). `meta` and the layers are this body's.
   */
  id = $state<string | null>(null);
  /** The planet of the body shown (itself, or a moon's planet), as an id like `id`. */
  worldId = $state<string | null>(null);
  /** The world's metadata (its star and moons), whichever body is shown. */
  worldMeta = $state.raw<WorldMetadata | null>(null);
  /** The world's biome statistics and rivers, whichever body is shown. */
  worldStats = $state.raw<WorldStats | null>(null);
  worldRivers = $state.raw<RiverIndex | null>(null);
  /** Why the planet's metadata could not be read while one of its moons is shown. */
  worldError = $state<string | null>(null);
  private planetController: AbortController | null = null;
  // Raw state: typed arrays and large JSON must not be proxied.
  meta = $state.raw<WorldMetadata | null>(null);
  layers = $state.raw<WorldLayers | null>(null);
  stats = $state.raw<WorldStats | null>(null);
  rivers = $state.raw<RiverIndex | null>(null);
  error = $state<string | null>(null);
  /**
   * The resolution shown, `WxH` (see sizeName), once listed: `meta`, the
   * layers, the tiles and the epochs are this resolution's. Null while the
   * resolutions of the body are listed, and when no body is loaded.
   */
  size = $state<string | null>(null);
  /** The resolutions of the body shown (`GET …/sizes`), once listed. */
  sizes = $state.raw<BodySizes | null>(null);
  /**
   * The revision of the resolution shown (`PresentSize.revision`), which
   * keys the cached files and tiles; null until it is listed.
   */
  revision = $state<number | null>(null);
  /** How far the download and decoding of the layers have got, while they load. */
  progress = $state.raw<LoadProgress | null>(null);
  /**
   * The epoch the layers show, in Ma before present (negative in the
   * future; see lib/epochs.ts): `layers` holds its elevation and biomes
   * once they have loaded, the present's other layers, and `stats` stay
   * the present's.
   */
  epochMa = $state(0);
  /** Whether the layers of the epoch asked for are loading. */
  epochLoading = $state(false);
  epochError = $state<string | null>(null);
  /**
   * The planetary system shown (docs/system-spec.md), when one is: its
   * file, the names given to its bodies, and the body shown (`c`, `c-1`).
   * A body not generated yet has no metadata nor layers: the view draws
   * its preview.
   */
  system = $state.raw<System | null>(null);
  systemId = $state<string | null>(null);
  systemNames = $state.raw<Record<string, string>>({});
  /** The bodies of the system that lost their files after a while unopened (see evicted.ts). */
  systemEvicted = $state.raw<string[]>([]);
  systemBody = $state<string | null>(null);
  /**
   * What the viewer shows: the body (`planet`), or the star and its system
   * round the body's planet. A system opens on its star unless a body is
   * asked for.
   */
  subject = $state<'planet' | 'star'>('planet');
  /**
   * Which view of the star `subject === 'star'` shows: the system round it
   * (StarPane) or the star alone, close up (StarSurfacePane). The system
   * view by default and with every new system; `loadSystem(id, 'star')`
   * opens the star view (`#system=<id>&body=star`).
   */
  starMode = $state<'system' | 'star'>('system');
  /**
   * The `body` of the address: the body shown, `'star'` in the star view,
   * none in the system view. `'star'` is never a body id: the server
   * reserves it for the star (`STAR`, see SYSTEM_NAME_KEY), and bodies are
   * lettered (`b`, `c-1`).
   */
  linkBody = $derived(this.subject === 'star' ? (this.starMode === 'star' ? 'star' : null) : this.systemBody);
  /**
   * The planets of the system shown whose cards are pinned in the star
   * view, by body id, in pinning order. They last for the session while
   * the same system is shown (opening a planet and coming back keeps
   * them); another system or a lone world clears them.
   */
  pinnedPlanets = $state<string[]>([]);
  /** The body of the system shown, from its file. */
  shownSystemBody = $derived(this.system?.bodies.find((b) => b.id === this.systemBody) ?? null);
  private systemController: AbortController | null = null;
  /** The layers as loaded, at the present. */
  private present: WorldLayers | null = null;
  /** Aborts the current load: its downloads and its decoding worker. */
  private controller: AbortController | null = null;
  private epochController: AbortController | null = null;
  /**
   * Counts the listings of the sizes asked for (each load, each refresh):
   * an answer to an older one never overrides a newer one.
   */
  private listing = 0;
  /** Counts the switches: a PUT of the displayed size superseded by a newer switch is skipped. */
  private switches = 0;
  /** The PUTs of the displayed size, one after the other, so that they land in order. */
  private displayedPut: Promise<void> = Promise.resolve();
  /** The size a switch asked for while its PUT has not landed: loads of the body keep it. */
  private switchingTo: { id: string; size: string } | null = null;
  /** The size shown whose missing file is being checked (fileMissing): `id size revision`. */
  private missingChecked: string | null = null;

  /**
   * Loads a world, or clears the pane with null. Earlier loads are aborted.
   * Loading the id shown again reloads it (a world regenerated in place).
   */
  load(id: string | null): void {
    this.systemController?.abort();
    this.system = null;
    this.systemId = null;
    this.systemBody = null;
    this.systemNames = {};
    this.systemEvicted = [];
    this.subject = 'planet';
    this.starMode = 'system';
    this.pinnedPlanets = [];
    this.worldId = id;
    this.worldMeta = null;
    this.worldStats = null;
    this.worldRivers = null;
    this.worldError = null;
    this.loadBody(id);
  }

  /**
   * Loads system `id` and shows its body `body`, or the body it was asked
   * around, else its first generated planet, else its first planet.
   * Loading it again (after a generation or a rename) keeps the body shown,
   * unless `focus` asks for the planet it was drawn around, in the planet view.
   * `body` `'star'` opens the star view (see starMode), on the body it
   * would open without one.
   */
  loadSystem(id: string, asked: string | null = null, focus = false): void {
    this.systemController?.abort();
    const controller = new AbortController();
    this.systemController = controller;
    const again = this.systemId === id;
    const starView = asked === 'star' && !focus;
    const body = asked === 'star' ? null : asked;
    if (!again) {
      this.subject = body || focus ? 'planet' : 'star';
      this.starMode = starView ? 'star' : 'system';
      this.pinnedPlanets = [];
      // Nothing of the system left may stand under the new id: its file,
      // names, body and layers would be read as the new system's until
      // its own file arrives (moon ids requested under the wrong system).
      this.system = null;
      this.systemNames = {};
      this.systemEvicted = [];
      this.systemBody = null;
      this.worldMeta = null;
      this.worldStats = null;
      this.worldRivers = null;
      this.loadBody(null);
    } else if (starView) {
      this.subject = 'star';
      this.starMode = 'star';
    }
    this.systemId = id;
    this.worldId = null;
    (async () => {
      const { system, names, evicted } = await fetchSystem(id, controller.signal);
      if (controller.signal.aborted) return;
      this.system = system;
      this.systemNames = names;
      this.systemEvicted = evicted;
      // `focus` shows the planet the system was drawn around, as a planet.
      if (focus) this.subject = 'planet';
      const wanted = body ?? (focus ? null : again ? this.systemBody : null);
      const known = (b: string | null | undefined) => (b && system.bodies.some((x) => x.id === b) ? b : null);
      const first =
        known(wanted) ??
        known(system.focus) ??
        system.bodies.find((b) => b.kind === 'planet' && b.generated)?.id ??
        system.bodies.find((b) => b.kind === 'planet')?.id ??
        null;
      const shown = this.systemBody;
      const was = shown ? system.bodies.find((b) => b.id === shown) : undefined;
      // Kept loaded when it is the same body and still as generated.
      this.systemBody = null;
      this.showSystemBody(first, again && shown === first && was?.generated === true && this.meta !== null);
    })().catch((e: unknown) => {
      if (!controller.signal.aborted) this.error = t.world.loadFailed({ id, error: String(e) });
    });
  }

  /**
   * Shows body `body` of the system loaded: its maps when it is generated,
   * its preview otherwise. `keep` leaves its layers as they are.
   */
  showSystemBody(body: string | null, keep = false): void {
    const system = this.system;
    if (!system || !this.systemId || body === this.systemBody) return;
    this.systemBody = body;
    const entry = system.bodies.find((b) => b.id === body);
    const planet = entry?.parent ?? entry?.id ?? null;
    const previous = this.worldId;
    this.worldId = planet ? systemBodyId(this.systemId, planet) : null;
    if (keep) {
      // Its resolutions may have changed meanwhile (one generated or deleted).
      this.refreshSizes().catch((e: unknown) => console.warn('Could not list the resolutions', e));
      return;
    }
    const moon = entry !== undefined && planet !== null && entry.id !== planet;
    // A moon's card is its planet's: kept when the planet is the one shown
    // already (its biomes and rivers come with its maps), read otherwise.
    if (!moon || previous !== this.worldId) {
      this.worldMeta = null;
      this.worldStats = null;
      this.worldRivers = null;
    }
    this.worldError = null;
    this.planetController?.abort();
    this.planetController = null;
    if (moon && this.worldId) this.loadPlanetMeta(this.worldId);
    this.loadBody(entry?.generated ? systemBodyId(this.systemId, entry.id) : null);
  }

  /**
   * Reads the metadata, biome statistics and rivers of the planet `id` of a
   * moon shown (its card is the planet's), at the resolution displayed. All
   * peeks: the planet is not the body looked at, so the server does not
   * count it opened. The card shows the planet's statistics, whichever of
   * the two was shown first (the moon's own are not on its card).
   */
  private loadPlanetMeta(id: string): void {
    const controller = new AbortController();
    this.planetController = controller;
    const { signal } = controller;
    (async () => {
      const sizes = await api.sizes(id, signal);
      const shown = displayedSize(sizes);
      if (!shown) throw new Error('no resolution on disk');
      const meta = await api.peekMetadata(id, sizeName(shown.width, shown.height), signal);
      if (signal.aborted || this.worldId !== id) return;
      if (meta.format_version < MIN_FORMAT_VERSION) {
        this.worldError = t.world.outdated({ id });
        return;
      }
      this.worldMeta = meta;
      const name = sizeName(shown.width, shown.height);
      const [stats, rivers] = await Promise.all([
        loadPeekedStats(peeked(worldFile(id, name, `raw_data/${meta.layers.biomes.file}`)), meta, signal),
        meta.rivers ? api.peekRivers(id, name, meta.rivers, signal) : null,
      ]);
      if (signal.aborted || this.worldId !== id) return;
      this.worldStats = stats;
      if (rivers) this.worldRivers = new RiverIndex(rivers, meta.circumference_km);
    })().catch((e: unknown) => {
      if (signal.aborted || this.worldId !== id) return;
      this.worldError = t.world.loadFailed({ id, error: String(e) });
    });
  }

  /**
   * Shows the body loaded at resolution `size` (`WxH`, one on disk) and
   * remembers it as the one displayed, in every browser. Resolves once the
   * server has remembered it (or a newer switch replaced this one); the
   * maps load meanwhile. The PUTs go one after the other, so the last
   * switch is the one remembered.
   */
  async switchSize(size: string): Promise<void> {
    const id = this.id;
    if (!id) throw new Error(`${size} is not generated`);
    const switched = ++this.switches;
    let present = this.sizes ? findSize(this.sizes, size) : null;
    if (!present) {
      // Just generated, and not in the listing yet (a refresh dropped for
      // a newer one, say): list them again.
      const listing = ++this.listing;
      const sizes = await api.sizes(id);
      if (this.id !== id || switched !== this.switches) return;
      if (listing === this.listing) this.sizes = sizes;
      present = findSize(sizes, size);
      if (!present) throw new Error(`${size} is not generated`);
    }
    const { width, height } = present;
    this.switchingTo = { id, size };
    if (size !== this.size) {
      this.loadBody(id, size);
    }
    // After the PUTs sent before it; skipped when a newer switch follows.
    const put = this.displayedPut
      .then(async () => {
        if (switched === this.switches) await api.setDisplayedSize(id, width, height);
      })
      .finally(() => {
        if (switched === this.switches) this.switchingTo = null;
      });
    this.displayedPut = put.catch(() => undefined);
    await put;
  }

  /**
   * Lists the resolutions of the body shown again (after one was generated
   * or deleted, here or in another tab). The body loads again when the
   * resolution shown is gone (at the displayed one) or was regenerated.
   */
  async refreshSizes(): Promise<void> {
    const id = this.id;
    if (!id) return;
    const listing = ++this.listing;
    const sizes = await api.sizes(id);
    // A load or a refresh started meanwhile (a switch, say) knows better.
    if (this.id !== id || listing !== this.listing) return;
    this.sizes = sizes;
    const shown = this.size;
    if (shown === null) return;
    const present = findSize(sizes, shown);
    if (!present) {
      this.loadBody(id);
    } else if (present.revision !== this.revision) {
      this.loadBody(id, shown);
    }
  }

  /**
   * A file of the resolution shown could not be read (a tile answered 404,
   * say): the resolution may have been deleted elsewhere. Lists the sizes
   * again, which loads the displayed one when it is gone. One check at a
   * time per resolution shown, so that many failures at once (tiles) list
   * them once; a check that finds the size still there is over, so a later
   * failure checks again.
   */
  fileMissing(): void {
    const { id, size, revision } = this;
    if (!id || size === null) return;
    const key = `${id} ${size} ${revision}`;
    if (this.missingChecked === key) return;
    this.missingChecked = key;
    this.refreshSizes()
      .catch((e: unknown) => console.warn('Could not list the resolutions', e))
      .finally(() => {
        if (this.missingChecked === key) this.missingChecked = null;
      });
  }

  /**
   * Loads body `id` at resolution `wanted`, else at its displayed one, or
   * clears it with null. `retried`: this load already follows a resolution
   * deleted while it loaded.
   */
  private loadBody(id: string | null, wanted: string | null = null, retried = false): void {
    this.controller?.abort();
    this.epochController?.abort();
    this.present = null;
    this.epochLoading = false;
    this.epochError = null;
    const controller = new AbortController();
    this.controller = controller;
    const { signal } = controller;
    const current = () => !signal.aborted;
    // The same body keeps its listing while it loads at another size.
    if (id !== this.id) this.sizes = null;
    // A switch whose PUT has not landed yet: the listing would still
    // display the size it left.
    if (wanted === null && id !== null && this.switchingTo?.id === id) wanted = this.switchingTo.size;
    const listing = ++this.listing;
    this.id = id;
    this.size = null;
    this.revision = null;
    this.meta = null;
    this.layers = null;
    this.stats = null;
    this.rivers = null;
    this.error = null;
    this.progress = null;
    if (!id) {
      return;
    }
    (async () => {
      const sizes = await api.sizes(id, signal);
      if (!current()) return;
      if (listing === this.listing) this.sizes = sizes;
      const shown = (wanted !== null ? findSize(sizes, wanted) : null) ?? displayedSize(sizes);
      if (!shown) throw new Error('no resolution on disk');
      const size = sizeName(shown.width, shown.height);
      this.size = size;
      this.revision = shown.revision;
      const meta = await api.metadata(id, size, signal);
      if (!current()) return;
      // Made by an older generator: nothing to request, tiles and epochs
      // included.
      if (meta.format_version < MIN_FORMAT_VERSION) {
        this.error = t.world.outdated({ id });
        return;
      }
      this.meta = meta;
      if (id === this.worldId) {
        this.worldMeta = meta;
      }
      const { layers, stats } = await loadLayers(id, size, meta, signal, (progress) => {
        if (current()) this.progress = progress;
      });
      if (!current()) return;
      this.progress = null;
      this.present = layers;
      this.layers = layers;
      this.stats = stats;
      if (id === this.worldId) {
        this.worldStats = stats;
      }
      // An epoch asked for before the layers arrived.
      this.showEpoch();
      if (meta.rivers) {
        const file = await api.rivers(id, size, meta.rivers, signal);
        if (!current()) return;
        this.rivers = new RiverIndex(file, meta.circumference_km);
        if (id === this.worldId) {
          this.worldRivers = this.rivers;
        }
      }
    })().catch(async (e: unknown) => {
      if (!current()) return;
      // The resolution shown deleted meanwhile (from another tab): load the
      // displayed one instead, once.
      const shown = this.size;
      if (shown !== null && !retried) {
        const sizes = await api.sizes(id, signal).catch(() => null);
        if (!current()) return;
        if (sizes && !findSize(sizes, shown)) {
          this.loadBody(id, null, true);
          return;
        }
      }
      this.progress = null;
      this.error = t.world.loadFailed({ id, error: String(e) });
    });
  }

  /**
   * Shows the layers at `epochMa` (0: the present), loaded from the server
   * at the resolution shown. Called before the layers have loaded, it
   * applies once they arrive. A newer call cancels this one.
   */
  setEpoch(epochMa: number): void {
    if (epochMa === this.epochMa && !this.epochError) {
      return;
    }
    this.epochMa = epochMa;
    this.showEpoch();
  }

  private showEpoch(): void {
    this.epochController?.abort();
    this.epochController = null;
    this.epochError = null;
    const { id, size, meta, present } = this;
    const epochMa = this.epochMa;
    if (!id || !size || !meta || !present) {
      this.epochLoading = false;
      return;
    }
    if (epochMa === 0) {
      this.epochLoading = false;
      this.layers = present;
      return;
    }
    const controller = new AbortController();
    this.epochController = controller;
    this.epochLoading = true;
    const urls = {
      elevation: epochFile(id, size, epochMa, 'elevation.png', this.revision),
      biomes: epochFile(id, size, epochMa, 'biomes.png', this.revision),
    };
    loadEpochLayers(urls, meta, present, controller.signal)
      .then((layers) => {
        if (controller.signal.aborted) return;
        this.layers = layers;
      })
      .catch((e: unknown) => {
        if (controller.signal.aborted) return;
        this.epochError = t.epochs.failed({ error: String(e) });
      })
      .finally(() => {
        if (!controller.signal.aborted) this.epochLoading = false;
      });
  }
}

/** Resolution `size` (`WxH`) of a listing, when it is on disk. */
function findSize(sizes: BodySizes, size: string): PresentSize | null {
  return sizes.present.find((p) => sizeName(p.width, p.height) === size) ?? null;
}

/** The resolution a listing displays, else its original, else its first on disk. */
function displayedSize(sizes: BodySizes): PresentSize | null {
  return (
    findSize(sizes, sizeName(...sizes.displayed)) ?? findSize(sizes, sizeName(...sizes.original)) ?? sizes.present[0] ?? null
  );
}

/**
 * A system and the names given to its bodies (`GET /api/systems/{id}`),
 * or its file alone (`/systems/{id}/system.json`) when the route is not
 * there.
 */
async function fetchSystem(
  id: string,
  signal: AbortSignal,
): Promise<{ system: System; names: Record<string, string>; evicted: string[] }> {
  const response = await fetch(`/api/systems/${encodeURIComponent(id)}`, { signal, cache: 'no-cache' });
  if (response.ok) {
    const body = (await response.json()) as { system: System; names?: Record<string, string>; evicted?: string[] };
    return { system: body.system, names: body.names ?? {}, evicted: body.evicted ?? [] };
  }
  const file = await fetch(`/systems/${encodeURIComponent(id)}/system.json`, { signal, cache: 'no-cache' });
  if (!file.ok) throw new Error(`${file.status} ${file.statusText}`);
  return { system: (await file.json()) as System, names: {}, evicted: [] };
}
