// The range of the temperature colour scale, shared by the shader, the
// legend and the readouts.
import { CELSIUS_VERSION, celsius } from './climate';

/** The range Earth-like worlds are drawn on, °C. */
export const DEFAULT_TEMPERATURE_RANGE: readonly [number, number] = [-30, 40];

/** The narrowest range the scale is stretched over, °C. */
const MIN_SPAN = 20;

/**
 * The °C range the temperature layer is coloured on: -30 to 40 °C when most
 * of the world's temperatures fall inside it (an Earth-like world looks as it
 * always did, its coldest poles clamped to blue), else the world's own range
 * from the coldest to the hottest, rounded out to 10 °C, so a hot world or a
 * frozen one is not one flat colour.
 *
 * `layer` is the `temperature` entry of the world's layers (its decoded
 * min and max); `formatVersion` tells whether these are °C.
 */
export function temperatureRange(layer: { min: number; max: number } | null | undefined, formatVersion: number): [number, number] {
  const [lo, hi] = DEFAULT_TEMPERATURE_RANGE;
  if (!layer || !Number.isFinite(layer.min) || !Number.isFinite(layer.max)) return [lo, hi];
  const old = formatVersion < CELSIUS_VERSION;
  const min = old ? celsius(layer.min) : layer.min;
  const max = old ? celsius(layer.max) : layer.max;
  const span = max - min;
  const overlap = Math.min(max, hi) - Math.max(min, lo);
  if (span <= 0 ? min >= lo && max <= hi : overlap >= span / 2) return [lo, hi];
  let low = Math.floor(min / 10) * 10;
  let high = Math.ceil(max / 10) * 10;
  if (high - low < MIN_SPAN) {
    const mid = (low + high) / 2;
    low = mid - MIN_SPAN / 2;
    high = mid + MIN_SPAN / 2;
  }
  return [low, high];
}
