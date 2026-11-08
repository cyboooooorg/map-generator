import { describe, expect, it } from 'vitest';
import { Vector3 } from 'three';
import { hiddenBySkyDisc, hiddenByShape } from './occlusion';

describe('hiddenByShape', () => {
  const sphere = () => 1;
  const camera = new Vector3(0, 0, 5);

  it('hides a point behind a sphere and not one beside or in front of it', () => {
    expect(hiddenByShape(camera, new Vector3(0, 0, -1.25), sphere, 1)).toBe(true);
    expect(hiddenByShape(camera, new Vector3(2, 0, -1.25), sphere, 1)).toBe(false);
    expect(hiddenByShape(camera, new Vector3(0, 0, 1.25), sphere, 1)).toBe(false);
  });

  it('hides a point behind a bulge that a unit sphere would miss', () => {
    // Elongated along x to 1.6 radii: a point just behind its end.
    const long = (d: Vector3) => {
      const n = d.clone().normalize();
      return 1 / Math.hypot(n.x / 1.6, n.y, n.z);
    };
    const behind = new Vector3(1.3, 0, -0.4);
    const end = new Vector3(1.3, 0, 5);
    expect(hiddenByShape(end, behind, sphere, 1)).toBe(false);
    expect(hiddenByShape(end, behind, long, 1.6)).toBe(true);
  });
});

describe('hiddenBySkyDisc', () => {
  const camera: [number, number, number] = [0, 0, 0];
  const host = { position: [0, 0, -10] as [number, number, number], radius: 2 };

  it('hides what lies behind a nearer disc, along its direction', () => {
    expect(hiddenBySkyDisc(camera, [0.1, 0, -1], 50, [host])).toBe(true);
    // The star, at infinity.
    expect(hiddenBySkyDisc(camera, [0, 0, -1], Infinity, [host])).toBe(true);
  });

  it('keeps what lies beside the disc, in front of it or behind the camera', () => {
    expect(hiddenBySkyDisc(camera, [1, 0, -1], 50, [host])).toBe(false);
    expect(hiddenBySkyDisc(camera, [0, 0, -1], 5, [host])).toBe(false);
    expect(hiddenBySkyDisc(camera, [0, 0, 1], 50, [host])).toBe(false);
  });

  it('never hides a body behind itself', () => {
    expect(hiddenBySkyDisc(camera, [0, 0, -1], 10, [host])).toBe(false);
  });
});
