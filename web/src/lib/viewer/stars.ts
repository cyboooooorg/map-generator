// The star field behind the globe: fixed points on the sky sphere, so they
// turn with the planet when the camera orbits it.
//
// A fixed seed gives every world the same sky, so two compared worlds with
// linked cameras show the same stars. Most stars are faint and white; a
// few are bright, and a band of fainter stars crosses the sky, tilted to
// the planet's equator like the Milky Way.

/** One buffer per attribute, `count` stars long. */
export interface StarField {
  /** Unit directions, x y z per star. */
  directions: Float32Array;
  /** Linear RGB, already scaled by the star's brightness. */
  colours: Float32Array;
  /** Point size in CSS pixels. */
  sizes: Float32Array;
}

/** Stars spread over the whole sky, and stars along the band. */
export const STAR_COUNTS = { sky: 5500, band: 3000 } as const;

/** Angle between the band and the planet's equator, radians. */
const BAND_TILT = (60 * Math.PI) / 180;
/** Spread of the band around its great circle, radians (standard deviation). */
const BAND_WIDTH = (7 * Math.PI) / 180;

/** Star colours (linear RGB) and how common they are: cool red dwarfs are many, hot blue stars rare. */
const PALETTE: readonly { colour: readonly [number, number, number]; weight: number }[] = [
  { colour: [0.62, 0.72, 1.0], weight: 0.12 },
  { colour: [0.85, 0.9, 1.0], weight: 0.3 },
  { colour: [1.0, 1.0, 1.0], weight: 0.28 },
  { colour: [1.0, 0.92, 0.75], weight: 0.18 },
  { colour: [1.0, 0.75, 0.5], weight: 0.12 },
];

/** Small deterministic generator (mulberry32), uniform in [0, 1). */
function random(seed: number): () => number {
  let state = seed >>> 0;
  return () => {
    state = (state + 0x6d2b79f5) >>> 0;
    let z = state;
    z = Math.imul(z ^ (z >>> 15), z | 1);
    z ^= z + Math.imul(z ^ (z >>> 7), z | 61);
    return ((z ^ (z >>> 14)) >>> 0) / 4_294_967_296;
  };
}

/** The star field. The same seed always gives the same stars. */
export function starField(seed = 7): StarField {
  const next = random(seed);
  const count = STAR_COUNTS.sky + STAR_COUNTS.band;
  const directions = new Float32Array(count * 3);
  const colours = new Float32Array(count * 3);
  const sizes = new Float32Array(count);
  const cosTilt = Math.cos(BAND_TILT);
  const sinTilt = Math.sin(BAND_TILT);
  for (let i = 0; i < count; i += 1) {
    const inBand = i >= STAR_COUNTS.sky;
    let x: number;
    let y: number;
    let z: number;
    if (inBand) {
      // Around the band's great circle: latitude from a normal law (Box-Muller).
      const lon = next() * 2 * Math.PI;
      const lat = BAND_WIDTH * Math.sqrt(-2 * Math.log(1 - next())) * Math.cos(2 * Math.PI * next());
      const bx = Math.cos(lat) * Math.cos(lon);
      const by = Math.sin(lat);
      const bz = Math.cos(lat) * Math.sin(lon);
      // Tilted about the x axis.
      x = bx;
      y = by * cosTilt - bz * sinTilt;
      z = by * sinTilt + bz * cosTilt;
    } else {
      // Uniform on the sphere.
      y = 2 * next() - 1;
      const r = Math.sqrt(1 - y * y);
      const lon = next() * 2 * Math.PI;
      x = r * Math.cos(lon);
      z = r * Math.sin(lon);
    }
    directions.set([x, y, z], i * 3);
    // Brightness: many faint stars, few bright ones; the band's are fainter.
    const brightness = next() ** (inBand ? 6 : 4);
    let pick = next();
    let colour = PALETTE[PALETTE.length - 1]!.colour;
    for (const entry of PALETTE) {
      if (pick < entry.weight) {
        colour = entry.colour;
        break;
      }
      pick -= entry.weight;
    }
    const intensity = (inBand ? 0.18 : 0.25) + 0.75 * brightness;
    colours.set(
      colour.map((c) => c * intensity),
      i * 3,
    );
    sizes[i] = 1 + 2.2 * brightness;
  }
  return { directions, colours, sizes };
}
