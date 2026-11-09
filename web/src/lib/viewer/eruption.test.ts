import { Group, ShaderMaterial } from 'three';
import { describe, expect, it } from 'vitest';
import type { ActiveRegion, RegionNow } from '../starActivity';
import { CoronalLoops } from './coronalLoops';
import { Eruption, ERUPTION_S, eruptionClock, eruptionLook, eruptive } from './eruption';
import { PROFILES } from './quality';

const REGION: ActiveRegion = {
  id: 42,
  born: 10,
  growth: 3,
  lifetime: 40,
  peakArea: 0.002,
  latitude: 0.35,
  longitude: 1.2,
  tilt: 0.1,
  separation: 0.2,
  polarity: 1,
};
const material = () =>
  new ShaderMaterial({
    uniforms: { uOpacity: { value: 0 }, uTime: { value: 0 }, uLift: { value: 1 }, uFray: { value: 0 } },
  });
const now = (id: number, area: number, longitude: number): RegionNow => ({ region: { ...REGION, id }, area, longitude });

/** A star's loops and its eruption, as a view holds them. */
function scene() {
  const arcs = new Group();
  const loops = new CoronalLoops(PROFILES.high, material);
  arcs.add(loops.group);
  const eruption = new Eruption(arcs, PROFILES.high.prominenceStrands, material);
  return { arcs, loops, eruption };
}

describe('eruption', () => {
  it('runs one cycle every ERUPTION_S seconds', () => {
    expect(eruptionClock(0)).toEqual({ cycle: 0, phase: 0 });
    const mid = eruptionClock(ERUPTION_S * 2.5);
    expect(mid.cycle).toBe(2);
    expect(mid.phase).toBeCloseTo(0.5, 12);
  });

  it('rises, frays and fades over its cycle, faded with the loops', () => {
    const start = eruptionLook(0, 1);
    expect(start.lift).toBe(1);
    expect(start.fray).toBe(0);
    expect(start.opacity).toBeCloseTo(1.1, 12);
    const risen = eruptionLook(0.7, 1);
    expect(risen.lift).toBeCloseTo(10, 12);
    expect(risen.fray).toBeCloseTo(0.35, 12);
    expect(eruptionLook(0.999, 1).opacity).toBeLessThan(1e-5);
    expect(eruptionLook(0.3, 0.5).opacity).toBeCloseTo(eruptionLook(0.3, 1).opacity / 2, 12);
  });

  it('erupts on G, K and M stars only', () => {
    expect(['G', 'K', 'M'].every(eruptive)).toBe(true);
    expect(['O', 'B', 'A', 'F'].some(eruptive)).toBe(false);
  });

  it('launches once a cycle from the largest region, follows it and is cut with it', () => {
    const { arcs, loops, eruption } = scene();
    loops.update([now(1, 0.002, 0.5), now(2, 0.001, 1)], 0);
    eruption.animate(loops, 0.1, true, 7, 1);
    expect(eruption.region).toBe(1);
    expect(eruption.turn).toBe(0.5);
    expect(arcs.children).toHaveLength(2);
    // The clock moves the region (the tick, the time bar, a zoom): the eruption turns with it.
    loops.update([now(1, 0.002, 2), now(2, 0.001, 1)], 3);
    eruption.follow(loops);
    expect(eruption.turn).toBe(2);
    // Its region leaves the shown set: cut, and not launched again in the same cycle.
    loops.update([now(2, 0.001, 1)], 6);
    eruption.follow(loops);
    expect(eruption.region).toBeNull();
    expect(arcs.children).toHaveLength(1);
    eruption.animate(loops, 1, true, 7, 1);
    expect(eruption.region).toBeNull();
    // The next cycle launches from the largest region then.
    eruption.animate(loops, ERUPTION_S + 0.1, true, 7, 1);
    expect(eruption.region).toBe(2);
  });

  it('is cut when the loops hide, and none starts on a quiet star', () => {
    const { arcs, loops, eruption } = scene();
    loops.update([now(1, 0.002, 0.5)], 0);
    eruption.animate(loops, 0.1, true, 7, 1);
    arcs.visible = false;
    eruption.follow(loops);
    expect(eruption.region).toBeNull();
    arcs.visible = true;
    eruption.reset();
    eruption.animate(loops, 0.2, false, 7, 1);
    expect(eruption.region).toBeNull();
    expect(arcs.children).toHaveLength(1);
  });
});
