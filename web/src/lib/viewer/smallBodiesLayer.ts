// The star view's small bodies (see smallBodies.ts): a planetary system's
// belts as clouds of points turning at Kepler's pace, each planet's trojan
// swarms round its L4 and L5 points, moving with it, and the selected
// planet's five Lagrange points.
import { BufferAttribute, BufferGeometry, Color, Group, Points, PointsMaterial, Vector3 } from 'three';
import type { Belt } from '../api/types/Belt';
import type { Trojans } from '../api/types/Trojans';
import {
  BELT_POINTS,
  beltParticles,
  lagrangePoints,
  massShare,
  type LagrangePoint,
  type Particle,
  periodDays,
  random,
  swarmParticles,
  swarmPoints,
  type Trojan,
} from './smallBodies';

/** A planet as the small bodies need it. */
export interface SmallBodiesPlanet {
  orbit_au: number;
  mass_earth: number;
  trojans: Trojans | null;
}

/** The colour of a belt's points, by what it is made of; the trojans'; the Lagrange points', stable or not. */
const BELT_COLOUR: Record<Belt['composition'], string> = { rocky: '#c9ab86', mixed: '#bba78f', icy: '#a3c6e8' };
const TROJAN_COLOUR = '#e9c97c';
const STABLE_COLOUR = '#7fe0a0';
const UNSTABLE_COLOUR = '#ff9a7a';
/** Inclination spread of the trojans' orbits, radians (Jupiter's reach 40°). */
const TROJAN_TILT = (20 * Math.PI) / 180;

/** Screen positions (CSS px) the small bodies' labels go to, as StarView reports them. */
export interface SmallBodiesLabels {
  /** A point of each belt shown, by index in `belts`. */
  belts: { x: number; y: number; index: number }[];
  /** The middle of each swarm shown: its planet (index in `planets`), its point and its count. */
  swarms: { x: number; y: number; planet: number; point: 'L4' | 'L5'; count: number }[];
  /** The selected planet's Lagrange points. */
  lagrange: { x: number; y: number; name: LagrangePoint['name']; stable: boolean }[];
}

interface Cloud {
  points: Points<BufferGeometry, PointsMaterial>;
  particles: Particle[];
  belt: Belt;
}

interface Swarm {
  points: Points<BufferGeometry, PointsMaterial>;
  trojans: Trojan[];
  tilts: number[];
  planet: number;
  point: 'L4' | 'L5';
  count: number;
}

const cloud = (count: number, colour: string, size: number, opacity: number) =>
  new Points(
    new BufferGeometry().setAttribute('position', new BufferAttribute(new Float32Array(count * 3), 3)),
    new PointsMaterial({ color: colour, size, sizeAttenuation: false, transparent: true, opacity, depthWrite: false }),
  );

export class SmallBodiesLayer {
  readonly group = new Group();
  private readonly beltGroup = new Group();
  private readonly swarmGroup = new Group();
  private readonly lagrangeGroup = new Group();
  private clouds: Cloud[] = [];
  private swarms: Swarm[] = [];
  private lagrange: Points<BufferGeometry, PointsMaterial> | null = null;
  private lagrangeAt: LagrangePoint[] = [];
  private lagrangeShown: Vector3[] = [];
  private planets: SmallBodiesPlanet[] = [];
  private selected = -1;
  private starMass = 1;
  private at: (au: number) => number = (au) => au;
  private belts: Belt[] = [];

  constructor() {
    this.group.add(this.beltGroup, this.swarmGroup, this.lagrangeGroup);
    for (const g of [this.beltGroup, this.swarmGroup, this.lagrangeGroup]) g.renderOrder = 3;
  }

