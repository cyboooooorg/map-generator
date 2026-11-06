import { describe, expect, it } from 'vitest';
import type { System } from './api/types/System';
import fixture from './systemSky.fixture.json';
import { libration, systemSky } from './systemSky';
import { dialFigureText, formatDayLength, formatDialAngle, formatLocalTime, rotationDial } from './rotationDial';
import type { Vec3 } from './skyBodies';

// The system of systemSky.test.ts: b (terran), c (a gas giant) and its
// moons c-1 and c-2, locked to it.
const system = fixture as unknown as System;
const textures = (id: string) => `${id}.png`;
const body = (id: string) => system.bodies.find((x) => x.id === id)!;

describe('rotation dial', () => {
  it('reads a free body’s turn against the stars, 0° at its equinox', () => {
    const day = body('b').spin.period_hours / 24;
    const at = (d: number) => rotationDial(system, 'b', d, 0, systemSky(system, 'b', d, textures))!.state;
    expect(at(0).reading).toBeCloseTo(0, 6);
    expect(at(day / 4).reading).toBeCloseTo(Math.PI / 2, 6);
    const s = at(day / 4);
    [-1, 0, 0].forEach((v, i) => expect(s.zero[i]).toBeCloseTo(v, 6));
    expect(s.pointerLongitude).toBe(0);
    expect(s.locked).toBe(false);
  });

  it('graduates a locked moon’s ring against its giant, the pointer swinging by the libration', () => {
    const eccentric = structuredClone(system);
    const orbit = eccentric.bodies.find((x) => x.id === 'c-1')!.orbit;
    orbit.eccentricity = 0.2;
    const d = orbit.period_days / 8;
    const dial = rotationDial(eccentric, 'c-1', d, 0, systemSky(eccentric, 'c-1', d, textures))!;
    expect(dial.state.locked).toBe(true);
    expect(dial.state.pointerLongitude).toBeCloseTo(Math.PI, 12);
    expect(Math.abs(dial.state.reading)).toBeCloseTo(Math.abs(libration(orbit, d)), 6);
    expect(dial.figures.libration).toBeCloseTo(libration(orbit, d), 12);
    expect(dial.figures.librationAmplitude).toBeCloseTo(0.4, 12);
    expect(dial.state.host).not.toBeNull();
  });

  it('gives the local time at the pointer’s meridian, noon under the sun, wrapping past midnight', () => {
    // Over a whole solar day of b the local time runs once through 0..24, never 24.
    const { solarHours } = rotationDial(system, 'b', 0, 0, systemSky(system, 'b', 0, textures))!.figures;
    const hours = Array.from({ length: 48 }, (_, k) => {
      const d = ((k / 48) * solarHours!) / 24;
      return rotationDial(system, 'b', d, 0, systemSky(system, 'b', d, textures))!.figures.localHours;
    });
    hours.forEach((h) => {
      expect(h).toBeGreaterThanOrEqual(0);
      expect(h).toBeLessThan(24);
    });
    // Noon: the sun at the pointer's longitude.
    const noon = hours.findIndex((h, k) => k > 0 && hours[k - 1]! < 12 && h >= 12);
    expect(noon).toBeGreaterThan(-1);
  });

  it('gives the sidereal and solar days, a moon taking its planet’s year', () => {
    const b = rotationDial(system, 'b', 0, 0, systemSky(system, 'b', 0, textures))!.figures;
    expect(b.siderealHours).toBeCloseTo(body('b').spin.period_hours, 9);
    expect(b.solarHours).toBeCloseTo(1 / (1 / body('b').spin.period_hours - 1 / (body('b').orbit.period_days * 24)), 6);
    const m = rotationDial(system, 'c-1', 0, 0, systemSky(system, 'c-1', 0, textures))!.figures;
    expect(m.siderealHours).toBeCloseTo(body('c-1').orbit.period_days * 24, 9);
    expect(m.solarHours).toBeCloseTo(1 / (1 / m.siderealHours - 1 / (body('c').orbit.period_days * 24)), 6);
  });

  it('treats a planet locked to its star: one marker, the sun standing still', () => {
    const locked = structuredClone(system);
    const b = locked.bodies.find((x) => x.id === 'b')!;
    b.spin.locked = true;
    b.spin.axial_tilt_deg = 0;
    const dial = rotationDial(locked, 'b', 10, 0, systemSky(locked, 'b', 10, textures))!;
    expect(dial.state.host).toBeNull();
    expect(dial.figures.solarHours).toBeNull();
    expect(Math.abs(dial.state.reading)).toBeCloseTo(Math.abs(libration(b.orbit, 10)), 6);
  });

  it('hides the sun marker when the sun is along the spin axis', () => {
    const sky = { ...systemSky(system, 'b', 0, textures), sun: [0, 1, 0] as Vec3 };
    expect(rotationDial(system, 'b', 0, 0, sky)!.state.sun).toBeNull();
  });

  it('has no dial for a body the system lacks', () => {
    expect(rotationDial(system, 'zz', 0, 0, systemSky(system, 'b', 0, textures))).toBeNull();
  });
});

