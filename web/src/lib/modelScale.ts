// How a model field's slider spans its limits: a logarithmic scale for the
// quantities that run over decades (with a "none" stop at 0 for those that
// may be 0), a linear one for the shares, and the percent shown for shares.
import type { Limits } from './api/types/Limits';
import { type ModelFieldKey, scaleOf } from './modelFields';

/** The slider runs over 0 to this many steps. */
export const SLIDER_STEPS = 1000;

/** Where the log slider of a field that may be 0 starts counting, just above its "none" stop. */
const LOG_FLOOR: Partial<Record<ModelFieldKey, number>> = { water_oceans: 0.001, co2: 0.01, n2: 0.01, ch4: 0.01 };

/** The age slider stops at the universe's, the number box takes the longest stellar life. */
const AGE_SLIDER: [number, number] = [0.1, 14];

const PERCENT: ReadonlySet<ModelFieldKey> = new Set(['core_mass_fraction', 'envelope_fraction']);

/** A share is shown in percent, every other value as it is sent. */
export const toShown = (key: ModelFieldKey, value: number): number => (PERCENT.has(key) ? Number((value * 100).toPrecision(6)) : value);
export const fromShown = (key: ModelFieldKey, shown: number): number => (PERCENT.has(key) ? shown / 100 : shown);

/** The floor of a log slider that has a "none" stop, null when the field's minimum is positive. */
const floorOf = (key: ModelFieldKey, limits: Limits): number | null => (limits.min > 0 ? null : (LOG_FLOOR[key] ?? 0.001));

const clamp = (n: number, lo: number, hi: number) => Math.min(hi, Math.max(lo, n));

function linearRange(key: ModelFieldKey, limits: Limits): [number, number] {
  return key === 'age_gyr' ? [AGE_SLIDER[0], Math.min(AGE_SLIDER[1], limits.max)] : [limits.min, limits.max];
}

/** The slider position (0 to {@link SLIDER_STEPS}) of a value. */
export function toPosition(key: ModelFieldKey, value: number, limits: Limits): number {
  if (scaleOf(key) === 'linear') {
    const [lo, hi] = linearRange(key, limits);
    return clamp(((value - lo) / (hi - lo)) * SLIDER_STEPS, 0, SLIDER_STEPS);
  }
  const floor = floorOf(key, limits);
  if (floor === null) {
    return clamp((Math.log(Math.max(value, limits.min) / limits.min) / Math.log(limits.max / limits.min)) * SLIDER_STEPS, 0, SLIDER_STEPS);
  }
  if (value <= 0) return 0;
  return clamp(1 + (Math.log(Math.max(value, floor) / floor) / Math.log(limits.max / floor)) * (SLIDER_STEPS - 1), 1, SLIDER_STEPS);
}

/** The value at a slider position, to three significant digits, inside the limits. */
export function fromPosition(key: ModelFieldKey, position: number, limits: Limits): number {
  const round = (v: number) => clamp(Number(v.toPrecision(3)), limits.min, limits.max);
  if (position >= SLIDER_STEPS) return scaleOf(key) === 'linear' ? round(linearRange(key, limits)[1]) : limits.max;
  if (scaleOf(key) === 'linear') {
    const [lo, hi] = linearRange(key, limits);
    return round(lo + (position / SLIDER_STEPS) * (hi - lo));
  }
  const floor = floorOf(key, limits);
  if (floor === null) return round(limits.min * (limits.max / limits.min) ** (position / SLIDER_STEPS));
  if (position <= 0) return 0;
  return round(floor * (limits.max / floor) ** ((position - 1) / (SLIDER_STEPS - 1)));
}
