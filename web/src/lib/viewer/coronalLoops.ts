// Coronal loops standing over the star's active regions: the arches of
// hot plasma that follow the field lines joining a bipole's two spots
// (the leader and the follower), as the Sun's X-ray and EUV images show
// them. Each region gets a few loops, built once when the region appears
// and dropped when it leaves; a frame only turns each region's group to
// its longitude, sets its opacity and advances the strands' shimmer.
import { Group, Mesh, type BufferGeometry, type ShaderMaterial } from 'three';
import { hash32, type ActiveRegion, type RegionNow } from '../starActivity';
import { mergeGeometries } from 'three/examples/jsm/utils/BufferGeometryUtils.js';
import { prominenceGeometry } from './prominenceGeometry';
import type { QualityProfile } from './quality';
import { STAR_RADIUS, type Prominence } from './starSystem';

/** Loops standing over each region in the high and the light profiles. */
const LOOPS_HIGH = 3;
const LOOPS_LIGHT = 1;

/** Strands in each loop: 8 of the full bundle's 12, so 24 loops stay near the cost of the 7 prominences they replace. */
const STRANDS_PER_LOOP = 8;

/** Opacity of a loop over a region at its peak. */
const PEAK_OPACITY = 0.9;

/** A number in (0, 1) from hash keys: never exactly 0 or 1, so bounds stay strict. */
function unit(...keys: number[]): number {
  return (hash32(...keys) + 0.5) / 4_294_967_296;
}

type Vec3 = [number, number, number];

const normalise = (v: Vec3): Vec3 => {
  const n = Math.hypot(v[0], v[1], v[2]);
  return [v[0] / n, v[1] / n, v[2] / n];
};

/**
 * A region's loops in its own frame: the region centred at longitude 0, at
 * its latitude, in the frame of the surface shader (starSurface.ts): the
 * centre is (cos λ, sin λ, 0), east is -z, north is (-sin λ, cos λ, 0).
 * The bipole's axis points east and, in the hemisphere's sense, towards the
 * equator by the tilt (Joy's law); the leader sits at +axis, so `from` is
 * the leader's footpoint and `to` the follower's.
 *
 * `n` loops, side by side across the axis, each with feet a little nearer
 * or farther than the spots and a height of separation / 2 within ±20 %:
 * a loop's height is about half the distance between its feet
 * (Aschwanden 2004, *Physics of the Solar Corona*, semicircular loops).
 * Everything derives from the region's id, so the same region always has
 * the same loops.
 */
export function loopArcs(region: ActiveRegion, n: number): Prominence[] {
  const lat = region.latitude;
  const centre: Vec3 = [Math.cos(lat), Math.sin(lat), 0];
  const east: Vec3 = [0, 0, -1];
  const north: Vec3 = [-Math.sin(lat), Math.cos(lat), 0];
  const along = Math.cos(region.tilt);
  const up = -Math.sign(lat || 1) * Math.sin(region.tilt);
  const axis: Vec3 = [
    along * east[0] + up * north[0],
    along * east[1] + up * north[1],
    along * east[2] + up * north[2],
  ];
  // Across the axis, on the tangent plane: axis × centre.
  const across: Vec3 = [
    axis[1] * centre[2] - axis[2] * centre[1],
    axis[2] * centre[0] - axis[0] * centre[2],
    axis[0] * centre[1] - axis[1] * centre[0],
  ];
  const half = region.separation / 2;
  const arcs: Prominence[] = [];
  for (let k = 0; k < n; k += 1) {
    // Loops side by side, up to 0.2 half-separations either side of the axis.
    const shift = n > 1 ? (k / (n - 1) - 0.5) * 0.4 * Math.tan(half) : 0;
    // Feet 10 % nearer to 10 % farther than the spots.
    const reach = Math.tan(half * (0.9 + 0.2 * unit(region.id, k, 0)));
    const middle: Vec3 = [0, 1, 2].map((i) => centre[i]! + shift * across[i]!) as Vec3;
    const from = normalise([0, 1, 2].map((i) => middle[i]! + reach * axis[i]!) as Vec3);
    const to = normalise([0, 1, 2].map((i) => middle[i]! - reach * axis[i]!) as Vec3);
    const height = half * (0.8 + 0.4 * unit(region.id, k, 1));
    arcs.push({ from, to, height, thickness: 0.01 + 0.06 * height });
  }
  return arcs;
}

/** Which region ids to add and to remove, going from `shown` to `wanted`. */
export function diffRegions(shown: readonly number[], wanted: readonly number[]): { add: number[]; remove: number[] } {
  const have = new Set(shown);
  const want = new Set(wanted);
  return { add: wanted.filter((id) => !have.has(id)), remove: shown.filter((id) => !want.has(id)) };
}

/** What is kept of a region shown: its mesh (all its loops in one geometry), its first arc and its longitude now. */
interface Shown {
  mesh: Mesh<BufferGeometry, ShaderMaterial>;
  arc: Prominence;
  peakArea: number;
  longitude: number;
}

