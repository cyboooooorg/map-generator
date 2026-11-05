import { Quaternion, Vector3 } from 'three';
import { describe, expect, it } from 'vitest';
import { axesOnScreen } from './axes';

describe('axes gizmo', () => {
  it('shows x right, y up and z towards a camera that looks down -z', () => {
    const axes = Object.fromEntries(axesOnScreen(new Quaternion()).map((a) => [a.axis, a]));
    expect(axes['x']!.dx).toBeCloseTo(1, 10);
    expect(axes['y']!.dy).toBeCloseTo(-1, 10);
    expect(axes['z']!.front).toBe(true);
    expect(Math.hypot(axes['z']!.dx, axes['z']!.dy)).toBeCloseTo(0, 10);
  });

  it('turns with the camera and lists the back axes first', () => {
    // Turned a quarter round y: the camera looks down -x, so x points at it, drawn last.
    const turned = new Quaternion().setFromAxisAngle(new Vector3(0, 1, 0), Math.PI / 2);
    const axes = axesOnScreen(turned);
    expect(axes[2]!.axis).toBe('x');
    expect(axes[2]!.front).toBe(true);
    const z = axes.find((a) => a.axis === 'z')!;
    expect(z.dx).toBeCloseTo(-1, 10);
  });
});
