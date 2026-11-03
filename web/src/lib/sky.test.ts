import { describe, expect, it } from 'vitest';
import type { WorldMetadata } from './api/types/WorldMetadata';
import { skyFromWorld } from './sky';

const world = (pressure: number | undefined, atmosphere?: object) =>
  ({ pressure_bar: pressure, preset: { atmosphere } }) as unknown as WorldMetadata;

describe('skyFromWorld', () => {
  it('gives airless worlds no sky', () => {
    expect(skyFromWorld(world(0, { haze: 0 }))).toBeNull();
  });

  it("gives worlds made before atmospheres Earth's blue sky", () => {
    const sky = skyFromWorld(world(undefined))!;
    expect(sky.betaR[2]).toBeGreaterThan(sky.betaR[0] * 5);
    expect(sky.top).toBeGreaterThan(1);
  });

  it('thickens and darkens the sky with pressure', () => {
    const thin = skyFromWorld(world(0.006))!;
    const dense = skyFromWorld(world(92))!;
    expect(dense.top).toBeGreaterThan(thin.top);
    expect(dense.betaR[2] * dense.heightR).toBeGreaterThan(thin.betaR[2] * thin.heightR * 10);
  });
});
