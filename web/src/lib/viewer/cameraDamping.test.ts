// @vitest-environment jsdom
import { describe, expect, it } from 'vitest';
import { PerspectiveCamera } from 'three';
import { OrbitControls } from 'three/examples/jsm/controls/OrbitControls.js';

/** The star views' rule: only frames that follow an interaction advance the damping. */
function makeView() {
  const canvas = document.createElement('canvas');
  const camera = new PerspectiveCamera(40, 1, 0.1, 100);
  camera.position.set(0, 0, 5);
  const controls = new OrbitControls(camera, canvas);
  controls.enableDamping = true;
  let settling = false;
  controls.addEventListener('change', () => (settling = true));
  controls.addEventListener('start', () => (settling = true));
  const render = () => {
    const moving = settling && controls.update();
    if (!moving) settling = false;
  };
  return { camera, controls, render };
}

describe('star view camera damping', () => {
  it('does not creep on clock-only frames after the damping settles', () => {
    const { camera, controls, render } = makeView();
    controls.update();
    // A drag: the pointer events start the controls and leave some rotation to damp.
    const canvas = controls.domElement as HTMLCanvasElement;
    const fire = (type: string, x: number) =>
      canvas.dispatchEvent(
        new PointerEvent(type, { pointerId: 1, button: 0, clientX: x, clientY: 10, pointerType: 'mouse' }),
      );
    fire('pointerdown', 0);
    fire('pointermove', 40);
    fire('pointermove', 90);
    fire('pointerup', 90);
    let guard = 0;
    while (controls.update() && guard++ < 10000);
    expect(guard).toBeLessThan(10000);
    // Settle the view's own flag the same way the render loop would.
    for (let i = 0; i < 2; i++) render();
    const rest = camera.position.clone();
    for (let i = 0; i < 50; i++) render();
    expect(camera.position).toEqual(rest);
  });
});
