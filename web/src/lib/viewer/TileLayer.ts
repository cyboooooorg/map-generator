// Deep zoom: detail patches drawn over the base surface where the camera
// needs more resolution than the base map has.
//
// Each frame the quadtree is walked from the root: a tile splits while one
// of its texels projects to more than SPLIT_PX screen pixels, and tiles
// outside the view (frustum, or behind the horizon on the globe) are
// skipped. Leaves at or below the first detail level get a patch: a small
// mesh drawn by the surface shader over the tile's rectangle, showing the
// tile, or a crop of its closest loaded ancestor while it loads. Patches
// mark the stencil buffer and the base surface is drawn only around them.
import {
  AlwaysStencilFunc,
  Frustum,
  LinearFilter,
  LinearMipmapLinearFilter,
  Matrix4,
  Mesh,
  type PerspectiveCamera,
  PlaneGeometry,
  ReplaceStencilOp,
  type Scene,
  type ShaderMaterial,
  SRGBColorSpace,
  Sphere,
  Texture,
  Vector3,
  Vector4,
} from 'three';
import { bodyApi } from '../api/client';
import {
  ancestor,
  ancestorIndex,
  cropOf,
  firstDetailLevel,
  MAX_LEVEL,
  TILE_SIZE,
  type TileId,
  tileIndex,
  tileKey,
  tileOfIndex,
  tileRect,
} from './tiles';
import { releaseOnUpload } from './textures';
import { morphPoint } from './viewTransfer';

/** Split a tile while one of its texels covers more screen pixels than this. */
const SPLIT_PX = 1.5;
/** Concurrent tile downloads. */
const MAX_IN_FLIGHT = 6;
/** Delay before downloading a failed tile again, doubled at each failure up to RETRY_MAX_MS. */
const RETRY_MS = 2000;
const RETRY_MAX_MS = 60_000;
/**
 * Downloads younger than this are kept when the view no longer needs their
 * tile: fast zooming would otherwise abort and restart them every frame.
 */
const KEEP_MS = 400;
/**
 * A tile with nothing loaded to show also requests its ancestor this many
 * levels up: one coarse tile covers 4^n leaves and fills the gap quickly.
 */
const FALLBACK_LEVELS = 2;
/** Hidden patch meshes kept for reuse. */
const MAX_SPARE = 128;
/** Tile textures kept on the GPU, by default (see setTextureBudget). */
export const MAX_TEXTURES = 300;

/** Stencil value marking the pixels a detail patch covers. */
export const DETAIL_STENCIL = 1;
/**
 * Version of the server's tile renderer (mapgen::detail::RENDER_VERSION),
 * part of the tile URL so that tiles cached for good are dropped when the
 * renderer changes. A server test checks both values agree.
 */
export const TILE_RENDER_VERSION = 5;

/** Creates the material of a patch; uniforms come from the base surface. */
export type PatchMaterialFactory = () => ShaderMaterial;

interface CachedTile {
  texture: Texture;
  lastUsed: number;
}

interface Download {
  controller: AbortController;
  started: number;
}

interface Failure {
  /** When to try again (Infinity: never, the server refused the tile). */
  retryAt: number;
  attempts: number;
}

/**
 * Delay before retrying a tile after `attempts` failures, or Infinity when
 * retrying cannot help (a client error such as 404: the tile does not exist).
 */
export function retryDelay(attempts: number, status: number | null): number {
  if (status !== null && status >= 400 && status < 500 && status !== 408 && status !== 429) {
    return Infinity;
  }
  return Math.min(RETRY_MAX_MS, RETRY_MS * 2 ** Math.max(0, attempts - 1));
}

/** Frees a tile texture, and its decoded image if it was never uploaded. */
function disposeTile(texture: Texture): void {
  texture.dispose();
  (texture.image as ImageBitmap).close();
}

