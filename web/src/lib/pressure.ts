// The generation form's pressure slider: logarithmic, so that thin
// atmospheres (Mars, 0.006 bar) are as easy to reach as thick ones (Venus,
// 92 bar), with its leftmost position for an airless world (0 bar).

/** Slider positions: 0 is airless, 1 to STEPS cover MIN_BAR to MAX_BAR. */
export const PRESSURE_STEPS = 1000;
/** Thinnest pressure on the slider, in bar; below it the number input still goes. */
export const MIN_BAR = 0.001;
/** Thickest pressure on the slider (the server's limit, `request::PRESSURE_BAR`). */
export const MAX_BAR = 1000;

const MIN_LOG = Math.log10(MIN_BAR);
const MAX_LOG = Math.log10(MAX_BAR);

/** Rounds to two significant digits, so dragging gives round values (0.006, 1, 93). */
function twoDigits(value: number): number {
  return Number(value.toPrecision(2));
}

/** The pressure in bar at a slider position. */
export function sliderToPressure(position: number): number {
  if (position <= 0) {
    return 0;
  }
  const t = (Math.min(position, PRESSURE_STEPS) - 1) / (PRESSURE_STEPS - 1);
  return twoDigits(10 ** (MIN_LOG + t * (MAX_LOG - MIN_LOG)));
}

/** The slider position of a pressure in bar (0 for airless, the nearest step otherwise). */
export function pressureToSlider(bar: number): number {
  if (!(bar > 0)) {
    return 0;
  }
  const t = (Math.log10(bar) - MIN_LOG) / (MAX_LOG - MIN_LOG);
  return Math.min(PRESSURE_STEPS, Math.max(1, Math.round(1 + t * (PRESSURE_STEPS - 1))));
}
