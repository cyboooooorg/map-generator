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
import {
  ancestor,
  children,
  cropOf,
  firstDetailLevel,
  grid,
  MAX_LEVEL,
  TILE_SIZE,
  type TileId,
  tileKey,
  tileRect,
} from './tiles';
import { releaseOnUpload } from './textures';

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
export const TILE_RENDER_VERSION = 2;

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

/** Position of a texture coordinate on the morphed surface (see shaders.ts). */
function surfacePoint(u: number, v: number, morph: number, out: Vector3): Vector3 {
  const lon = u * 2 * Math.PI;
  const lat = (v - 0.5) * Math.PI;
  const sx = Math.cos(lat) * Math.sin(lon);
  const sy = Math.sin(lat);
  const sz = Math.cos(lat) * Math.cos(lon);
  const px = (u - 0.5) * 2 * Math.PI;
  const py = (v - 0.5) * Math.PI;
  return out.set(px + (sx - px) * morph, py + (sy - py) * morph, sz * morph);
}

export class TileLayer {
  private readonly geometry = new PlaneGeometry(1, 1, 16, 16);
  private readonly patches = new Map<string, Mesh<PlaneGeometry, ShaderMaterial>>();
  /** Hidden patches, reused by the next ones shown. */
  private readonly spare: Mesh<PlaneGeometry, ShaderMaterial>[] = [];
  private readonly cache = new Map<string, CachedTile>();
  private readonly loading = new Map<string, Download>();
  /** Tiles whose download failed, with the time to retry them. */
  private readonly failed = new Map<string, Failure>();
  /** Timers that wake the view up when failed tiles can be retried. */
  private readonly retryTimers = new Set<ReturnType<typeof setTimeout>>();
  private world: { id: string; revision: number; firstLevel: number } | null = null;
  private enabled = true;
  private maxTextures = MAX_TEXTURES;
  private frame = 0;
  private readonly frustum = new Frustum();
  private readonly matrix = new Matrix4();
  private readonly sphere = new Sphere();
  private readonly corners = Array.from({ length: 5 }, () => new Vector3());

  /**
   * @param onChange called when the view should be drawn again: a tile
   *   arrived, or a failed one can be retried.
   */
  constructor(
    private readonly scene: Scene,
    private readonly makeMaterial: PatchMaterialFactory,
    private readonly onChange: () => void,
  ) {}

