import { describe, expect, it } from 'vitest';
import { RotationDial } from './rotationDial';

describe('RotationDial', () => {
  it('lays 72 ticks, 12 of them long, with the 0° tick along the state’s zero', () => {
    const dial = new RotationDial();
    dial.set({ locked: false, zero: [1, 0, 0], pointerLongitude: 0, reading: Math.PI / 2, sun: null, host: null }, 1.25);
    const { ticks } = dial.anchors();
    expect(ticks).toHaveLength(12);
    expect(ticks[0]!.clone().normalize().x).toBeCloseTo(1, 9);
    // 90° on: turned right-handed about +y from +x, towards −z.
    expect(ticks[3]!.clone().normalize().z).toBeCloseTo(-1, 9);
    ticks.forEach((t) => expect(t.y).toBeCloseTo(0, 12));
    dial.dispose();
  });

  it('runs the pointer to the ring at its longitude and puts the markers on the ring', () => {
    const dial = new RotationDial();
    dial.set({ locked: true, zero: [0, 0, -1], pointerLongitude: Math.PI, reading: 0, sun: [1, 0, 0], host: [0, 0, -1] }, 2);
    const { pointer, sun, host } = dial.anchors();
    [0, 0, -2].forEach((v, i) => expect(pointer.getComponent(i)).toBeCloseTo(v, 6));
    expect(sun!.length()).toBeCloseTo(2, 6);
    expect(host!.length()).toBeCloseTo(2, 6);
    dial.dispose();
  });

  it('hides and fades with the globe', () => {
    const dial = new RotationDial();
    dial.update(false, 1);
    expect(dial.group.visible).toBe(false);
    dial.update(true, 0.5);
    expect(dial.group.visible).toBe(true);
    dial.dispose();
  });
});