/** Position of a texture coordinate on the surface morphed round `centreLon` (see shaders.ts). */
function surfacePoint(u: number, v: number, morph: number, centreLon: number, out: Vector3): Vector3 {
  return out.set(...morphPoint({ lon: u * 2 * Math.PI, lat: (v - 0.5) * Math.PI }, morph, centreLon));
}

export class TileLayer {
  private readonly geometry = new PlaneGeometry(1, 1, 16, 16);
  /** Patches shown, by tileIndex. */
  private readonly patches = new Map<number, Mesh<PlaneGeometry, ShaderMaterial>>();
  /** Hidden patches, reused by the next ones shown. */
  private readonly spare: Mesh<PlaneGeometry, ShaderMaterial>[] = [];
  /** Loaded tiles, downloads and failures, by tileIndex. */
  private readonly cache = new Map<number, CachedTile>();
  private readonly loading = new Map<number, Download>();
  /** Tiles whose download failed, with the time to retry them. */
  private readonly failed = new Map<number, Failure>();
  /** Timers that wake the view up when failed tiles can be retried. */
  private readonly retryTimers = new Set<ReturnType<typeof setTimeout>>();
  private world: { id: string; revision: number; epochMa: number; firstLevel: number } | null = null;
  private enabled = true;
  private maxTextures = MAX_TEXTURES;
  private frame = 0;
  private readonly frustum = new Frustum();
  private readonly matrix = new Matrix4();
  private readonly sphere = new Sphere();
  private readonly corners = Array.from({ length: 5 }, () => new Vector3());
  // Scratch state of an update, reused from frame to frame so that walking
  // the quadtree allocates nothing but the leaves it finds.
  private readonly walkCamera = new Vector3();
  private walkMorph = 1;
  private walkCentreLon = 0;
  private pxPerUnit = 1;
  private readonly leaves: TileId[] = [];
  /** tileIndex of each leaf, and of the tile it shows (itself or an ancestor; -1: none). */
  private readonly leafIndices: number[] = [];
  private readonly sources: number[] = [];
  private readonly wanted = new Map<number, TileId>();
  private readonly useful = new Set<number>();
  private readonly shown = new Set<number>();
  private readonly missing: { tile: TileId; distance: number }[] = [];
  private readonly centre = new Vector3();

  /**
   * @param onChange called when the view should be drawn again: a tile
   *   arrived, or a failed one can be retried.
   */
  constructor(
    private readonly scene: Scene,
    private readonly makeMaterial: PatchMaterialFactory,
    private readonly onChange: () => void,
  ) {}

  /**
   * Shows the detail of a world (`revision` keys the browser cache) at an
   * epoch in Ma (0, the default, for the present; see lib/epochs.ts), or none.
   */
  setWorld(world: { id: string; revision: number; baseWidth: number; epochMa?: number } | null): void {
    this.clear();
    this.world = world && {
      id: world.id,
      revision: world.revision,
      epochMa: world.epochMa ?? 0,
      firstLevel: firstDetailLevel(world.baseWidth),
    };
  }

  /**
   * Sets how many tile textures stay on the GPU (the least recently shown
   * go first, from the next update on). Tiles the current view shows are
   * kept even beyond it.
   */
  setTextureBudget(count: number): void {
    this.maxTextures = Math.max(1, Math.floor(count));
  }

  /** Detail only makes sense on the biome layer. */
  setEnabled(enabled: boolean): void {
    this.enabled = enabled;
    if (!enabled) {
      this.shown.clear();
      this.hidePatches(this.shown);
    }
  }

