/** First `raw_data` format version whose temperatures are in °C. */
export const CELSIUS_VERSION = 4;

/**
 * Annual mean temperature, in °C, of an insolation level `t` on the former
 * `[0, 1]` scale (0 at Earth's poles, 1 at its equator): the curve the
 * generator fits to Earth's mean profile (`climate::celsius`). Worlds of
 * format version 3 and older stored `t`; this converts their layer.
 */
export function celsius(t: number): number {
  return -18 + 66.2 * t - 21.2 * t * t;
}
