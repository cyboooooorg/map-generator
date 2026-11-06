import { describe, expect, it } from 'vitest';
import { Color, Vector3 } from 'three';
import { BodyGuides, hostLineRadii, labelIndex } from './bodyGuides';

describe('hostLineRadii', () => {
  it('runs from the surface to the host, geometrically', () => {
    const radii = hostLineRadii(100, 5);
    expect(radii).toHaveLength(5);
    expect(radii[0]).toBe(1);
    expect(radii[4]).toBeCloseTo(100);
    expect(radii[2]).toBeCloseTo(10);
  });

  it('stays at the surface for a host closer than it', () => {
    expect(hostLineRadii(0.5, 3)).toEqual([1, 1, 1]);
  });
});

describe('labelIndex', () => {
  it('picks the middle of the longest run in view', () => {
    expect(labelIndex([false, true, true, true, false, true])).toBe(2);
    expect(labelIndex([true, false, true, true, true, true])).toBe(3);
  });

  it('is -1 when none is in view', () => {
    expect(labelIndex([false, false])).toBe(-1);
    expect(labelIndex([])).toBe(-1);
  });
});

describe('BodyGuides', () => {
  it('starts the spin arrow and the host arrows from the surface where they point', () => {
    const guides = new BodyGuides(new Color('#ffffff'));
    guides.setSpin(true);
    guides.setHost({ position: [0, 0, -100], velocity: [1, 0, 0] });
    guides.update(true, true, 1);
    const sphere = guides.points();
    // A moonlet: 1.4 radii out at its poles, 1.6 towards its host
    // (its long axis), 1.1 in the direction it moves.
    const at = (d: Vector3) => (Math.abs(d.z) > 0.9 ? 1.6 : Math.abs(d.x) > 0.9 ? 1.1 : 1);
    guides.setSurface(1.4, at);
    guides.spin.updateMatrixWorld(true);
    guides.host.updateMatrixWorld(true);
    const moonlet = guides.points();
    expect(moonlet.spin!.y).toBeCloseTo(1.4 * sphere.spin!.y, 6);
    expect(moonlet.pull!.length() - sphere.pull!.length()).toBeCloseTo(0.6, 6);
    expect(moonlet.velocity!.length() - sphere.velocity!.length()).toBeCloseTo(0.1, 6);
    // The host line still runs to the host's centre.
    expect(moonlet.host!.at(-1)!.z).toBeCloseTo(-100, 6);
    // A host set afterwards starts from the surface too.
    guides.setHost({ position: [0, 0, -100], velocity: null });
    guides.host.updateMatrixWorld(true);
    expect(guides.points().pull!.length() - sphere.pull!.length()).toBeCloseTo(0.6, 6);
    guides.dispose();
  });
});
