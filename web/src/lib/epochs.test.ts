import { afterEach, describe, expect, it } from 'vitest';
import type { WorldMetadata } from './api/types/WorldMetadata';
import { epochFile, epochRange, epochSteps, formatEpoch, nearestStep } from './epochs';
import { setLocale } from './i18n/index.svelte';

/** Narrow no-break space. */
const S = '\u202f';

const meta = (overrides: Record<string, unknown>) =>
  ({
    format_version: 7,
    pressure_bar: 0,
    surface_age_ga: 4,
    preset: { geology: { regime: 'stagnant', wet_until: 4.6 } },
    ...overrides,
  }) as unknown as WorldMetadata;

describe('epochs', () => {
  afterEach(() => setLocale('en'));

  it('are offered by dry worlds with craters', () => {
    expect(epochRange(meta({}))).toEqual([-1000, 4000]);
    // Back to the end of the wet era when more recent.
    expect(epochRange(meta({ preset: { geology: { wet_until: 3.5 } } }))).toEqual([-1000, 3500]);
    expect(epochRange(meta({ pressure_bar: 1 }))).toBeNull();
    expect(epochRange(meta({ surface_age_ga: undefined }))).toBeNull();
  });

  it('step finely near the present, coarsely far from it', () => {
    const steps = epochSteps([-1000, 4000]);
    expect(steps[0]).toBe(-1000);
    expect(steps.at(-1)).toBe(4000);
    expect(steps).toContain(0);
    expect(steps).toContain(99);
    expect(steps).not.toContain(101);
    expect(steps).toContain(110);
    expect(steps).toContain(1050);
    expect(steps).not.toContain(1010);
    expect([...steps].sort((a, b) => a - b)).toEqual(steps);
    // A range ending off the steps keeps its end.
    expect(epochSteps([-1000, 3333]).at(-1)).toBe(3333);
    expect(nearestStep(steps, 1234)).toBe(1250);
  });

  it('are worded for the slider', () => {
    expect(formatEpoch(0)).toBe('present');
    expect(formatEpoch(3800)).toBe('3.8 Ga ago');
    expect(formatEpoch(-250)).toBe('in 250 Ma');
    setLocale('fr');
    expect(formatEpoch(3800)).toBe(`il y a 3,8${S}Ga`);
    expect(formatEpoch(-250)).toBe(`dans 250${S}Ma`);
  });

  it('name their layers', () => {
    expect(epochFile('systems/mgs-1-3/bodies/b', 2000, 'surface.png', 17, 4)).toBe(
      '/api/systems/mgs-1-3/bodies/b/epoch/2000/surface.png?rev=17&render=5&scale=4',
    );
  });
});
