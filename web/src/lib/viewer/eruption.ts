// The interim eruption shared by the system view (StarView) and the star
// view (StarBodyView): now and then, on an active star, the loop of the
// largest region rises, stretches, tears into strands and fades. It
// stands on its region (see eruptionTurn): whoever moves the clock (the
// tick, the time bar, a zoom), it turns with the region and is cut when
// the region leaves the shown set or the loops are withheld or hidden, so
// the surface stays a function of the day. The launch and the rise follow
// the animation's seconds, not the day: it is an animation, which a view
// removes when it pauses.
import { Mesh, type BufferGeometry, type Group, type ShaderMaterial } from 'three';
import { type CoronalLoops, eruptionTurn } from './coronalLoops';
import { prominenceGeometry } from './prominenceGeometry';
import { STAR_RADIUS } from './starSystem';

/** Seconds between two eruptions, while animated. */
export const ERUPTION_S = 9;

/** The eruption cycle `time` (seconds of animation) falls in, and the share of it elapsed, [0, 1). */
export function eruptionClock(time: number): { cycle: number; phase: number } {
  return { cycle: Math.floor(time / ERUPTION_S), phase: (time % ERUPTION_S) / ERUPTION_S };
}

/**
 * The eruption's shader values at `phase` of its cycle: it rises over the
 * first part of the cycle (`uLift`, radial stretch), its strands come apart
 * as torn field lines (`uFray`), and it is gone by the cycle's end, faded
 * like the loops by `fade` (CoronalLoops' loopFade).
 */
export function eruptionLook(phase: number, fade: number): { lift: number; fray: number; opacity: number } {
  const rise = Math.min(phase * 1.6, 1) ** 1.5;
  return { lift: 1 + 9 * rise, fray: 0.35 * rise, opacity: 1.1 * (1 - phase) ** 2 * fade };
}

/** Whether a star of spectral class `klass` (its type's first letter) erupts: G, K and M stars. */
export function eruptive(klass: string): boolean {
  return klass === 'M' || klass === 'K' || klass === 'G';
}

/** The eruption of a star: at most one at a time, a mesh added to the star's `arcs` group. */
export class Eruption {
  private mesh: Mesh<BufferGeometry, ShaderMaterial> | null = null;
  private from: number | null = null;
  private launchedCycle = -1;

  /**
   * `arcs` holds the loops (its visibility is the loops switch); `strands`
   * per bundle (QualityProfile.prominenceStrands); `material` makes the
   * eruption's material from a seed, as the loops' (uOpacity, uLift,
   * uFray, uTime).
   */
  constructor(
    private readonly arcs: Group,
    private readonly strands: number,
    private readonly material: (seed: number) => ShaderMaterial,
  ) {}

  /** The region the eruption under way launched from, or null when there is none. */
  get region(): number | null {
    return this.mesh ? this.from : null;
  }

  /** Its longitude now, radians, or null when there is none. */
  get turn(): number | null {
    return this.mesh ? this.mesh.rotation.y : null;
  }

  /**
   * Advances it to `time` seconds of animation: on an `active` star with
   * the loops shown, one launch per cycle from the largest region of
   * `loops` (an eruption cut mid-cycle does not start again), then its
   * rise and fade, times `fade`. `seed` varies its strands by star.
   */
  animate(loops: CoronalLoops, time: number, active: boolean, seed: number, fade: number): void {
    const { cycle, phase } = eruptionClock(time);
    if (this.mesh && !active) this.remove();
    if (!active || !this.arcs.visible) return;
    if (this.launchedCycle !== cycle) {
      this.remove();
      const arc = loops.largestArc();
      const region = loops.largestId();
      if (!arc || region === null) return;
      this.launchedCycle = cycle;
      this.mesh = new Mesh(
        prominenceGeometry({ ...arc, height: arc.height + 0.15 }, STAR_RADIUS, seed * 13 + cycle, this.strands),
        this.material(cycle),
      );
      this.mesh.renderOrder = 2;
      this.from = region;
      this.mesh.rotation.y = loops.longitudeOf(region) ?? 0;
      this.arcs.add(this.mesh);
    }
    if (!this.mesh) return;
    const look = eruptionLook(phase, fade);
    const u = this.mesh.material.uniforms;
    u['uLift']!.value = look.lift;
    u['uFray']!.value = look.fray;
    u['uOpacity']!.value = look.opacity;
  }

  /**
   * Keeps it on its region after `loops` were updated, whoever moved the
   * clock: it turns with the region, and is cut like the loops when they
   * are withheld (a fast clock, a small star), hidden, or when its region
   * left the shown set.
   */
  follow(loops: CoronalLoops): void {
    if (!this.mesh) return;
    const longitude = eruptionTurn(loops, this.from, this.arcs.visible);
    if (longitude === null) this.remove();
    else this.mesh.rotation.y = longitude;
  }

  /** Removes the eruption under way, if any (a pause: none hangs half-risen). */
  remove(): void {
    if (!this.mesh) return;
    this.arcs.remove(this.mesh);
    this.mesh.geometry.dispose();
    this.mesh.material.dispose();
    this.mesh = null;
    this.from = null;
  }

  /** Removes it and forgets the cycle launched (another star, or loops built again). */
  reset(): void {
    this.remove();
    this.launchedCycle = -1;
  }
}