/** Regions kept past the `loopRegions` cap while they stay in the next ranks, so that a swap of ranks does not make a loop blink. */
const KEEP_MARGIN = 2;

/**
 * The loops of the largest active regions. Add `group` to the star's frame
 * (scaled like the star mesh) and call `update` as the clock moves. A
 * region's loops are one mesh with one material: one draw call a region.
 */
export class CoronalLoops {
  readonly group = new Group();
  private readonly shown = new Map<number, Shown>();
  private readonly perRegion: number;
  private readonly maxRegions: number;
  private readonly strands: number;
  private index = 0;
  private largest: { id: number } | null = null;

  /**
   * `material` makes the material of the loops of a region, given a running
   * index: a shader with a `uOpacity` and a `uTime` uniform (see
   * prominenceFragment). Each region owns its material, which `dispose` frees.
   */
  constructor(
    quality: QualityProfile,
    private readonly material: (index: number) => ShaderMaterial,
  ) {
    this.perRegion = quality.quality === 'high' ? LOOPS_HIGH : LOOPS_LIGHT;
    this.maxRegions = quality.loopRegions;
    // Several loops share the budget: a bundle of at most 8 strands each.
    this.strands = Math.min(quality.prominenceStrands, STRANDS_PER_LOOP);
  }

  /**
   * Keeps one mesh per region among the `loopRegions` largest (a region
   * already shown stays while it is in the `loopRegions + 2` largest);
   * turns each to its longitude now, opacity ∝ sqrt(area / peakArea) times
   * `fade`. Allocates nothing when the set of regions is unchanged.
   */
  update(regions: readonly RegionNow[], days: number, fade = 1): void {
    void days; // Placement is a function of the regions alone; the day is already in their longitudes.
    const limit = Math.min(regions.length, this.maxRegions + KEEP_MARGIN);
    let present = 0;
    let missing = false;
    for (let i = 0; i < limit; i += 1) {
      if (this.shown.has(regions[i]!.region.id)) present += 1;
      else if (i < this.maxRegions) missing = true;
    }
    if (missing || present !== this.shown.size) this.restructure(regions, limit);
    for (let i = 0; i < limit; i += 1) {
      const now = regions[i]!;
      const shown = this.shown.get(now.region.id);
      if (!shown) continue;
      shown.longitude = now.longitude;
      shown.mesh.rotation.y = now.longitude;
      const opacity = PEAK_OPACITY * fade * Math.sqrt(Math.min(1, now.area / shown.peakArea));
      shown.mesh.material.uniforms['uOpacity']!.value = opacity;
    }
    this.largest = regions.length > 0 && this.shown.has(regions[0]!.region.id) ? { id: regions[0]!.region.id } : null;
  }

  /** Drops the regions out of the kept ranks and builds those new in the top ones. */
  private restructure(regions: readonly RegionNow[], limit: number): void {
    const kept = new Set<number>();
    for (let i = 0; i < limit; i += 1) kept.add(regions[i]!.region.id);
    for (const id of [...this.shown.keys()]) if (!kept.has(id)) this.drop(id);
    for (let i = 0; i < Math.min(limit, this.maxRegions); i += 1) {
      const region = regions[i]!.region;
      if (!this.shown.has(region.id)) this.build(region);
    }
  }

  /** Advances the strands' shimmer to `time` seconds. */
  setTime(time: number): void {
    for (const shown of this.shown.values()) shown.mesh.material.uniforms['uTime']!.value = time;
  }

  /** The arc of the largest region's first loop, in the region's frame, for the interim eruption; null when none. */
  largestArc(): Prominence | null {
    return this.largest ? (this.shown.get(this.largest.id)?.arc ?? null) : null;
  }

  /** The id of the largest region (the one `largestArc` is of), or null. */
  largestId(): number | null {
    return this.largest ? this.largest.id : null;
  }

  /** The longitude now of region `id`, or null when it is not shown. */
  longitudeOf(id: number): number | null {
    return this.shown.get(id)?.longitude ?? null;
  }

  /** Frees every region's geometry and material. */
  dispose(): void {
    for (const id of [...this.shown.keys()]) this.drop(id);
  }

  private build(region: ActiveRegion): void {
    const arcs = loopArcs(region, this.perRegion);
    const parts = arcs.map((arc, k) => prominenceGeometry(arc, STAR_RADIUS, hash32(region.id, k, 2) % 1_000_003, this.strands));
    const geometry = parts.length === 1 ? parts[0]! : mergeGeometries(parts)!;
    if (geometry !== parts[0]) for (const part of parts) part.dispose();
    const mesh = new Mesh(geometry, this.material(this.index++));
    mesh.renderOrder = 2;
    this.group.add(mesh);
    this.shown.set(region.id, { mesh, arc: arcs[0]!, peakArea: region.peakArea, longitude: 0 });
  }

  private drop(id: number): void {
    const shown = this.shown.get(id);
    if (!shown) return;
    shown.mesh.geometry.dispose();
    shown.mesh.material.dispose();
    this.group.remove(shown.mesh);
    this.shown.delete(id);
  }
}
