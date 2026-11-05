// A system's thumbnail in the library and the compare bar: a small orrery
// drawn from its summary (components/SystemOrrery.svelte), the star in the
// middle, the habitable zone as a ring, each planet as a dot on its orbit,
// seen a little from above so that the orbits fill a wide box.
import type { SystemSummary } from './api/types/SystemSummary';

/** A planet of the orrery: its orbit's radii and its dot, in box units. */
export interface OrreryPlanet {
  id: string;
  /** The orbit, an ellipse round the box's centre. */
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

const GIANTS = new Set(['gas-giant', 'ice-giant', 'hot-jupiter']);
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
 * place puts it (so a system always looks the same).
 */
export function orrery(system: SystemSummary, width: number, height: number): Orrery {
  const cx = width / 2;
  const cy = height / 2;
  const scale = Math.min(width, height) / 40;
  const starR = 2.6 * scale;
  const from = starR + 1.5 * scale;
  const planets = system.bodies.filter((b) => b.kind === 'planet' && b.orbit_au > 0);
  const maxAu = Math.max(0, ...planets.map((p) => p.orbit_au));
  // The orbits fill the box: as wide as it is, as tall as it is.
  const toX = cx - PAD * scale;
  const toY = cy - PAD * scale;
  const radii = (au: number): [number, number] => [orbitRadius(au, maxAu, from, toX), orbitRadius(au, maxAu, from, toY)];
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
      // The golden angle apart, from a start the seed sets.
      const angle = (system.seed % 360) * (Math.PI / 180) + k * 2.399_963;
      return {
        id: planet.id,
        rx,
        ry,
        x: cx + Math.cos(angle) * rx,
        y: cy + Math.sin(angle) * ry,
        r: (GIANTS.has(planet.body_type) ? 2.1 : 1.4) * scale,
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
