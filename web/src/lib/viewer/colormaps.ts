// Colour ramps of the data layers. One definition feeds both the GLSL shader
// (generated functions) and the legend (CSS gradients), so they always match.

/** A colour stop: position in [0, 1] and an sRGB colour. */
type Stop = readonly [number, readonly [number, number, number]];

export type DataLayer = 'elevation' | 'temperature' | 'moisture' | 'flow' | 'volcanic' | 'coast' | 'floodplain' | 'wind' | 'cloudTops';

/** A ramp's stops; its legend texts are in the i18n catalog (`legends`). */
export interface Ramp {
  stops: readonly Stop[];
}

export const RAMPS: Record<DataLayer, Ramp> = {
  // t = (elevation above the coastline + 1) / 2: the coastline sits at 0.5.
  elevation: {
    stops: [
      [0.0, [8, 24, 80]],
      [0.42, [30, 90, 170]],
      [0.5, [90, 160, 220]],
      [0.5001, [70, 140, 70]],
      [0.6, [190, 180, 100]],
      [0.75, [140, 100, 65]],
      [1.0, [245, 245, 245]],
    ],
  },
  // t = (°C + 30) / 70.
  temperature: {
    stops: [
      [0.0, [45, 55, 160]],
      [0.25, [70, 160, 225]],
      [0.5, [240, 230, 140]],
      [0.75, [240, 140, 60]],
      [1.0, [175, 30, 30]],
    ],
  },
  // t = (moisture + 1) / 2.
  moisture: {
    stops: [
      [0.0, [140, 80, 35]],
      [0.35, [225, 205, 125]],
      [0.6, [110, 185, 85]],
      [1.0, [30, 100, 190]],
    ],
  },
  // t = log10(1 + km²) / 7.
  flow: {
    stops: [
      [0.0, [12, 16, 28]],
      [0.55, [25, 60, 115]],
      [0.75, [55, 160, 225]],
      [1.0, [225, 250, 255]],
    ],
  },
  volcanic: {
    stops: [
      [0.0, [30, 30, 36]],
      [0.4, [95, 60, 50]],
      [0.7, [210, 90, 30]],
      [1.0, [255, 220, 90]],
    ],
  },
  // t = √(km / cap): the first hundreds of km get most of the ramp.
  coast: {
    stops: [
      [0.0, [235, 245, 250]],
      [0.25, [110, 200, 190]],
      [0.6, [60, 110, 160]],
      [1.0, [40, 25, 70]],
    ],
  },
  // A giant's wind, t = (share of the strongest + 1) / 2: westward blue,
  // calm white, eastward red.
  wind: {
    stops: [
      [0.0, [40, 70, 170]],
      [0.3, [120, 170, 225]],
      [0.5, [240, 240, 235]],
      [0.7, [235, 150, 100]],
      [1.0, [170, 35, 40]],
    ],
  },
  // A giant's cloud tops, t over CLOUD_TOPS: deep belts in dark rust,
  // the cloud deck tan, high zones cream, storm tops white.
  cloudTops: {
    stops: [
      [0.0, [70, 35, 25]],
      [0.2, [140, 80, 50]],
      [0.35, [200, 160, 110]],
      [0.6, [235, 220, 185]],
      [1.0, [255, 255, 250]],
    ],
  },
  floodplain: {
    stops: [
      [0.0, [24, 26, 32]],
      [0.5, [70, 130, 100]],
      [1.0, [170, 235, 160]],
    ],
  },
};

const glslVec = ([r, g, b]: readonly [number, number, number]) =>
  `vec3(${(r / 255).toFixed(4)}, ${(g / 255).toFixed(4)}, ${(b / 255).toFixed(4)})`;

/** GLSL function `vec3 ramp_<name>(float t)` returning an sRGB colour. */
export function glslRamp(name: DataLayer): string {
  const stops = RAMPS[name].stops;
  const lines: string[] = [`vec3 ramp_${name}(float t) {`, '  t = clamp(t, 0.0, 1.0);'];
  for (let i = 1; i < stops.length; i += 1) {
    const [p0, c0] = stops[i - 1]!;
    const [p1, c1] = stops[i]!;
    lines.push(
      `  if (t <= ${p1.toFixed(4)}) return mix(${glslVec(c0)}, ${glslVec(c1)}, (t - ${p0.toFixed(4)}) / ${Math.max(p1 - p0, 1e-4).toFixed(4)});`,
    );
  }
  lines.push(`  return ${glslVec(stops[stops.length - 1]![1])};`, '}');
  return lines.join('\n');
}

/** CSS linear-gradient of a ramp, left to right. */
export function cssGradient(name: DataLayer): string {
  const stops = RAMPS[name].stops.map(([p, [r, g, b]]) => `rgb(${r} ${g} ${b}) ${(p * 100).toFixed(2)}%`);
  return `linear-gradient(to right, ${stops.join(', ')})`;
}

/**
 * The elevations a giant's cloud tops span (lowest, span), mapped onto the
 * `cloudTops` ramp: the cloud deck sits at 0.3 (`CLOUD_DECK` in mapgen's
 * giant_world.rs), belts and zones within about 0.08 of it, storm tops up
 * to 0.5.
 */
export const CLOUD_TOPS: readonly [number, number] = [0.2, 0.3];
