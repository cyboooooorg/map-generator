import { describe, expect, it } from 'vitest';
import { Color, Vector3 } from 'three';
import { BodyGuides, HOST_LABEL_BEYOND, hostLabelRadius, hostLineRadii } from './bodyGuides';

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
    // The host label stands one diameter beyond the surface towards the host.
    expect(moonlet.host!.z).toBeCloseTo(-(1.6 + HOST_LABEL_BEYOND), 6);
    // A host set afterwards starts from the surface too.
    guides.setHost({ position: [0, 0, -100], velocity: null });
    guides.host.updateMatrixWorld(true);
    expect(guides.points().pull!.length() - sphere.pull!.length()).toBeCloseTo(0.6, 6);
    guides.dispose();
  });
});

describe('hostLabelRadius', () => {
  it('hangs one diameter beyond the surface', () => {
    expect(hostLabelRadius(1, 2.4, 1000)).toBe(1 + HOST_LABEL_BEYOND);
    expect(hostLabelRadius(1.6, 3, 1000)).toBeCloseTo(3.6, 9);
  });

  it('goes to the tip of the pull arrow when that lies farther out', () => {
    expect(hostLabelRadius(1, 4.5, 1000)).toBe(4.5);
  });

  it('stays within the line to the host', () => {
    expect(hostLabelRadius(1, 2.4, 2)).toBe(2);
    expect(hostLabelRadius(1, 2.4, 0.5)).toBe(1);
  });

  it('holds still on the axis as the surface and the pull are set', () => {
    const guides = new BodyGuides(new Color('#ffffff'));
    guides.setHost({ position: [0, 0, -100], velocity: null });
    guides.update(false, true, 1);
    guides.host.updateMatrixWorld(true);
    const label = guides.points().host!;
    expect(label.x).toBeCloseTo(0, 9);
    expect(label.z).toBeCloseTo(-(1 + HOST_LABEL_BEYOND), 9);
    // A moonlet bulging 1.6 radii towards its host moves it out with the surface.
    guides.setSurface(1, () => 1.6);
    guides.host.updateMatrixWorld(true);
    expect(guides.points().host!.z).toBeCloseTo(-(1.6 + HOST_LABEL_BEYOND), 9);
    // Hidden with the host guides.
    guides.update(false, false, 1);
    expect(guides.points().host).toBeNull();
    guides.dispose();
  });
});
