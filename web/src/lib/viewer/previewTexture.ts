// A body's preview (what the viewer draws before its maps exist, see
// shaders.ts `previewGlsl`) as a small equirectangular image, for the star
// view's markers: a giant's belts and zones, a rocky body's land and its
// polar caps, in the same colours and with the same cheap noise.
import type { Preview } from '../api/types/Preview';

/** Slots of the band-edge array in the shader: a giant has at most 31 bands. */
export const PREVIEW_EDGES = 32;
/** Latitude (degrees) where the polar regions of a preview with no edges start. */
const OLD_POLAR = 64;

const fract = (x: number) => x - Math.floor(x);
const smoothstep = (a: number, b: number, x: number) => {
  const t = Math.min(1, Math.max(0, (x - a) / (b - a)));
  return t * t * (3 - 2 * t);
};
const mix = (a: number, b: number, t: number) => a + (b - a) * t;

/** previewHash in previewGlsl. */
function hash(x: number, y: number, z: number): number {
  let px = fract(x * 0.318_309_9 + 0.1) * 17;
  let py = fract(y * 0.318_309_9 + 0.1) * 17;
  let pz = fract(z * 0.318_309_9 + 0.1) * 17;
  return fract(px * py * pz * (px + py + pz));
}

/** previewNoise in previewGlsl: value noise, smoothly interpolated. */
function noise(x: number, y: number, z: number): number {
  const ix = Math.floor(x);
  const iy = Math.floor(y);
  const iz = Math.floor(z);
  const s = (v: number) => v * v * (3 - 2 * v);
  const fx = s(x - ix);
  const fy = s(y - iy);
  const fz = s(z - iz);
  const h = (dx: number, dy: number, dz: number) => hash(ix + dx, iy + dy, iz + dz);
  return mix(
    mix(mix(h(0, 0, 0), h(1, 0, 0), fx), mix(h(0, 1, 0), h(1, 1, 0), fx), fy),
    mix(mix(h(0, 0, 1), h(1, 0, 1), fx), mix(h(0, 1, 1), h(1, 1, 1), fx), fy),
    fz,
  );
}

/** A linear colour channel as an sRGB byte. */
const toByte = (c: number) => {
  const v = Math.min(1, Math.max(0, c));
  return Math.round(255 * (v <= 0.003_130_8 ? 12.92 * v : 1.055 * v ** (1 / 2.4) - 0.055));
};

type Rgb = readonly [number, number, number];
const blend = (a: Rgb, b: Rgb, t: number): Rgb => [mix(a[0], b[0], t), mix(a[1], b[1], t), mix(a[2], b[2], t)];

/**
 * The latitudes (degrees, ascending) of the edges of a giant's bands: those
 * the generated globe will have, or, for a preview made before they were
 * stored, `bands` equal bands between the polar regions.
 */
export function previewEdges(preview: Preview): number[] {
  if (preview.bands < 1) return [];
  if (preview.edges.length === preview.bands + 1) return preview.edges;
  return Array.from({ length: preview.bands + 1 }, (_, k) => -OLD_POLAR + (2 * OLD_POLAR * k) / preview.bands);
}

/** The edges as the shader's fixed-size array, padded with the last edge. */
export function previewEdgeUniform(preview: Preview): number[] {
  const edges = previewEdges(preview).slice(0, PREVIEW_EDGES);
  const pad = edges[edges.length - 1] ?? 0;
  return Array.from({ length: PREVIEW_EDGES }, (_, k) => edges[k] ?? pad);
}

/**
 * The colour of a preview at the point of normal (x, y, z), linear RGB, as
 * previewColour in previewGlsl draws it.
 */
export function previewColourAt(preview: Preview, x: number, y: number, z: number): Rgb {
  const base: Rgb = preview.colours[0] ?? [0.4, 0.4, 0.4];
  const second: Rgb = preview.colours[1] ?? [0, 0, 0];
  const poles: Rgb = preview.colours[2] ?? [0, 0, 0];
  const lat = Math.asin(Math.min(1, Math.max(-1, y)));
  if (preview.bands > 0.5) {
    // The band after an odd number of edges is a belt, after an even number a zone.
    const edges = previewEdges(preview);
    const d = (lat * 180) / Math.PI + 3 * Math.sin(Math.atan2(x, z) * 6 + lat * 9);
    let below = 0;
    let nearest = 90;
    edges.forEach((e, i) => {
      if (d >= e) below += 1;
      if (i > 0 && i < edges.length - 1) nearest = Math.min(nearest, Math.abs(d - e));
    });
    const zone = below % 2 === 0;
    const colour = blend(zone ? base : second, zone ? second : base, 0.5 * (1 - smoothstep(0, 1.2, nearest)));
    const beyond = Math.max(edges[0]! - d, d - edges[edges.length - 1]!);
    return blend(colour, poles, smoothstep(-3, 3, beyond));
  }
  let colour = base;
  if (preview.colours.length > 1) {
    const n = noise(x * 2.5, y * 2.5, z * 2.5) * 0.65 + noise(x * 6, y * 6, z * 6) * 0.35;
    colour = blend(base, second, smoothstep(0.5, 0.56, n));
  }
  if (preview.colours.length > 2 && (poles[0] > 0 || poles[1] > 0 || poles[2] > 0)) {
    colour = blend(colour, poles, smoothstep(1.15, 1.25, Math.abs(lat)));
  }
  return colour;
}

/**
 * A preview as a `width` × `height` equirectangular RGBA image, sRGB bytes,
 * its first row the south pole (as a DataTexture reads it onto a sphere).
 */
export function previewTexture(preview: Preview, width = 64, height = 32): Uint8Array {
  const data = new Uint8Array(width * height * 4);
  for (let j = 0; j < height; j++) {
    const lat = -Math.PI / 2 + ((j + 0.5) / height) * Math.PI;
    for (let i = 0; i < width; i++) {
      const lon = ((i + 0.5) / width) * 2 * Math.PI - Math.PI;
      const c = previewColourAt(preview, Math.cos(lat) * Math.sin(lon), Math.sin(lat), Math.cos(lat) * Math.cos(lon));
      const k = (j * width + i) * 4;
      data[k] = toByte(c[0]);
      data[k + 1] = toByte(c[1]);
      data[k + 2] = toByte(c[2]);
      data[k + 3] = 255;
    }
  }
  return data;
}
