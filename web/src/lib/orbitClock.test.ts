import { describe, expect, it } from 'vitest';
import { orbitReadout } from './orbitClock';

describe('orbitReadout', () => {
  it('states the year of a planet, not the ten-day slider span', () => {
    // A 6.9-day year read "69.3-day orbit" when the span was handed over.
    expect(orbitReadout({ parent: null, orbitPeriodDays: 6.9 }, 0, 0)).toEqual({ day: 0, period: 6.9 });
    expect(orbitReadout({ parent: null, orbitPeriodDays: 33 }, 0.5, 0)?.period).toBe(33);
  });
  it('counts the day of the year from the season and the orbit clock', () => {
    const r = orbitReadout({ parent: null, orbitPeriodDays: 40 }, 0.5, 30);
    expect(r?.day).toBeCloseTo(10);
  });
  it('states the orbit of a moon, past the season', () => {
    expect(orbitReadout({ parent: 'b', orbitPeriodDays: 12 }, 0.7, 5)).toEqual({ day: 5, period: 12 });
  });
  it('is null without an orbit', () => {
    expect(orbitReadout({ parent: null, orbitPeriodDays: 0 }, 0, 0)).toBeNull();
  });
});