  /** Shows the detail of a world (`revision` keys the browser cache), or none. */
  setWorld(world: { id: string; revision: number; baseWidth: number } | null): void {
    this.clear();
    this.world = world && { id: world.id, revision: world.revision, firstLevel: firstDetailLevel(world.baseWidth) };
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
      this.hidePatches(new Set());
    }
  }

  /** Picks, loads and places the patches for the current view. */
  update(camera: PerspectiveCamera, morph: number, viewportHeight: number): void {
    this.frame += 1;
    if (!this.world || !this.enabled) {
      return;
    }
    this.matrix.multiplyMatrices(camera.projectionMatrix, camera.matrixWorldInverse);
    this.frustum.setFromProjectionMatrix(this.matrix);
    // Screen pixels per world unit at distance 1.
    const pxPerUnit = viewportHeight / (2 * Math.tan((camera.fov * Math.PI) / 360) / camera.zoom);

    const leaves: TileId[] = [];
    const walk = (tile: TileId): void => {
      const [u0, v0, du, dv] = tileRect(tile);
      const points = [
        surfacePoint(u0, v0, morph, this.corners[0]!),
        surfacePoint(u0 + du, v0, morph, this.corners[1]!),
        surfacePoint(u0, v0 + dv, morph, this.corners[2]!),
        surfacePoint(u0 + du, v0 + dv, morph, this.corners[3]!),
        surfacePoint(u0 + du / 2, v0 + dv / 2, morph, this.corners[4]!),
      ];
      this.sphere.setFromPoints(points);
      // A quarter of the globe can bulge beyond its corners: pad coarse tiles.
      this.sphere.radius *= tile.z < 2 ? 1.6 : 1.05;
      if (!this.frustum.intersectsSphere(this.sphere)) {
        return;
      }
      if (morph > 0.99 && tile.z >= 2) {
        // Behind the horizon: no point of the tile faces the camera.
        const cam = camera.position;
        if (points.every((p) => p.dot(cam) < 1 - 1e-3)) {
          return;
        }
      }
      const distance = Math.max(this.sphere.center.distanceTo(camera.position) - this.sphere.radius, 1e-6);
      // A texel spans π / (rows × 256) in latitude, on the globe and the map alike.
      const texel = Math.PI / (grid(tile.z).rows * TILE_SIZE);
      const projected = (texel / distance) * pxPerUnit;
      if (projected > SPLIT_PX && tile.z < MAX_LEVEL) {
        for (const child of children(tile)) {
          walk(child);
        }
      } else if (tile.z >= this.world!.firstLevel) {
        leaves.push(tile);
      }
    };
    walk({ z: 0, x: 0, y: 0 });
    walk({ z: 0, x: 1, y: 0 });

    this.request(leaves, camera.position);
    const shown = new Set<string>();
    for (const leaf of leaves) {
      const source = this.bestLoaded(leaf);
      if (source) {
        this.place(leaf, source);
        shown.add(tileKey(leaf));
      }
    }
    this.hidePatches(shown);
    this.evict();
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

  private request(leaves: TileId[], camera: Vector3): void {
    const first = this.world!.firstLevel;
    const now = performance.now();
    // Leaves with nothing to show yet also want a coarser ancestor.
    const wantedTiles = new Map<string, TileId>();
    for (const leaf of leaves) {
      wantedTiles.set(tileKey(leaf), leaf);
      if (leaf.z - FALLBACK_LEVELS >= first && !this.bestLoaded(leaf)) {
        const fallback = ancestor(leaf, leaf.z - FALLBACK_LEVELS);
        wantedTiles.set(tileKey(fallback), fallback);
      }
    }
    // Ancestors of the wanted tiles stay useful while they load.
    const useful = new Set<string>();
    for (const tile of wantedTiles.values()) {
      for (let level = tile.z; level >= first; level -= 1) {
        useful.add(tileKey(ancestor(tile, level)));
      }
    }
    // Abort downloads of tiles no longer needed, unless just started.
    for (const [key, download] of this.loading) {
      if (!useful.has(key) && now - download.started > KEEP_MS) {
        download.controller.abort();
        this.loading.delete(key);
      }
    }
    const missing = [...wantedTiles.entries()]
      .filter(([key]) => {
        const failure = this.failed.get(key);
        return !this.cache.has(key) && !this.loading.has(key) && (failure === undefined || failure.retryAt <= now);
      })
      .map(([, tile]) => tile);
    // Coarse tiles first, then the closest to the camera.
    const centre = new Vector3();
    missing.sort((a, b) => {
      if (a.z !== b.z) return a.z - b.z;
      const [au, av, adu, adv] = tileRect(a);
      const [bu, bv, bdu, bdv] = tileRect(b);
      const da = surfacePoint(au + adu / 2, av + adv / 2, 1, centre).distanceTo(camera);
      const db = surfacePoint(bu + bdu / 2, bv + bdv / 2, 1, centre).distanceTo(camera);
      return da - db;
    });
    for (const tile of missing) {
      if (this.loading.size >= MAX_IN_FLIGHT) {
        break;
      }
      void this.load(tile);
    }
  }

  private async load(tile: TileId): Promise<void> {
    const world = this.world;
    if (!world) return;
    const key = tileKey(tile);
    const controller = new AbortController();
    this.loading.set(key, { controller, started: performance.now() });
    let status: number | null = null;
    try {
      const url = `/api/worlds/${encodeURIComponent(world.id)}/tiles/${key}?rev=${world.revision}&render=${TILE_RENDER_VERSION}`;
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
      this.cache.set(key, { texture, lastUsed: this.frame });
      this.failed.delete(key);
      this.onChange();
    } catch {
      if (!controller.signal.aborted && this.world === world) {
        // Retry later, less and less often: a tile left out shows a blurry
        // ancestor for good. A tile the server refuses is not retried.
        const attempts = (this.failed.get(key)?.attempts ?? 0) + 1;
        const delay = retryDelay(attempts, status);
        this.failed.set(key, { retryAt: performance.now() + delay, attempts });
        if (Number.isFinite(delay)) {
          const timer = setTimeout(() => {
            this.retryTimers.delete(timer);
            this.onChange();
          }, delay);
          this.retryTimers.add(timer);
        }
      }
    } finally {
      if (this.loading.get(key)?.controller === controller) {
        this.loading.delete(key);
      }
    }
  }

  /** The tile itself, or its closest loaded ancestor. */
  private bestLoaded(tile: TileId): TileId | null {
    const first = this.world?.firstLevel ?? MAX_LEVEL;
    for (let level = tile.z; level >= first; level -= 1) {
      const candidate = ancestor(tile, level);
      if (this.cache.has(tileKey(candidate))) {
        return candidate;
      }
    }
    return null;
  }

  // ── Patches ────────────────────────────────────────────────────────────────

  private place(tile: TileId, source: TileId): void {
    const key = tileKey(tile);
    let patch = this.patches.get(key);
    if (!patch) {
      patch = this.spare.pop() ?? this.newPatch();
      const [u0, v0, du, dv] = tileRect(tile);
      (patch.material.uniforms['uTileRect']!.value as Vector4).set(u0, v0, du, dv);
      this.patches.set(key, patch);
      this.scene.add(patch);
    }
    const cached = this.cache.get(tileKey(source))!;
    cached.lastUsed = this.frame;
    patch.material.uniforms['uTile']!.value = cached.texture;
    const crop = source.z === tile.z ? [0, 0, 1, 1] : cropOf(tile, source);
    (patch.material.uniforms['uTexRect']!.value as Vector4).set(crop[0]!, crop[1]!, crop[2]!, crop[3]!);
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
  private hidePatches(keep: Set<string>): void {
    for (const [key, patch] of this.patches) {
      if (!keep.has(key)) {
        this.scene.remove(patch);
        patch.material.uniforms['uTile']!.value = null;
        this.patches.delete(key);
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
    for (const [key, tile] of byAge.slice(0, this.cache.size - this.maxTextures)) {
      if (tile.lastUsed === this.frame) {
        break;
      }
      disposeTile(tile.texture);
      this.cache.delete(key);
    }
  }

  private clear(): void {
    for (const download of this.loading.values()) {
      download.controller.abort();
    }
    this.loading.clear();
    this.hidePatches(new Set());
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
