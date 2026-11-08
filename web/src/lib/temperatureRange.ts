// The range of the temperature colour scale, shared by the shader, the
// legend and the readouts.

/** The range Earth-like worlds are drawn on, °C. */
export const DEFAULT_TEMPERATURE_RANGE: readonly [number, number] = [-30, 40];

/** The narrowest range the scale is stretched over, °C. */
const MIN_SPAN = 20;

/** Share of the pixels left out at each end of the span (lava pools, the odd pole). */
const TAIL = 0.002;

/**
 * The coldest and the hottest temperature of a world, °C, from its raw
 * 16-bit layer: the 0.2th and 99.8th percentiles, so a few extreme pixels do
 * not stretch the scale. `info` is the layer's encoding in `world.json`
 * (`min` and `max` there are the range the 16 bits cover, not the data's).
 */
export function temperatureSpan(
  pixels: Uint16Array,
  info: { min: number; max: number },
): { min: number; max: number } {
  const counts = new Uint32Array(65536);
  for (let i = 0; i < pixels.length; i += 1) counts[pixels[i]!]! += 1;
  const cut = pixels.length * TAIL;
  let low = 0;
  let seen = 0;
  while (low < 65535 && seen + counts[low]! <= cut) seen += counts[low++]!;
  let high = 65535;
  seen = 0;
  while (high > low && seen + counts[high]! <= cut) seen += counts[high--]!;
  const scale = (info.max - info.min) / 65535;
  return { min: info.min + low * scale, max: info.min + high * scale };
}

/**
 * The °C range the temperature layer is coloured on: -30 to 40 °C when most
 * of the world's temperatures fall inside it (an Earth-like world looks as it
 * always did, its coldest poles clamped to blue), else the world's own range
 * from the coldest to the hottest, rounded out to 10 °C, so a hot world or a
 * frozen one is not one flat colour.
 */
export function temperatureRange(span: { min: number; max: number } | null | undefined): [number, number] {
  const [lo, hi] = DEFAULT_TEMPERATURE_RANGE;
  if (!span || !Number.isFinite(span.min) || !Number.isFinite(span.max)) return [lo, hi];
  const { min, max } = span;
  const width = max - min;
  const overlap = Math.min(max, hi) - Math.max(min, lo);
  if (width <= 0 ? min >= lo && max <= hi : overlap >= width / 2) return [lo, hi];
  let low = Math.floor(min / 10) * 10;
  let high = Math.ceil(max / 10) * 10;
  if (high - low < MIN_SPAN) {
    const mid = (low + high) / 2;
    low = mid - MIN_SPAN / 2;
    high = mid + MIN_SPAN / 2;
  }
  return [low, high];
}
