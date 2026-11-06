// The geometry of a prominence: not one tube but a bundle of thin strands,
// as H-alpha images of the Sun show them. The strands are pinned together
// at the feet, spread and braid along the arch, and each carries the
// attributes the prominence shader needs to make it flow and fray.
import { BufferAttribute, BufferGeometry, CatmullRomCurve3, TubeGeometry, Vector3 } from 'three';
import { mergeGeometries } from 'three/examples/jsm/utils/BufferGeometryUtils.js';
import type { Prominence } from './starSystem';

/** Strands in a prominence. */
export const STRANDS = 12;
/** Points along each strand's curve, and segments along and around its tube. */
const POINTS = 25;
const SEGMENTS = 64;
const SIDES = 5;

/** A small seeded random generator (mulberry32), for the strands' look. */
function random(seed: number): () => number {
  let state = (seed * 2_654_435_761) >>> 0;
  return () => {
    state = (state + 0x6d2b79f5) >>> 0;
    let z = state;
    z = Math.imul(z ^ (z >>> 15), z | 1);
    z ^= z + Math.imul(z ^ (z >>> 7), z | 61);
    return ((z ^ (z >>> 14)) >>> 0) / 4_294_967_296;
  };
}

/**
 * The strands of a prominence on a star of radius `radius`, merged into
 * one geometry. Besides the tube's `uv` (x along the strand), each vertex
 * carries `aStrand` (a number in [0, 1) proper to its strand) and `aDrift`
 * (the unit direction its strand frays towards when the prominence erupts).
 *
 * `strands` lightens it (the light profile, see quality.ts): the first
 * `strands` of the full bundle, each with half the segments along it.
 */
export function prominenceGeometry(arc: Prominence, radius: number, seed: number, strands = STRANDS): BufferGeometry {
  const next = random(seed);
  const from = new Vector3(...arc.from);
  const to = new Vector3(...arc.to);
  // Across the arch: perpendicular to the plane of the two feet and the centre.
  const across = new Vector3().crossVectors(from, to).normalize();
  const segments = strands < STRANDS ? SEGMENTS / 2 : SEGMENTS;
  const tubes: BufferGeometry[] = [];
  for (let s = 0; s < Math.min(strands, STRANDS); s += 1) {
    const strand = next();
    // Each strand rises a little higher or lower than the arch, sits to one
    // side of it and winds round its neighbours along the way.
    const height = arc.height * (0.75 + 0.5 * next());
    const side = (next() - 0.5) * 2;
    const lift = (next() - 0.5) * 2;
    const turns = 1 + 2 * next();
    const phase = next() * 2 * Math.PI;
    const width = arc.thickness * (1 + 1.2 * next());
    const points = Array.from({ length: POINTS }, (_, i) => {
      const t = i / (POINTS - 1);
      const arch = Math.sin(Math.PI * t);
      const up = from.clone().lerp(to, t).normalize();
      // Narrower at the feet, the bundle spreads widest at the top.
      const spread = width * (0.4 + 0.6 * Math.sqrt(arch));
      const wind = 2 * Math.PI * turns * t + phase;
      const offset = across
        .clone()
        .multiplyScalar(spread * (0.6 * side + 0.4 * Math.sin(wind)))
        .addScaledVector(up, spread * (0.6 * lift + 0.4 * Math.cos(wind)));
      return up.multiplyScalar(1 + height * arch).add(offset).multiplyScalar(radius);
    });
    const tube = new TubeGeometry(new CatmullRomCurve3(points), segments, arc.thickness * radius * (0.2 + 0.2 * next()), SIDES);
    const count = tube.getAttribute('position').count;
    tube.setAttribute('aStrand', new BufferAttribute(new Float32Array(count).fill(strand), 1));
    // Frays out sideways and along the arch, each strand its own way.
    const drift = across.clone().multiplyScalar(side + 0.3 * (next() - 0.5)).add(to.clone().sub(from).normalize().multiplyScalar(next() - 0.5));
    drift.normalize();
    const drifts = new Float32Array(count * 3);
    for (let v = 0; v < count; v += 1) drifts.set([drift.x, drift.y, drift.z], v * 3);
    tube.setAttribute('aDrift', new BufferAttribute(drifts, 3));
    tubes.push(tube);
  }
  const merged = mergeGeometries(tubes)!;
  for (const tube of tubes) tube.dispose();
  return merged;
}
