import { describe, expect, it, vi } from 'vitest';
import { skyBakeParts } from './skyBake';
import { skyLook } from './skyLook';
import { skyNebulae } from './skyNebulae';

vi.setConfig({ testTimeout: 30_000 });

const look = skyLook({ source: 'Derived', position_kpc: [4.5, 0.01, -0.14], axes: [[0, 1, 0], [-1, 0, 0], [0, 0, 1]], sky_seed: 2725666414 } as never);

describe('skyBakeParts', () => {
  it('bakes the cube first, then every nebula once, the largest first', () => {
    const parts = [...skyBakeParts(look, 'photo')];
    expect(parts[0]!.part).toBe('cube');
    const list = skyNebulae(look);
    const indices = parts.slice(1).map((p) => (p.part === 'nebula' ? p.index : -1));
    expect([...indices].sort((a, b) => a - b)).toEqual(list.map((_, i) => i));
    const radii = indices.map((i) => list[i]!.radius);
    for (let k = 1; k < radii.length; k++) expect(radii[k]!).toBeLessThanOrEqual(radii[k - 1]!);
  });

  // The whole sky of system A (R 4.5 kpc, 50 nebulae) took about 10 s before; the browser shows it
  // part by part, but the target is 2 s on the author's machine. Loose here: CI machines are slower.
  it('bakes a sky near the core in a few seconds', () => {
    const t0 = performance.now();
    for (const _ of skyBakeParts({ ...look, seed: look.seed + 1 }, 'eye'));
    expect(performance.now() - t0).toBeLessThan(6000);
  });
});