  /** Picks, loads and places the patches for the current view. */
  update(camera: PerspectiveCamera, morph: number, viewportHeight: number, morphLon = 0): void {
    this.frame += 1;
    if (!this.world || !this.enabled) {
      return;
    }
    this.matrix.multiplyMatrices(camera.projectionMatrix, camera.matrixWorldInverse);
    this.frustum.setFromProjectionMatrix(this.matrix);
    // Screen pixels per world unit at distance 1.
    this.pxPerUnit = viewportHeight / (2 * Math.tan((camera.fov * Math.PI) / 360) / camera.zoom);
    this.walkCamera.copy(camera.position);
    this.walkMorph = morph;
    this.walkCentreLon = morphLon;

    const { leaves, leafIndices, sources, shown } = this;
    leaves.length = 0;
    this.walk(0, 0, 0);
    this.walk(0, 1, 0);
    leafIndices.length = 0;
    sources.length = 0;
    for (const leaf of leaves) {
      leafIndices.push(tileIndex(leaf.z, leaf.x, leaf.y));
      sources.push(this.bestLoaded(leaf));
    }

    this.request(camera.position);
    shown.clear();
    for (let i = 0; i < leaves.length; i += 1) {
      const source = sources[i]!;
      if (source >= 0) {
        this.place(leaves[i]!, leafIndices[i]!, source);
        shown.add(leafIndices[i]!);
      }
    }
    this.hidePatches(shown);
    this.evict();
  }

  /**
   * Visits tile (z, x, y): skips it outside the view, splits it while its
   * texels look too large, and adds it to the leaves otherwise.
   */
  private walk(z: number, x: number, y: number): void {
    const cols = 2 ** (z + 1);
    const rows = 2 ** z;
    const du = 1 / cols;
    const dv = 1 / rows;
    const u0 = x * du;
    const v0 = 1 - (y + 1) * dv;
    const morph = this.walkMorph;
    const centreLon = this.walkCentreLon;
    const [c0, c1, c2, c3, c4] = this.corners as [Vector3, Vector3, Vector3, Vector3, Vector3];
    surfacePoint(u0, v0, morph, centreLon, c0);
    surfacePoint(u0 + du, v0, morph, centreLon, c1);
    surfacePoint(u0, v0 + dv, morph, centreLon, c2);
    surfacePoint(u0 + du, v0 + dv, morph, centreLon, c3);
    surfacePoint(u0 + du / 2, v0 + dv / 2, morph, centreLon, c4);
    this.sphere.setFromPoints(this.corners);
    // A quarter of the globe can bulge beyond its corners: pad coarse tiles.
    this.sphere.radius *= z < 2 ? 1.6 : 1.05;
    if (!this.frustum.intersectsSphere(this.sphere)) {
      return;
    }
    const cam = this.walkCamera;
    if (morph > 0.99 && z >= 2) {
      // Behind the horizon: no point of the tile faces the camera.
      const limit = 1 - 1e-3;
      if (c0.dot(cam) < limit && c1.dot(cam) < limit && c2.dot(cam) < limit && c3.dot(cam) < limit && c4.dot(cam) < limit) {
        return;
      }
    }
    const distance = Math.max(this.sphere.center.distanceTo(cam) - this.sphere.radius, 1e-6);
    // A texel spans π / (rows × 256) in latitude, on the globe and the map alike.
    const texel = Math.PI / (rows * TILE_SIZE);
    const projected = (texel / distance) * this.pxPerUnit;
    if (projected > SPLIT_PX && z < MAX_LEVEL) {
      this.walk(z + 1, 2 * x, 2 * y);
      this.walk(z + 1, 2 * x + 1, 2 * y);
      this.walk(z + 1, 2 * x, 2 * y + 1);
      this.walk(z + 1, 2 * x + 1, 2 * y + 1);
    } else if (z >= this.world!.firstLevel) {
      this.leaves.push({ z, x, y });
    }
  }

  dispose(): void {
    this.clear();
    for (const patch of this.spare) {
      patch.material.dispose();
    }
    this.spare.length = 0;
    this.geometry.dispose();
  }

  // ── Loading ────────────────────────────────────────────────────────────────

