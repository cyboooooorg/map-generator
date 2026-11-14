import { Vector3 } from 'three';
import { describe, expect, it, vi } from 'vitest';
import type { Vec3 } from '../skyBodies';
import { lookFor, skyShown, SkyBackdrop } from './skyBackdrop';
import { FIXED_SKY, skyLook } from './skyLook';
import { starField } from './stars';

const options = { hideHostStar: true, renderOrder: -3 };

describe('SkyBackdrop', () => {
  it('orients the group by the system axes', () => {
    const backdrop = new SkyBackdrop(options);
    const axes: [Vec3, Vec3, Vec3] = [
      [0, 0, 1],
      [1, 0, 0],
      [0, 1, 0],
    ];
    backdrop.setAxes(axes);
    const mapped = new Vector3(1, 2, 3).applyMatrix4(backdrop.group.matrix);
    // d.x * ax + d.y * ay + d.z * az
    expect(mapped.toArray()).toEqual([2, 3, 1]);
    expect(backdrop.group.matrixAutoUpdate).toBe(false);
    backdrop.setAxes(null);
    expect(backdrop.group.matrixAutoUpdate).toBe(true);
    expect(new Vector3(1, 2, 3).applyMatrix4(backdrop.group.matrix).toArray()).toEqual([1, 2, 3]);
  });

  it('keeps the same texture for an equal look', () => {
    const backdrop = new SkyBackdrop(options);
    const look = { ...FIXED_SKY, seed: 31 };
    backdrop.setLook(look);
    const first = backdrop.group.children[0] as unknown as { material: { uniforms: { uSky: { value: unknown } } } };
    const texture = first.material.uniforms.uSky.value;
    expect(texture).not.toBeNull();
    backdrop.setLook({ ...look });
    expect(first.material.uniforms.uSky.value).toBe(texture);
    backdrop.setLook({ ...FIXED_SKY, seed: 32 });
    expect(first.material.uniforms.uSky.value).not.toBe(texture);
  });

  it('draws the fixed sky for no place', () => {
    const backdrop = new SkyBackdrop(options);
    backdrop.setLook(skyLook(null));
    const count = backdrop.stars.geometry.getAttribute('position').count;
    expect(count).toBe(starField(FIXED_SKY).sizes.length);
  });

  it('disposes its geometry, materials and texture', () => {
    const backdrop = new SkyBackdrop(options);
    const starGeometry = vi.spyOn(backdrop.stars.geometry, 'dispose');
    const starMaterial = vi.spyOn(backdrop.stars.material, 'dispose');
    const cube = backdrop.group.children[0] as unknown as {
      geometry: { dispose(): void };
      material: { dispose(): void; uniforms: { uSky: { value: { dispose(): void } } } };
    };
    const cubeGeometry = vi.spyOn(cube.geometry, 'dispose');
    const cubeMaterial = vi.spyOn(cube.material, 'dispose');
    const texture = vi.spyOn(cube.material.uniforms.uSky.value, 'dispose');
    backdrop.dispose();
    for (const spy of [starGeometry, starMaterial, cubeGeometry, cubeMaterial, texture]) {
      expect(spy).toHaveBeenCalled();
    }
  });

  it('resolves the choice to a look and a visibility', () => {
    const own = { ...FIXED_SKY, seed: 99 };
    expect(lookFor('system', own)).toBe(own);
    expect(lookFor('fixed', own)).toBe(FIXED_SKY);
    expect(skyShown('off')).toBe(false);
    expect(skyShown('fixed')).toBe(true);
  });
});