describe('rotation dial figures', () => {
  const units = { hour: 'h', minute: 'min', day: 'd' };

  it('writes a local time as HH:MM, minutes floored, never 24:00', () => {
    expect(formatLocalTime(14.3333)).toBe('14:20');
    expect(formatLocalTime(23.9999)).toBe('23:59');
    expect(formatLocalTime(0)).toBe('00:00');
    expect(formatLocalTime(9.99)).toBe('09:59');
  });

  it('writes a day under 48 h in hours and minutes, else in days', () => {
    expect(formatDayLength(10.883, units)).toBe('10 h 53 min');
    expect(formatDayLength(0.9, units)).toBe('54 min');
    expect(formatDayLength(59.82, units)).toBe('2.49 d');
  });

  it('takes the locale’s unit words, space and numbers', () => {
    const fr = { hour: 'h', minute: 'min', day: 'j' };
    const number = (v: number, digits: number) => v.toFixed(digits).replace('.', ',');
    expect(formatDayLength(59.82, fr, { space: '\u202f', number })).toBe('2,49\u202fj');
    expect(formatDayLength(10.883, fr, { space: '\u202f', number })).toBe('10\u202fh 53\u202fmin');
  });

  it('reads the dial in whole degrees, or signed tenths for a locked body', () => {
    const rad = (deg: number) => (deg * Math.PI) / 180;
    expect(formatDialAngle(rad(137.4), false)).toBe('137°');
    expect(formatDialAngle(rad(359.7), false)).toBe('0°');
    expect(formatDialAngle(rad(0.3), true)).toBe('+0.3°');
    expect(formatDialAngle(rad(-0.3), true)).toBe('−0.3°');
    expect(formatDialAngle(rad(-0.01), true)).toBe('+0.0°');
  });

  it('gives a locked body’s libration the pointer’s sign, so that the panel and the label agree', () => {
    const eccentric = structuredClone(system);
    const orbit = eccentric.bodies.find((x) => x.id === 'c-1')!.orbit;
    orbit.eccentricity = 0.2;
    for (const f of [0.1, 0.6]) {
      const d = orbit.period_days * f;
      const dial = rotationDial(eccentric, 'c-1', d, 0, systemSky(eccentric, 'c-1', d, textures))!;
      const text = dialFigureText(dial, units);
      expect(text.locked).toBe(true);
      if (!text.locked) continue;
      expect(text.libration).toBe(formatDialAngle(dial.state.reading, true));
      expect(text.libration.startsWith(dial.state.reading < 0 ? '−' : '+')).toBe(true);
      expect(text.amplitude).toBe('22.9°');
    }
  });

  it('writes a free body’s rotation and days', () => {
    const dial = rotationDial(system, 'b', 0, 0, systemSky(system, 'b', 0, textures))!;
    const text = dialFigureText(dial, units);
    expect(text.locked).toBe(false);
    if (text.locked) return;
    expect(text.angle).toBe(formatDialAngle(dial.state.reading, false));
    expect(text.sidereal).toBe(formatDayLength(dial.figures.siderealHours, units));
  });
});