  /**
   * Builds the belts and swarms of a system round a star of `starMass`
   * Suns, on the view's scale `at` (AU to view units); `seed` keeps their
   * points the same each time.
   */
  build(belts: readonly Belt[], planets: readonly SmallBodiesPlanet[], starMass: number, at: (au: number) => number, seed: number): void {
    this.dispose();
    this.belts = [...belts];
    this.planets = [...planets];
    this.starMass = starMass;
    this.at = at;
    this.clouds = belts.map((belt, k) => {
      const particles = beltParticles(belt, BELT_POINTS[belt.kind], seed + 101 * k);
      const scattered = belt.kind === 'scattered';
      const points = cloud(particles.length, BELT_COLOUR[belt.composition], scattered ? 1.5 : 1.8, scattered ? 0.4 : 0.7);
      points.frustumCulled = false;
      this.beltGroup.add(points);
      return { points, particles, belt };
    });
    planets.forEach((planet, k) => {
      const total = (planet.trojans?.l4 ?? 0) + (planet.trojans?.l5 ?? 0);
      if (!planet.trojans || total === 0) return;
      const mu = massShare(planet.mass_earth, starMass);
      for (const point of ['L4', 'L5'] as const) {
        const count = point === 'L4' ? planet.trojans.l4 : planet.trojans.l5;
        const n = swarmPoints(count);
        if (n === 0) continue;
        // Drawn at least 4 % of the orbit wide: most swarms would be a line.
        const trojans = swarmParticles(point, n, planet.orbit_au, mu, seed + 7 * k + (point === 'L4' ? 1 : 2), 0.04 * planet.orbit_au);
        const next = random(seed + 13 * k);
        const tilts = trojans.map(() => (next() - 0.5) * 2 * TROJAN_TILT);
        const points = cloud(n, TROJAN_COLOUR, 2.6, 1);
        points.frustumCulled = false;
        this.swarmGroup.add(points);
        this.swarms.push({ points, trojans, tilts, planet: k, point, count });
      }
    });
  }

  /** The planet whose Lagrange points are marked (-1: none). */
  setSelected(index: number): void {
    this.selected = index;
    if (this.lagrange) {
      this.lagrange.geometry.dispose();
      this.lagrange.material.dispose();
      this.lagrangeGroup.remove(this.lagrange);
      this.lagrange = null;
    }
    const planet = this.planets[index];
    if (!planet) {
      this.lagrangeAt = [];
      return;
    }
    this.lagrangeAt = lagrangePoints(planet.orbit_au, massShare(planet.mass_earth, this.starMass));
    const geometry = new BufferGeometry().setAttribute('position', new BufferAttribute(new Float32Array(15), 3));
    const colours = new Float32Array(15);
    this.lagrangeAt.forEach((p, k) => {
      colours.set(new Color(p.stable ? STABLE_COLOUR : UNSTABLE_COLOUR).toArray(), 3 * k);
    });
    geometry.setAttribute('color', new BufferAttribute(colours, 3));
    this.lagrange = new Points(
      geometry,
      new PointsMaterial({ size: 7, sizeAttenuation: false, vertexColors: true, transparent: true, opacity: 0.95, depthWrite: false }),
    );
    this.lagrange.frustumCulled = false;
    this.lagrangeGroup.add(this.lagrange);
  }

  /** Which layers show. */
  setVisible(belts: boolean, trojans: boolean, lagrange: boolean): void {
    this.beltGroup.visible = belts;
    this.swarmGroup.visible = trojans;
    this.lagrangeGroup.visible = lagrange;
  }

