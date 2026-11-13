// A system's thumbnail in the library and the compare bar: a small orrery
// drawn from its summary (components/SystemOrrery.svelte), the star in the
// middle, the habitable zone as a ring, each planet as a dot on its orbit,
// seen a little from above so that the orbits fill a wide box.
import type { SystemSummary } from './api/types/SystemSummary';
import { isGiantType } from './planetClass';

/** A planet of the orrery: its orbit and its dot, in box units. */
export interface OrreryPlanet {
  id: string;
  /** The orbit, an SVG path: an ellipse with the star at a focus, on the orrery's scale. */
  orbit: string;
  /** The orbit's mean radii (its semi-major axis on the scale), across and down the box. */
  rx: number;
  ry: number;
  /** The dot's centre and radius. */
  x: number;
  y: number;
  r: number;
  /** Its preview colour, linear RGB. */
  colour: [number, number, number];
  generated: boolean;
}

/** What the orrery draws, in a box `width` × `height` with its centre at (cx, cy). */
export interface Orrery {
  cx: number;
  cy: number;
  /** The star's disk. */
  starR: number;
  /** The habitable zone's inner and outer ellipses (rx, ry), null when unknown or off the box. */
  zone: { inner: [number, number]; outer: [number, number] } | null;
  planets: OrreryPlanet[];
}

/** Points along an orbit's path. */
const ORBIT_POINTS = 48;
/** A coordinate as the path writes it: two decimals are plenty at thumbnail size. */
const round = (v: number) => Math.round(v * 100) / 100;
/** Room kept round the outermost orbit, box units (a giant's dot fits in it). */
const PAD = 3;

/**
 * Where an orbit of `au` falls between the star's edge (`from`) and the
 * edge of the box (`to`), with `maxAu` the outermost: on a square-root
 * scale, so that the inner planets stay apart while far giants still fit.
 */
export function orbitRadius(au: number, maxAu: number, from: number, to: number): number {
  if (maxAu <= 0) return from;
  return from + (to - from) * Math.sqrt(Math.max(0, au) / maxAu);
}

/**
 * The orrery of a system in a `width` × `height` box: every planet with a
 * known orbit, its dot where an angle fixed by the system's seed and its
 * place puts it (so a system always looks the same). `scaleKind` 'log'
 * spaces the orbits by the logarithm of their distance, with a smaller
 * star, so that the inner planets of a crowded system spread out (the
 * compact cards use it; the default keeps the thumbnails as they were).
 */
export function orrery(system: SystemSummary, width: number, height: number, scaleKind: 'sqrt' | 'log' = 'sqrt'): Orrery {
  const cx = width / 2;
  const cy = height / 2;
  const scale = Math.min(width, height) / 40;
  const log = scaleKind === 'log';
  const starR = (log ? 1.8 : 2.6) * scale;
  const from = starR + (log ? 1.2 : 1.5) * scale;
  const planets = system.bodies.filter((b) => b.kind === 'planet' && b.orbit_au > 0);
  // The farthest point of any orbit sits at the edge.
  const maxAu = Math.max(0, ...planets.map((p) => p.orbit_au * (1 + Math.min(p.eccentricity, 0.99))));
  // The orbits fill the box: as wide as it is, as tall as it is.
  const toX = cx - PAD * scale;
  const toY = cy - PAD * scale;
  // The log scale starts a little inside the innermost orbit or zone edge.
  const inner = [...planets.map((p) => p.orbit_au * (1 - Math.min(p.eccentricity, 0.99))), ...(system.hz_inner_au > 0 ? [system.hz_inner_au] : [])];
  const minAu = Math.min(...inner) * 0.7;
  const radius = (au: number, to: number): number => {
    if (!log || maxAu <= minAu) return orbitRadius(au, maxAu, from, to);
    const f = (Math.log(Math.max(au, minAu)) - Math.log(minAu)) / (Math.log(maxAu) - Math.log(minAu));
    return from + (to - from) * Math.min(1, f);
  };
  const radii = (au: number): [number, number] => [radius(au, toX), radius(au, toY)];
  const zone =
    system.hz_outer_au > 0 && maxAu > 0 && system.hz_inner_au < maxAu
      ? { inner: radii(system.hz_inner_au), outer: radii(Math.min(system.hz_outer_au, maxAu)) }
      : null;
  return {
    cx,
    cy,
    starR,
    zone,
    planets: planets.map((planet, k) => {
      const [rx, ry] = radii(planet.orbit_au);
      const e = Math.min(Math.max(planet.eccentricity, 0), 0.99);
      const periapsis = planet.periapsis_deg * (Math.PI / 180);
      /** The point of the orbit at angle `theta` from the reference direction. */
      const at = (theta: number): [number, number] => {
        const au = (planet.orbit_au * (1 - e * e)) / (1 + e * Math.cos(theta - periapsis));
        const [ax, ay] = radii(au);
        return [cx + Math.cos(theta) * ax, cy + Math.sin(theta) * ay];
      };
      const points = Array.from({ length: ORBIT_POINTS }, (_, i) => at((i / ORBIT_POINTS) * 2 * Math.PI));
      const orbit = `M ${points.map(([x, y]) => `${round(x)} ${round(y)}`).join(' L ')} Z`;
      // The golden angle apart, from a start the seed sets.
      const [x, y] = at((system.seed % 360) * (Math.PI / 180) + k * 2.399_963);
      return {
        id: planet.id,
        orbit,
        rx,
        ry,
        x,
        y,
        r: (isGiantType(planet.body_type) ? 2.1 : 1.4) * scale,
        colour: planet.colour,
        generated: planet.generated,
      };
    }),
  };
}

/** A linear colour as CSS (the library's strip uses the same). */
export function cssColour([r, g, b]: [number, number, number]): string {
  const srgb = (c: number) => Math.round(255 * Math.min(1, Math.max(0, c)) ** (1 / 2.2));
  return `rgb(${srgb(r)} ${srgb(g)} ${srgb(b)})`;
}
