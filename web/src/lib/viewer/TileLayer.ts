// Deep zoom: detail patches drawn over the base surface where the camera
// needs more resolution than the base map has.
//
// Each frame the quadtree is walked from the root: a tile splits while one
// of its texels projects to more than SPLIT_PX screen pixels, and tiles
// outside the view (frustum, or behind the horizon on the globe) are
// skipped. Leaves at or below the first detail level get a patch: a small
// mesh drawn by the surface shader over the tile's rectangle, showing the
// tile, or a crop of its closest loaded ancestor while it loads. Where no
// tile is loaded the base surface shows through.
import {
  Frustum,
  LinearFilter,
  LinearMipmapLinearFilter,
  Matrix4,
  Mesh,
  type PerspectiveCamera,
  PlaneGeometry,
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

/** Split a tile while one of its texels covers more screen pixels than this. */
const SPLIT_PX = 1.5;
/** Concurrent tile downloads. */
const MAX_IN_FLIGHT = 6;
/** Tile textures kept on the GPU. */
const MAX_TEXTURES = 300;

/** Creates the material of a patch; uniforms come from the base surface. */
export type PatchMaterialFactory = () => ShaderMaterial;

interface CachedTile {
  texture: Texture;
  lastUsed: number;
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
  private readonly cache = new Map<string, CachedTile>();
  private readonly loading = new Map<string, AbortController>();
  private readonly failed = new Set<string>();
  private world: { id: string; revision: number; firstLevel: number } | null = null;
  private enabled = true;
  private frame = 0;
  private readonly frustum = new Frustum();
  private readonly matrix = new Matrix4();
  private readonly sphere = new Sphere();
  private readonly corners = Array.from({ length: 5 }, () => new Vector3());

  constructor(
    private readonly scene: Scene,
    private readonly makeMaterial: PatchMaterialFactory,
  ) {}

  /** Shows the detail of a world (`revision` keys the browser cache), or none. */
  setWorld(world: { id: string; revision: number; baseWidth: number } | null): void {
    this.clear();
    this.world = world && { id: world.id, revision: world.revision, firstLevel: firstDetailLevel(world.baseWidth) };
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
    this.geometry.dispose();
  }

  // ── Loading ────────────────────────────────────────────────────────────────

  private request(leaves: TileId[], camera: Vector3): void {
    const wanted = new Set(leaves.map(tileKey));
    // Abort downloads of tiles no longer needed.
    for (const [key, controller] of this.loading) {
      if (!wanted.has(key)) {
        controller.abort();
        this.loading.delete(key);
      }
    }
    const missing = leaves.filter((t) => {
      const key = tileKey(t);
      return !this.cache.has(key) && !this.loading.has(key) && !this.failed.has(key);
    });
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
    this.loading.set(key, controller);
    try {
      const url = `/api/worlds/${encodeURIComponent(world.id)}/tiles/${key}?rev=${world.revision}`;
      const response = await fetch(url, { signal: controller.signal });
      if (!response.ok) {
        throw new Error(`${response.status}`);
      }
      const bitmap = await createImageBitmap(await response.blob());
      if (this.world !== world) {
        bitmap.close();
        return;
      }
      const texture = new Texture(bitmap);
      texture.colorSpace = SRGBColorSpace;
      texture.flipY = false;
      texture.magFilter = LinearFilter;
      texture.minFilter = LinearMipmapLinearFilter;
      texture.needsUpdate = true;
      this.cache.set(key, { texture, lastUsed: this.frame });
    } catch (error) {
      if (!controller.signal.aborted) {
        this.failed.add(key);
      }
    } finally {
      if (this.loading.get(key) === controller) {
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
      patch = new Mesh(this.geometry, this.makeMaterial());
      // Deeper patches win over coarser ones and over the base surface.
      patch.material.polygonOffset = true;
      patch.material.polygonOffsetFactor = -1;
      patch.material.polygonOffsetUnits = -4 * (tile.z + 1);
      patch.renderOrder = tile.z + 1;
      patch.frustumCulled = false;
      const [u0, v0, du, dv] = tileRect(tile);
      (patch.material.uniforms['uTileRect']!.value as Vector4).set(u0, v0, du, dv);
      patch.material.uniforms['uTileOn']!.value = 1;
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

  private hidePatches(keep: Set<string>): void {
    for (const [key, patch] of this.patches) {
      if (!keep.has(key)) {
        this.scene.remove(patch);
        patch.material.dispose();
        this.patches.delete(key);
      }
    }
  }

  /** Frees the least recently used textures beyond the budget. */
  private evict(): void {
    if (this.cache.size <= MAX_TEXTURES) {
      return;
    }
    const byAge = [...this.cache.entries()].sort((a, b) => a[1].lastUsed - b[1].lastUsed);
    for (const [key, tile] of byAge.slice(0, this.cache.size - MAX_TEXTURES)) {
      if (tile.lastUsed === this.frame) {
        break;
      }
      tile.texture.dispose();
      this.cache.delete(key);
    }
  }

  private clear(): void {
    for (const controller of this.loading.values()) {
      controller.abort();
    }
    this.loading.clear();
    this.hidePatches(new Set());
    for (const tile of this.cache.values()) {
      tile.texture.dispose();
    }
    this.cache.clear();
    this.failed.clear();
  }
}
