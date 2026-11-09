import { ShaderMaterial } from 'three';
import { describe, expect, it } from 'vitest';
import type { ActiveRegion, RegionNow } from '../starActivity';
import { CoronalLoops, diffRegions, loopArcs } from './coronalLoops';
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

const dot = (a: readonly number[], b: readonly number[]) => a[0]! * b[0]! + a[1]! * b[1]! + a[2]! * b[2]!;

describe('coronal loops', () => {
  it('stands loops on the sphere between the bipole', () => {
    for (const arc of loopArcs(REGION, 3)) {
      expect(Math.hypot(...arc.from)).toBeCloseTo(1, 9);
      expect(Math.hypot(...arc.to)).toBeCloseTo(1, 9);
      const apart = Math.acos(dot(arc.from, arc.to));
      expect(apart).toBeGreaterThan(0.75 * REGION.separation);
      expect(apart).toBeLessThan(1.25 * REGION.separation);
      expect(arc.height).toBeGreaterThan(0.4 * REGION.separation);
      expect(arc.height).toBeLessThan(0.6 * REGION.separation);
    }
  });

  it('is the same for the same region', () => expect(loopArcs(REGION, 3)).toEqual(loopArcs(REGION, 3)));

  it('puts the leader east and equatorward, as the surface shader does', () => {
    const [arc] = loopArcs(REGION, 1);
    // Region centred at longitude 0: east is -z, so the leader (from) has z < 0 and lower |y|.
    expect(arc!.from[2]).toBeLessThan(arc!.to[2]);
    expect(arc!.from[1]).toBeLessThan(arc!.to[1]);
  });

  it('diffs region sets', () => expect(diffRegions([1, 2, 3], [2, 3, 4])).toEqual({ add: [4], remove: [1] }));
});

describe('CoronalLoops', () => {
  const material = () => new ShaderMaterial({ uniforms: { uOpacity: { value: 0 }, uTime: { value: 0 } } });
  const now = (id: number, area: number, longitude: number): RegionNow => ({ region: { ...REGION, id }, area, longitude });

  it('builds 3 loops per region in high and 1 in light, once, and turns them', () => {
    const high = new CoronalLoops(PROFILES.high, material);
    high.update([now(1, 0.002, 0.5)], 0);
    const group = high.group.children[0]!;
    expect(group.children).toHaveLength(3);
    const mesh = group.children[0]!;
    high.update([now(1, 0.0005, 1.5)], 1);
    expect(high.group.children[0]).toBe(group);
    expect(group.children[0]).toBe(mesh);
    expect(group.rotation.y).toBe(1.5);
    expect((mesh as unknown as { material: ShaderMaterial }).material.uniforms['uOpacity']!.value).toBeCloseTo(0.45, 9);
    const light = new CoronalLoops(PROFILES.light, material);
    light.update([now(1, 0.002, 0)], 0);
    expect(light.group.children[0]!.children).toHaveLength(1);
    high.dispose();
    light.dispose();
  });

  it('keeps the largest regions only, drops the gone, and has no arc when empty', () => {
    const loops = new CoronalLoops(PROFILES.light, material);
    expect(loops.largestArc()).toBeNull();
    const many = Array.from({ length: 6 }, (_, i) => now(i + 1, 0.01 - i * 0.001, 0));
    loops.update(many, 0);
    expect(loops.group.children).toHaveLength(PROFILES.light.loopRegions);
    expect(loops.largestArc()).not.toBeNull();
    loops.update([], 1);
    expect(loops.group.children).toHaveLength(0);
    expect(loops.largestArc()).toBeNull();
  });
});
