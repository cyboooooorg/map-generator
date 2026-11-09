// The epochs a world can be shown at (the time slider), mirroring the
// server (mapgen::epoch and WorldSummary.epochs_ma): worlds without liquid
// water today, from 1 Ga ahead back
// to their surface age, or to the end of their wet era if more recent.
// Epochs are whole Ma before present; negative ones are in the future.

import { bodyApi } from './api/client';
import type { WorldMetadata } from './api/types/WorldMetadata';
import { formatInt, formatUpTo, withUnit } from './format';
import { MIN_FORMAT_VERSION } from './formatVersion';
import { t } from './i18n/index.svelte';
import { TILE_RENDER_VERSION } from './viewer/TileLayer';

/** Below this surface pressure no water stays liquid (`atmosphere::LIQUID_WATER_BAR`). */
const LIQUID_WATER_BAR = 0.01;
/** How far ahead craters still to come are drawn, Ma (`craters::FUTURE_GA`). */
export const FUTURE_MA = 1000;

/** `[future, past]` in Ma, both included, or null when the world offers no other epoch. */
export function epochRange(meta: WorldMetadata): [number, number] | null {
  const age = meta.surface_age_ga;
  if (meta.format_version < MIN_FORMAT_VERSION || age <= 0) {
    return null;
  }
  if (meta.pressure_bar >= LIQUID_WATER_BAR) {
    return null;
  }
  const wetUntil = meta.preset.geology.wet_until;
  return [-FUTURE_MA, Math.floor(Math.min(age, wetUntil) * 1000)];
}

/**
 * The epochs the slider stops at, from the future to the past: every Ma
 * within 100 Ma of the present, every 10 Ma within 1 Ga, every 50 Ma
 * beyond, plus both ends. Rounded steps keep the URLs of the epochs shown
 * few, so the browser and the server caches serve them again.
 */
export function epochSteps([future, past]: readonly [number, number]): number[] {
  const step = (ma: number) => (Math.abs(ma) < 100 ? 1 : Math.abs(ma) < 1000 ? 10 : 50);
  const steps = new Set<number>([future, 0, past]);
  for (let ma = 0; ma <= past; ma += step(ma)) steps.add(ma);
  for (let ma = 0; ma >= future; ma -= step(ma)) steps.add(ma);
  return [...steps].filter((ma) => ma >= future && ma <= past).sort((a, b) => a - b);
}

/** The step of `steps` closest to `ma`. */
export function nearestStep(steps: readonly number[], ma: number): number {
  let best = steps[0] ?? 0;
  for (const s of steps) {
    if (Math.abs(s - ma) < Math.abs(best - ma)) best = s;
  }
  return best;
}

/** A duration in Ma, in Ga from 1 000 Ma on: "250 Ma", "3.8 Ga". */
export function formatDurationMa(ma: number): string {
  const abs = Math.abs(ma);
  return abs >= 1000 ? withUnit(formatUpTo(abs / 1000, 2), 'Ga') : withUnit(formatInt(abs), 'Ma');
}

/** An epoch for the slider: "present", "3.8 Ga ago", "in 250 Ma". */
export function formatEpoch(ma: number): string {
  if (ma === 0) return t.epochs.present;
  const time = formatDurationMa(ma);
  return ma > 0 ? t.epochs.ago({ time }) : t.epochs.ahead({ time });
}

/**
 * URL of a layer of a world at resolution `size` at an epoch
 * (`GET /api/systems/{id}/bodies/{body}/epoch/{ma}/{file}?size=`).
 */
export function epochFile(
  id: string,
  size: string,
  ma: number,
  file: 'colour.png' | 'normal.png' | 'elevation.png' | 'biomes.png',
  revision: number | null,
  scale: 1 | 2 | 4 = 1,
): string {
  const query = [`size=${encodeURIComponent(size)}`, `render=${TILE_RENDER_VERSION}`];
  if (revision !== null) query.unshift(`rev=${revision}`);
  if (scale !== 1) query.push(`scale=${scale}`);
  return `${bodyApi(id)}/epoch/${ma}/${file}?${query.join('&')}`;
}
