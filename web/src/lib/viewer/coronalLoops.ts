// Coronal loops standing over the star's active regions: the arches of
// hot plasma that follow the field lines joining a bipole's two spots
// (the leader and the follower), as the Sun's X-ray and EUV images show
// them. Each region gets a few loops, built once when the region appears
// and dropped when it leaves; a frame only turns each region's group to
// its longitude, sets its opacity and advances the strands' shimmer.
import { Group, Mesh, type BufferGeometry, type ShaderMaterial } from 'three';
import { hash32, type ActiveRegion, type RegionNow } from '../starActivity';
import { prominenceGeometry } from './prominenceGeometry';
import type { QualityProfile } from './quality';
import { STAR_RADIUS, type Prominence } from './starSystem';

/** Loops standing over each region in the high and the light profiles. */
const LOOPS_HIGH = 3;
const LOOPS_LIGHT = 1;

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

/** What is kept of a region shown: its group, its loops and their first arc. */
interface Shown {
  group: Group;
  meshes: Mesh<BufferGeometry, ShaderMaterial>[];
  arc: Prominence;
  peakArea: number;
}

/**
 * The loops of the largest active regions. Add `group` to the star's frame
 * (scaled like the star mesh) and call `update` as the clock moves.
 */
export class CoronalLoops {
  readonly group = new Group();
  private readonly shown = new Map<number, Shown>();
  private readonly perRegion: number;
  private readonly maxRegions: number;
  private readonly strands: number;
  private index = 0;
  private largest: { id: number; longitude: number } | null = null;

  /**
   * `material` makes the material of the loop of a given running index: a
   * shader with a `uOpacity` and a `uTime` uniform (see prominenceFragment).
   * Each loop owns its material, which `dispose` frees.
   */
  constructor(
    quality: QualityProfile,
    private readonly material: (index: number) => ShaderMaterial,
  ) {
    this.perRegion = quality.quality === 'high' ? LOOPS_HIGH : LOOPS_LIGHT;
    this.maxRegions = quality.loopRegions;
    this.strands = quality.prominenceStrands;
  }

  /** Keeps one sub-group per region among the `loopRegions` largest; turns each to its longitude now, opacity ∝ sqrt(area / peakArea). */
  update(regions: readonly RegionNow[], days: number): void {
    void days; // Placement is a function of the regions alone; the day is already in their longitudes.
    const wanted = regions.slice(0, this.maxRegions);
    const { add, remove } = diffRegions([...this.shown.keys()], wanted.map((r) => r.region.id));
    for (const id of remove) this.drop(id);
    for (const now of wanted) if (add.includes(now.region.id)) this.build(now.region);
    for (const now of wanted) {
      const shown = this.shown.get(now.region.id)!;
      shown.group.rotation.y = now.longitude;
      const opacity = PEAK_OPACITY * Math.sqrt(Math.min(1, now.area / shown.peakArea));
      for (const mesh of shown.meshes) mesh.material.uniforms['uOpacity']!.value = opacity;
    }
    const first = wanted[0];
    this.largest = first ? { id: first.region.id, longitude: first.longitude } : null;
  }

  /** Advances the strands' shimmer to `time` seconds. */
  setTime(time: number): void {
    for (const shown of this.shown.values()) {
      for (const mesh of shown.meshes) mesh.material.uniforms['uTime']!.value = time;
    }
  }

  /** The arc of the largest region's first loop, in the region's frame, for the interim eruption; null when none. */
  largestArc(): Prominence | null {
    return this.largest ? (this.shown.get(this.largest.id)?.arc ?? null) : null;
  }

  /** The longitude of the largest region now: where `largestArc` stands on the star. */
  largestLongitude(): number {
    return this.largest?.longitude ?? 0;
  }

  /** Frees every region's geometries and materials. */
  dispose(): void {
    for (const id of [...this.shown.keys()]) this.drop(id);
  }

  private build(region: ActiveRegion): void {
    const group = new Group();
    const arcs = loopArcs(region, this.perRegion);
    const meshes = arcs.map((arc, k) => {
      const mesh = new Mesh(
        prominenceGeometry(arc, STAR_RADIUS, hash32(region.id, k, 2) % 1_000_003, this.strands),
        this.material(this.index++),
      );
      mesh.renderOrder = 2;
      group.add(mesh);
      return mesh;
    });
    this.group.add(group);
    this.shown.set(region.id, { group, meshes, arc: arcs[0]!, peakArea: region.peakArea });
  }

  private drop(id: number): void {
    const shown = this.shown.get(id);
    if (!shown) return;
    for (const mesh of shown.meshes) {
      mesh.geometry.dispose();
      mesh.material.dispose();
    }
    this.group.remove(shown.group);
    this.shown.delete(id);
  }
}
