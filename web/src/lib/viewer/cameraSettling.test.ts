import { describe, expect, it } from 'vitest';
import { PerspectiveCamera, Vector3 } from 'three';
import { OrbitControls } from 'three/examples/jsm/controls/OrbitControls.js';
import { CameraSettling } from './cameraSettling';

/** The few DOM members OrbitControls touches, enough to drive a drag under Node. */
function fakeCanvas() {
  const target = new EventTarget();
  return Object.assign(target, {
    style: {} as Record<string, string>,
    ownerDocument: target,
    clientWidth: 800,
    clientHeight: 600,
    getRootNode: () => target,
    getBoundingClientRect: () => ({ left: 0, top: 0, width: 800, height: 600 }),
    setPointerCapture: () => {},
    releasePointerCapture: () => {},
    hasPointerCapture: () => false,
  });
}

function pointer(type: string, x: number, y: number) {
  return Object.assign(new Event(type, { cancelable: true }), {
    pointerId: 1,
    pointerType: 'mouse',
    button: 0,
    buttons: 1,
    clientX: x,
    clientY: y,
    pageX: x,
    pageY: y,
    ctrlKey: false,
    metaKey: false,
    shiftKey: false,
  });
}

describe('camera settling', () => {
  it('keeps the camera still on clock-only frames after a drag', () => {
    const canvas = fakeCanvas();
    const camera = new PerspectiveCamera(40, 800 / 600, 0.1, 100);
    camera.position.set(0, 0, 5);
    const controls = new OrbitControls(camera, canvas as unknown as HTMLElement);
    controls.enableDamping = true;
    const settling = new CameraSettling();
    controls.addEventListener('change', settling.mark);
    controls.addEventListener('start', settling.mark);

    canvas.dispatchEvent(pointer('pointerdown', 400, 300));
    canvas.dispatchEvent(pointer('pointermove', 440, 310));
    canvas.dispatchEvent(pointer('pointermove', 500, 320));
    canvas.dispatchEvent(pointer('pointerup', 500, 320));

    // The frames that follow the drag settle the damping.
    const start = camera.position.clone();
    let frames = 0;
    while (settling.step(() => controls.update()) && frames++ < 10000);
    expect(frames).toBeGreaterThan(1);
    expect(frames).toBeLessThan(10000);
    expect(camera.position.distanceTo(start)).toBeGreaterThan(0);

    // Frames drawn for the clock alone must not move it.
    const rest: Vector3 = camera.position.clone();
    for (let i = 0; i < 50; i++) settling.step(() => controls.update());
    expect(camera.position).toEqual(rest);

    // Calling update() every frame, as before the fix, does creep: the test would catch it.
    controls.update();
    expect(camera.position).not.toEqual(rest);
  });
});
