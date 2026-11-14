import { type Line, LineBasicMaterial, type Material, MeshBasicMaterial } from 'three';
import { describe, expect, it } from 'vitest';
import { LABEL_OUT, RotationDial } from './rotationDial';

describe('RotationDial', () => {
  it('lays 72 ticks, 12 of them long, with the 0° tick along the state’s zero', () => {
    const dial = new RotationDial();
    dial.set({ locked: false, zero: [1, 0, 0], pointerLongitude: 0, reading: Math.PI / 2, sun: null, host: null, sunAltitude: 0, bodies: [] }, 1.25);
    const { ticks } = dial.anchors();
    expect(ticks).toHaveLength(12);
    expect(ticks[0]!.clone().normalize().x).toBeCloseTo(1, 9);
    // 90° on: turned right-handed about +y from +x, towards −z.
    expect(ticks[3]!.clone().normalize().z).toBeCloseTo(-1, 9);
    ticks.forEach((t) => expect(t.y).toBeCloseTo(0, 12));
    // The numbers stand outside the ring.
    ticks.forEach((t) => expect(t.length()).toBeCloseTo(1.25 * LABEL_OUT, 9));
    dial.dispose();
  });

  it('runs the pointer to the ring at its longitude and hangs the markers at their heights', () => {
    const dial = new RotationDial();
    dial.set(
      {
        locked: true,
        zero: [0, 0, -1],
        pointerLongitude: Math.PI,
        reading: 0,
        sun: [1, 0, 0],
        host: [0, 0, -1],
        sunAltitude: Math.PI / 6,
        bodies: [
          { id: 'c', bearing: [0, 0, -1], altitude: 0, host: true },
          { id: 'c-2', bearing: [0, 0, 1], altitude: -Math.PI / 4, host: false },
        ],
      },
      2,
    );
    const { pointer, sun, host, bodies, arcTicks } = dial.anchors();
    [0, 0, -2].forEach((v, i) => expect(pointer.getComponent(i)).toBeCloseTo(v, 6));
    // The star 30° up its half-circle, at its bearing.
    [2 * Math.cos(Math.PI / 6), 2 * Math.sin(Math.PI / 6), 0].forEach((v, i) => expect(sun!.getComponent(i)).toBeCloseTo(v, 6));
    [0, 0, -2].forEach((v, i) => expect(host!.getComponent(i)).toBeCloseTo(v, 6));
    expect(bodies).toHaveLength(2);
    expect(bodies[1]!.y).toBeCloseTo(-2 * Math.SQRT1_2, 6);
    expect(bodies[1]!.z).toBeCloseTo(2 * Math.SQRT1_2, 6);
    // The half-circle's numbers, −90° to +90°, outside it, at the star's bearing.
    expect(arcTicks).toHaveLength(7);
    expect(arcTicks[0]!.y).toBeCloseTo(-2 * LABEL_OUT, 6);
    expect(arcTicks[3]!.x).toBeCloseTo(2 * LABEL_OUT, 6);
    // A body no longer chosen is dropped.
    dial.set({ locked: true, zero: [0, 0, -1], pointerLongitude: Math.PI, reading: 0, sun: null, host: null, sunAltitude: 0, bodies: [] }, 2);
    expect(dial.anchors().bodies).toHaveLength(0);
    expect(dial.anchors().arcTicks.every((t) => t === null)).toBe(true);
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

  it('draws the host as a half-circle its marker slides along, a moon as a quarter flipped below the ring', () => {
    const dial = new RotationDial();
    const state = (hostAltitude: number) => ({
      locked: true,
      zero: [0, 0, -1] as [number, number, number],
      pointerLongitude: Math.PI,
      reading: 0,
      sun: null,
      host: [0, 0, -1] as [number, number, number],
      sunAltitude: 0,
      bodies: [
        { id: 'c', bearing: [0, 0, -1] as [number, number, number], altitude: hostAltitude, host: true },
        { id: 'c-2', bearing: [0, 0, 1] as [number, number, number], altitude: -Math.PI / 4, host: false },
      ],
    });
    const arcs = () => dial.group.children.filter((o): o is Line => o.type === 'Line');
    const count = (line: Line) => line.geometry.getAttribute('position').count;
    const heights: number[] = [];
    for (const altitude of [0.01, -0.01]) {
      dial.set(state(altitude), 2);
      const [host, moon] = arcs();
      // Same geometry and orientation on both sides of 0: nothing flips.
      expect(count(host!)).toBe(73);
      expect(host!.scale.y).toBeGreaterThan(0);
      expect(count(moon!)).toBe(37);
      expect(moon!.scale.y).toBeLessThan(0);
      heights.push(dial.anchors().host!.y);
    }
    expect(heights[0]).toBeCloseTo(2 * Math.sin(0.01), 9);
    expect(heights[1]).toBeCloseTo(-2 * Math.sin(0.01), 9);
    dial.dispose();
  });

  it('takes the ring and the pointer from the theme but keeps the star’s colour', () => {
    const materials: Material[] = [];
    const dial = new RotationDial((material) => materials.push(material));
    const hexes = () => materials.map((m) => (m instanceof LineBasicMaterial || m instanceof MeshBasicMaterial ? m.color.getHexString() : ''));
    const before = hexes();
    dial.setColours({ ring: '#c46bff', pointer: '#ff7ae6' });
    const after = hexes();
    // Three ring materials, the star's two lines and marker, and the pointer.
    expect(after.filter((h) => h === 'c46bff')).toHaveLength(3);
    expect(after.filter((h) => h === 'ff7ae6')).toHaveLength(1);
    const kept = before.filter((h, i) => after[i] === h);
    expect(kept).toEqual(['c9b98a', 'c9b98a', 'c9b98a']);
    dial.dispose();
  });
});