  /** Starts the downloads the leaves of this frame need, and aborts the ones no longer useful. */
  private request(camera: Vector3): void {
    const first = this.world!.firstLevel;
    const now = performance.now();
    const { leaves, leafIndices, sources, wanted, useful, missing } = this;
    // Leaves with nothing to show yet also want a coarser ancestor.
    wanted.clear();
    for (let i = 0; i < leaves.length; i += 1) {
      const leaf = leaves[i]!;
      wanted.set(leafIndices[i]!, leaf);
      if (leaf.z - FALLBACK_LEVELS >= first && sources[i]! < 0) {
        const level = leaf.z - FALLBACK_LEVELS;
        const index = ancestorIndex(leaf.z, leaf.x, leaf.y, level);
        if (!wanted.has(index)) {
          wanted.set(index, ancestor(leaf, level));
        }
      }
    }
    // Abort downloads of tiles no longer needed, unless just started.
    // Ancestors of the wanted tiles stay useful while they load.
    if (this.loading.size > 0) {
      useful.clear();
      for (const tile of wanted.values()) {
        for (let level = tile.z; level >= first; level -= 1) {
          useful.add(ancestorIndex(tile.z, tile.x, tile.y, level));
        }
      }
      for (const [index, download] of this.loading) {
        if (!useful.has(index) && now - download.started > KEEP_MS) {
          download.controller.abort();
          this.loading.delete(index);
        }
      }
    }
    missing.length = 0;
    for (const [index, tile] of wanted) {
      const failure = this.failed.get(index);
      if (!this.cache.has(index) && !this.loading.has(index) && (failure === undefined || failure.retryAt <= now)) {
        const [u, v, du, dv] = tileRect(tile);
        missing.push({ tile, distance: surfacePoint(u + du / 2, v + dv / 2, 1, 0, this.centre).distanceTo(camera) });
      }
    }
    // Coarse tiles first, then the closest to the camera.
    missing.sort((a, b) => a.tile.z - b.tile.z || a.distance - b.distance);
    for (const { tile } of missing) {
      if (this.loading.size >= MAX_IN_FLIGHT) {
        break;
      }
      void this.load(tile);
    }
    missing.length = 0;
  }

  private async load(tile: TileId): Promise<void> {
    const world = this.world;
    if (!world) return;
    const key = tileKey(tile);
    const index = tileIndex(tile.z, tile.x, tile.y);
    const controller = new AbortController();
    this.loading.set(index, { controller, started: performance.now() });
    let status: number | null = null;
    try {
      const epoch = world.epochMa === 0 ? '' : `&epoch=${world.epochMa}`;
      const url = `${bodyApi(world.id)}/tiles/${key}?rev=${world.revision}&render=${TILE_RENDER_VERSION}${epoch}`;
      const response = await fetch(url, { signal: controller.signal });
      if (!response.ok) {
        status = response.status;
        throw new Error(`${response.status}`);
      }
      const bitmap = await createImageBitmap(await response.blob());
      if (this.world !== world || controller.signal.aborted) {
        bitmap.close();
        return;
      }
      const texture = new Texture(bitmap);
      texture.colorSpace = SRGBColorSpace;
      texture.flipY = false;
      texture.magFilter = LinearFilter;
      texture.minFilter = LinearMipmapLinearFilter;
      texture.needsUpdate = true;
      // The bitmap is closed once uploaded: a cached tile keeps only its
      // GPU copy, and patches showing it share that copy.
      releaseOnUpload(texture);
      this.cache.set(index, { texture, lastUsed: this.frame });
      this.failed.delete(index);
      this.onChange();
    } catch {
      if (!controller.signal.aborted && this.world === world) {
        // Retry later, less and less often: a tile left out shows a blurry
        // ancestor for good. A tile the server refuses is not retried.
        const attempts = (this.failed.get(index)?.attempts ?? 0) + 1;
        const delay = retryDelay(attempts, status);
        this.failed.set(index, { retryAt: performance.now() + delay, attempts });
        if (Number.isFinite(delay)) {
          const timer = setTimeout(() => {
            this.retryTimers.delete(timer);
            this.onChange();
          }, delay);
          this.retryTimers.add(timer);
        }
      }
    } finally {
      if (this.loading.get(index)?.controller === controller) {
        this.loading.delete(index);
      }
    }
  }