  /**
   * Places every point at the system's day `days`, the planets where
   * `planetAt` puts them now (view units), `minGap` (view units) the least
   * the selected planet's L1 and L2 marks keep from its marker.
   */
  place(days: number, planetAt: readonly Vector3[], minGap: number): void {
    const at = this.at;
    const mass = this.starMass;
    for (const { points, particles } of this.clouds) {
      const array = points.geometry.getAttribute('position') as BufferAttribute;
      particles.forEach((p, k) => {
        const angle = p.phase + (2 * Math.PI * days) / periodDays(p.a, mass);
        const r = at(p.a);
        array.setXYZ(k, Math.cos(angle) * r, r * Math.sin(p.inclination) * Math.sin(angle - p.node), -Math.sin(angle) * r);
      });
      array.needsUpdate = true;
    }
    for (const swarm of this.swarms) {
      const planet = planetAt[swarm.planet];
      if (!planet) continue;
      const base = Math.atan2(-planet.z, planet.x);
      const array = swarm.points.geometry.getAttribute('position') as BufferAttribute;
      swarm.trojans.forEach((t, k) => {
        const angle = base + t.angle;
        const r = at(t.au);
        array.setXYZ(k, Math.cos(angle) * r, r * Math.sin(swarm.tilts[k]!) * Math.sin(t.angle * 3), -Math.sin(angle) * r);
      });
      array.needsUpdate = true;
    }
    const planet = planetAt[this.selected];
    if (this.lagrange && planet) {
      const base = Math.atan2(-planet.z, planet.x);
      const r0 = at(this.planets[this.selected]!.orbit_au);
      const array = this.lagrange.geometry.getAttribute('position') as BufferAttribute;
      this.lagrangeShown = this.lagrangeAt.map((p, k) => {
        let r = at(p.au);
        // L1 and L2 sit within the planet's Hill sphere: kept clear of its
        // marker so that they can be seen (not to scale).
        if (p.name === 'L1') r = Math.min(r, r0 - minGap);
        if (p.name === 'L2') r = Math.max(r, r0 + minGap);
        const angle = base + p.angle;
        const v = new Vector3(Math.cos(angle) * r, p.name === 'L1' || p.name === 'L2' ? planet.y : 0, -Math.sin(angle) * r);
        array.setXYZ(k, v.x, v.y, v.z);
        return v;
      });
      array.needsUpdate = true;
    }
  }

  /** Where the labels go, `screen` turning a point of the scene into CSS px (null off screen). */
  labels(screen: (v: Vector3) => { x: number; y: number } | null, planetAt: readonly Vector3[]): SmallBodiesLabels {
    const belts = this.beltGroup.visible
      ? this.belts.flatMap((belt, index) => {
          // On the far side of the star, each kind at its own angle, so the
          // labels do not pile up.
          const angle = Math.PI / 2 + ({ inner: -0.35, hildas: 0.25, outer: 0.1, scattered: -0.6 } as const)[belt.kind];
          const r = this.at(belt.kind === 'scattered' ? belt.inner_au * 2.2 : (belt.inner_au + belt.outer_au) / 2);
          const at = screen(new Vector3(Math.cos(angle) * r, 0, -Math.sin(angle) * r));
          return at ? [{ ...at, index }] : [];
        })
      : [];
    const swarms = this.swarmGroup.visible
      ? this.swarms.flatMap((swarm) => {
          const planet = planetAt[swarm.planet];
          if (!planet) return [];
          const angle = Math.atan2(-planet.z, planet.x) + (swarm.point === 'L4' ? 1 : -1) * (Math.PI / 3);
          const r = this.at(this.planets[swarm.planet]!.orbit_au);
          const at = screen(new Vector3(Math.cos(angle) * r, 0, -Math.sin(angle) * r));
          return at ? [{ ...at, planet: swarm.planet, point: swarm.point, count: swarm.count }] : [];
        })
      : [];
    const lagrange = this.lagrangeGroup.visible
      ? this.lagrangeShown.flatMap((v, k) => {
          const at = screen(v);
          const p = this.lagrangeAt[k]!;
          return at ? [{ ...at, name: p.name, stable: p.stable }] : [];
        })
      : [];
    return { belts, swarms, lagrange };
  }

  dispose(): void {
    for (const { points } of [...this.clouds, ...this.swarms]) {
      points.geometry.dispose();
      points.material.dispose();
    }
    this.beltGroup.clear();
    this.swarmGroup.clear();
    this.clouds = [];
    this.swarms = [];
  }
}
