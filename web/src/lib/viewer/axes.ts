// The world's axes as a camera sees them, for the axes gizmo in a corner of
// each view (components/AxesGizmo.svelte): x red, y green, z blue.
import { Quaternion, Vector3 } from 'three';

export interface ScreenAxis {
  axis: 'x' | 'y' | 'z';
  /** Its direction on screen, x right and y down, of length 1 when it lies in the screen's plane. */
  dx: number;
  dy: number;
  /** Whether it points towards the camera (drawn over the others). */
  front: boolean;
}

const UNITS = { x: new Vector3(1, 0, 0), y: new Vector3(0, 1, 0), z: new Vector3(0, 0, 1) } as const;

/** The three axes on screen for a camera turned by `camera`, back ones first. */
export function axesOnScreen(camera: Quaternion): ScreenAxis[] {
  const inverse = camera.clone().invert();
  return (['x', 'y', 'z'] as const)
    .map((axis) => {
      const v = UNITS[axis].clone().applyQuaternion(inverse);
      return { axis, dx: v.x, dy: -v.y, front: v.z > 0, depth: v.z };
    })
    .sort((a, b) => a.depth - b.depth)
    .map(({ axis, dx, dy, front }) => ({ axis, dx, dy, front }));
}
