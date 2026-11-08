import { describe, expect, it } from 'vitest';
import { cameraPlanes } from './cameraPlanes';

describe('cameraPlanes', () => {
  it('keeps the far plane 3 radii behind the centre without rings', () => {
    const { near, far } = cameraPlanes(2, 0);
    expect(far).toBe(6);
    expect(near).toBeCloseTo(0.2);
  });

  it('keeps a ring of 6 radii inside the far plane from the ring plane', () => {
    for (const altitude of [0.01, 1, 5, 40]) {
      // In the ring plane the ring's far edge lies at distance + outer.
      expect(cameraPlanes(altitude, 6).far).toBeGreaterThan(altitude + 1 + 6);
    }
  });

  it('keeps the dust rings of mgs-32659-2754625522 d (to 4.84) inside', () => {
    expect(cameraPlanes(10, 4.8366).far).toBeGreaterThan(10 + 1 + 4.8366);
  });

  it('keeps the near plane above zero at the ground', () => {
    expect(cameraPlanes(0, 6).near).toBe(1e-6);
  });
});
