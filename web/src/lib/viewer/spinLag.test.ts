import { describe, expect, it } from 'vitest';
import { spinLag, spinShare } from './starSystem';

// A 10.4 h day (planet c of the fixture) at 60 frames a second.
const PERIOD_DAYS = 10.4 / 24;
const FRAME_S = 1 / 60;
const wrapTurn = (x: number) => x - Math.round(x);

/**
 * Runs the clock at `speeds` (days a second, one a frame) and returns the
 * turn the globe is seen to make each frame, wrapped to half a turn, and
 * the lag left.
 */
function run(speeds: number[], lag = 0, day = 0): { seen: number[]; lag: number; day: number } {
  const seen: number[] = [];
  for (const speed of speeds) {
    const step = speed * FRAME_S;
    const before = (day - lag) / PERIOD_DAYS;
    lag = spinLag(lag, step, PERIOD_DAYS);
    day += step;
    seen.push(wrapTurn((day - lag) / PERIOD_DAYS - before));
  }
  return { seen, lag, day };
}

describe('spinShare', () => {
  it('shows the whole turn while slow, none once a frame passes a quarter turn, easing between', () => {
    expect(spinShare(0)).toBe(1);
    expect(spinShare(0.1)).toBe(1);
    expect(spinShare(0.25)).toBe(0);
    expect(spinShare(1.2)).toBe(0);
    let last = 1;
    for (let r = 0; r <= 0.3; r += 0.005) {
      expect(spinShare(r)).toBeLessThanOrEqual(last);
      last = spinShare(r);
    }
  });
});

describe('spinLag', () => {
  it('turns a globe at its true rate at ×1', () => {
    const { seen, lag } = run(Array(600).fill(1));
    expect(Math.abs(lag)).toBeLessThan(1e-12);
    for (const turn of seen) expect(turn).toBeCloseTo(FRAME_S / PERIOD_DAYS, 9);
  });

  it('holds a globe still when its spin is too fast for the frames', () => {
    // 32 days a second: 1.2 turns a frame; 4096: noise without the hold.
    for (const speed of [32, 4096]) {
      const { seen } = run(Array(600).fill(speed));
      for (const turn of seen) expect(Math.abs(turn)).toBeLessThan(1e-6);
    }
  });

  it('never jumps while the speed sweeps up and down', () => {
    const up = Array.from({ length: 600 }, (_, i) => 2 ** ((12 * i) / 599));
    const { seen } = run([...up, ...[...up].reverse()]);
    // At most the fastest turn shown (0.15), plus a little catching up:
    // well short of the quarter turn where the eye loses the direction.
    for (const turn of seen) expect(Math.abs(turn)).toBeLessThanOrEqual(0.16);
  });

  it('catches up with the true hour once slow again, but not while paused', () => {
    const fast = run(Array(100).fill(32));
    expect(Math.abs(fast.lag)).toBeGreaterThan(0.01 * PERIOD_DAYS);
    // Paused: nothing turns.
    expect(spinLag(fast.lag, 0, PERIOD_DAYS)).toBe(fast.lag);
    // Thirty of its turns at ×1 (about 13 s): within a degree of its true hour.
    const slow = run(Array(Math.round((30 * PERIOD_DAYS) / FRAME_S)).fill(1), fast.lag, fast.day);
    expect(Math.abs(slow.lag / PERIOD_DAYS) * 360).toBeLessThan(1);
    for (const turn of slow.seen) expect(Math.abs(turn)).toBeLessThan(0.15);
  });
});