  /** tileIndex of the tile itself or of its closest loaded ancestor, or -1. */
  private bestLoaded(tile: TileId): number {
    const first = this.world?.firstLevel ?? MAX_LEVEL;
    for (let level = tile.z; level >= first; level -= 1) {
      const index = ancestorIndex(tile.z, tile.x, tile.y, level);
      if (this.cache.has(index)) {
        return index;
      }
    }
    return -1;
  }

  // ── Patches ────────────────────────────────────────────────────────────────

  /** Shows leaf `tile` (tileIndex `index`) with the loaded tile `source`. */
  private place(tile: TileId, index: number, source: number): void {
    let patch = this.patches.get(index);
    if (!patch) {
      patch = this.spare.pop() ?? this.newPatch();
      const [u0, v0, du, dv] = tileRect(tile);
      (patch.material.uniforms['uTileRect']!.value as Vector4).set(u0, v0, du, dv);
      this.patches.set(index, patch);
      this.scene.add(patch);
    }
    const cached = this.cache.get(source)!;
    cached.lastUsed = this.frame;
    patch.material.uniforms['uTile']!.value = cached.texture;
    const texRect = patch.material.uniforms['uTexRect']!.value as Vector4;
    if (source === index) {
      texRect.set(0, 0, 1, 1);
    } else {
      const [cu, cv, cdu, cdv] = cropOf(tile, tileOfIndex(source));
      texRect.set(cu, cv, cdu, cdv);
    }
    patch.visible = true;
  }

  private newPatch(): Mesh<PlaneGeometry, ShaderMaterial> {
    const patch = new Mesh(this.geometry, this.makeMaterial());
    // Patches are drawn before the base surface and mark the stencil, so
    // the base never covers them, even where the coarse geometry rises
    // above them (relief). Leaves never overlap each other.
    patch.renderOrder = -1;
    patch.material.stencilWrite = true;
    patch.material.stencilRef = DETAIL_STENCIL;
    patch.material.stencilFunc = AlwaysStencilFunc;
    patch.material.stencilZPass = ReplaceStencilOp;
    patch.material.uniforms['uTileOn']!.value = 1;
    patch.frustumCulled = false;
    return patch;
  }

  /**
   * Takes the patches not in `keep` out of the scene. Their meshes are kept
   * for reuse (up to MAX_SPARE): panning hides and shows patches every
   * frame, and creating and disposing a material per patch costs a shader
   * program lookup and a uniform setup each time.
   */
  private hidePatches(keep: ReadonlySet<number>): void {
    for (const [index, patch] of this.patches) {
      if (!keep.has(index)) {
        this.scene.remove(patch);
        patch.material.uniforms['uTile']!.value = null;
        this.patches.delete(index);
        if (this.spare.length < MAX_SPARE) {
          this.spare.push(patch);
        } else {
          patch.material.dispose();
        }
      }
    }
  }

  /** Frees the least recently used textures beyond the budget. */
  private evict(): void {
    if (this.cache.size <= this.maxTextures) {
      return;
    }
    const byAge = [...this.cache.entries()].sort((a, b) => a[1].lastUsed - b[1].lastUsed);
    for (const [index, tile] of byAge.slice(0, this.cache.size - this.maxTextures)) {
      if (tile.lastUsed === this.frame) {
        break;
      }
      disposeTile(tile.texture);
      this.cache.delete(index);
    }
  }

  private clear(): void {
    for (const download of this.loading.values()) {
      download.controller.abort();
    }
    this.loading.clear();
    this.shown.clear();
    this.hidePatches(this.shown);
    for (const tile of this.cache.values()) {
      disposeTile(tile.texture);
    }
    this.cache.clear();
    this.failed.clear();
    for (const timer of this.retryTimers) {
      clearTimeout(timer);
    }
    this.retryTimers.clear();
  }
}
