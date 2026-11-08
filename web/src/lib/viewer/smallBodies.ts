// A system's small bodies as the star view draws them
// (docs/analysis/13-asteroids.md): its belts as clouds of points on their
// orbits, each planet's trojans as two swarms round its L4 and L5 points,
// and the five Lagrange points of a planet. Far denser than real belts,
// whose bodies lie about a million km apart, as the planets' markers are
// larger than to scale.
import type { Belt } from '../api/types/Belt';

/** A point of a belt or a swarm: its orbit and where it starts. */
export interface Particle {
  /** Semi-major axis, AU. */
  a: number;
  /** Mean longitude at the system's day 0, radians. */
  phase: number;
  /** Tilt of its orbit and the longitude of its node, radians. */
  inclination: number;
  node: number;
}

/** A small, fast, seeded generator (mulberry32), so a belt's points stay the same each time. */
export function random(seed: number): () => number {
  let t = seed >>> 0;
  return () => {
    t = (t + 0x6d2b79f5) >>> 0;
    let r = Math.imul(t ^ (t >>> 15), 1 | t);
    r = (r + Math.imul(r ^ (r >>> 7), 61 | r)) ^ r;
    return ((r ^ (r >>> 14)) >>> 0) / 4_294_967_296;
  };
}

/** How many points draw a belt of this kind. */
export const BELT_POINTS: Record<Belt['kind'], number> = { inner: 3000, hildas: 300, outer: 2500, scattered: 1200 };

/**
 * `count` points of `belt`, spread evenly in distance across it but for
 * its gaps, their inclinations up to twice its typical one.
 */
export function beltParticles(belt: Belt, count: number, seed: number): Particle[] {
  const next = random(seed);
  const points: Particle[] = [];
  for (let tries = 0; points.length < count && tries < count * 4; tries++) {
    const a = belt.inner_au + next() * (belt.outer_au - belt.inner_au);
    if (belt.gaps.some(([centre, width]) => Math.abs(a - centre) < width / 2)) continue;
    // A scattered body's distance is its mean one; its orbit reaches out
    // far beyond (not drawn).
    const inclination = ((next() + next()) * belt.inclination_deg * Math.PI) / 180;
    points.push({ a, phase: next() * 2 * Math.PI, inclination, node: next() * 2 * Math.PI });
  }
  return points;
}

/** Days for an orbit of `a` AU round a star of `mass` Suns. */
export function periodDays(a: number, mass: number): number {
  return 365.25 * Math.sqrt(a ** 3 / Math.max(mass, 1e-3));
}

/** The planet's mass share `m / (M + m)`, from Earth masses and Suns. */
export function massShare(massEarth: number, starMass: number): number {
  const m = massEarth / 332_946;
  return m / (starMass + m);
}

/** A Lagrange point in the frame turning with its planet: its angle from the planet and its distance from the star (AU). */
export interface LagrangePoint {
  name: 'L1' | 'L2' | 'L3' | 'L4' | 'L5';
  angle: number;
  au: number;
  stable: boolean;
}

/** Routh's limit on the mass share for stable L4 and L5 points. */
export const ROUTH = 0.5 * (1 - Math.sqrt(23 / 27));

/**
 * The five Lagrange points of a planet at `a` AU from its star with mass
 * share `mu`, distances from the star: L3 lies at `a (1 + 5μ/12)` from the
 * system's centre of mass, which is `μa` from the star towards the planet,
 * so `a (1 − 7μ/12)` from the star.
 */
export function lagrangePoints(a: number, mu: number): LagrangePoint[] {
  const hill = Math.cbrt(mu / 3);
  const stable = mu < ROUTH;
  return [
    { name: 'L1', angle: 0, au: a * (1 - hill), stable: false },
    { name: 'L2', angle: 0, au: a * (1 + hill), stable: false },
    { name: 'L3', angle: Math.PI, au: a * (1 - (7 * mu) / 12), stable: false },
    { name: 'L4', angle: Math.PI / 3, au: a, stable },
    { name: 'L5', angle: -Math.PI / 3, au: a, stable },
  ];
}

/** Points drawn for a swarm of `count` trojans: from a few to a few hundred, by the logarithm. */
export function swarmPoints(count: number): number {
  return count <= 0 ? 0 : Math.min(400, Math.round(40 * Math.log10(1 + count)));
}

/** A trojan in the frame turning with its planet: its angle from the planet and its distance from the star, AU. */
export interface Trojan {
  angle: number;
  au: number;
}

/**
 * `count` trojans round `point` (L4 ahead, L5 behind) of a planet at `a` AU
 * with mass share `mu`: on tadpoles from about 30° to 95° from the planet,
 * as wide as `√(8μ/3) a` each side of its orbit at most, drawn at least
 * `minWidth` AU wide (a few planets' swarms would be invisible).
 */
export function swarmParticles(point: 'L4' | 'L5', count: number, a: number, mu: number, seed: number, minWidth = 0): Trojan[] {
  const next = random(seed);
  const side = point === 'L4' ? 1 : -1;
  const width = Math.max(Math.sqrt((8 * mu) / 3) * a, minWidth);
  return Array.from({ length: count }, () => {
    // Libration amplitude, and where on its tadpole the trojan is.
    const amplitude = (5 + 30 * next()) * (Math.PI / 180);
    const at = next() * 2 * Math.PI;
    const angle = side * (Math.PI / 3 + amplitude * Math.sin(at));
    const au = a + width * (amplitude / ((35 * Math.PI) / 180)) * Math.cos(at) * (0.6 + 0.4 * next());
    return { angle, au };
  });
}
