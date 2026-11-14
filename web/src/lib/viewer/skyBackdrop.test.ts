import { LinearFilter, Vector3 } from 'three';
import { describe, expect, it, vi } from 'vitest';
import type { Vec3 } from '../skyBodies';
import { lookFor, nebulaGeometry, skyShown, skyToRestore, SkyBackdrop } from './skyBackdrop';
import { FIXED_SKY, skyLook } from './skyLook';
import * as clouds from './skyClouds';
import { skyNebulae, type Nebula } from './skyNebulae';
import { random } from './skyRandom';
import { starField } from './stars';

// Without a worker the bake runs in the test, and a sky is a few seconds of it.
vi.setConfig({ testTimeout: 30_000 });

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

  const cubeOf = (backdrop: SkyBackdrop) =>
    backdrop.group.children[0] as unknown as {
      visible: boolean;
      geometry: { dispose(): void };
      material: { dispose(): void; uniforms: { uSky: { value: { dispose(): void } | null } } };
    };

  it('bakes nothing at construction nor while the sky is off', () => {
    const bake = vi.spyOn(clouds, 'bakeSkyClouds');
    const backdrop = new SkyBackdrop(options);
    backdrop.setSky('off', { ...FIXED_SKY, seed: 51 });
    expect(bake).not.toHaveBeenCalled();
    expect(cubeOf(backdrop).material.uniforms.uSky.value).toBeNull();
    expect(backdrop.stars.geometry.getAttribute('position')).toBeUndefined();
    backdrop.setSky('system', { ...FIXED_SKY, seed: 51 });
    expect(bake).toHaveBeenCalledTimes(1);
    bake.mockRestore();
  });

  it('keeps the same texture for an equal look', () => {
    const backdrop = new SkyBackdrop(options);
    const look = { ...FIXED_SKY, seed: 31 };
    backdrop.setSky('system', look);
    const first = cubeOf(backdrop);
    const texture = first.material.uniforms.uSky.value;
    expect(texture).not.toBeNull();
    backdrop.setSky('system', { ...look });
    expect(first.material.uniforms.uSky.value).toBe(texture);
    backdrop.setSky('system', { ...FIXED_SKY, seed: 32 });
    expect(first.material.uniforms.uSky.value).not.toBe(texture);
  });

  it('draws the fixed sky for no place', () => {
    const backdrop = new SkyBackdrop(options);
    backdrop.setSky('system', skyLook(null));
    const count = backdrop.stars.geometry.getAttribute('position').count;
    expect(count).toBe(starField(FIXED_SKY).sizes.length);
  });

  it('draws the fixed choice as today: stars only, no dust, not oriented by the axes', () => {
    const backdrop = new SkyBackdrop(options);
    const own = { ...FIXED_SKY, seed: 77, bulge: 0.5 };
    backdrop.setAxes([
      [0, 0, 1],
      [1, 0, 0],
      [0, 1, 0],
    ]);
    backdrop.setSky('fixed', own);
    const today = starField(FIXED_SKY);
    const attribute = (name: string) => backdrop.stars.geometry.getAttribute(name).array;
    expect(attribute('position')).toEqual(today.directions);
    expect(attribute('colour')).toEqual(today.colours);
    expect(cubeOf(backdrop).visible).toBe(false);
    expect(backdrop.orientedByAxes).toBe(false);
    expect(backdrop.group.matrixAutoUpdate).toBe(true);
    backdrop.setSky('system', own);
    expect(cubeOf(backdrop).visible).toBe(true);
    expect(backdrop.orientedByAxes).toBe(true);
    expect(backdrop.group.matrixAutoUpdate).toBe(false);
  });

  it('draws more stars in a long exposure than for the naked eye, and the fixed sky the same in both', () => {
    const count = (backdrop: SkyBackdrop) => backdrop.stars.geometry.getAttribute('position').count;
    const own = { ...FIXED_SKY, seed: 71 };
    const backdrop = new SkyBackdrop(options);
    backdrop.setExposure('eye');
    backdrop.setSky('system', own);
    const eye = count(backdrop);
    backdrop.setExposure('photo');
    const photo = count(backdrop);
    expect(photo).toBeGreaterThan(1.5 * eye);
    backdrop.setExposure('eye');
    expect(count(backdrop)).toBe(eye);
    backdrop.setSky('fixed', own);
    const fixed = count(backdrop);
    backdrop.setExposure('photo');
    expect(count(backdrop)).toBe(fixed);
  });

  it('replaces the star geometry rather than its attributes', () => {
    const backdrop = new SkyBackdrop(options);
    backdrop.setSky('system', { ...FIXED_SKY, seed: 61 });
    const old = backdrop.stars.geometry;
    const disposed = vi.spyOn(old, 'dispose');
    backdrop.setSky('system', { ...FIXED_SKY, seed: 62 });
    expect(backdrop.stars.geometry).not.toBe(old);
    expect(disposed).toHaveBeenCalled();
  });

  it('disposes its geometry, materials and texture', () => {
    const backdrop = new SkyBackdrop(options);
    backdrop.setSky('system', { ...FIXED_SKY, seed: 41 });
    const starGeometry = vi.spyOn(backdrop.stars.geometry, 'dispose');
    const starMaterial = vi.spyOn(backdrop.stars.material, 'dispose');
    const cube = cubeOf(backdrop);
    const cubeGeometry = vi.spyOn(cube.geometry, 'dispose');
    const cubeMaterial = vi.spyOn(cube.material, 'dispose');
    const texture = vi.spyOn(cube.material.uniforms.uSky.value!, 'dispose');
    backdrop.dispose();
    for (const spy of [starGeometry, starMaterial, cubeGeometry, cubeMaterial, texture]) {
      expect(spy).toHaveBeenCalled();
    }
  });

  it('brings back the choice shown, else the remembered one', () => {
    expect(skyToRestore('fixed', 'system')).toBe('fixed');
    expect(skyToRestore('off', 'fixed')).toBe('fixed');
    expect(skyToRestore('system', 'fixed')).toBe('system');
  });

  it('resolves the choice to a look and a visibility', () => {
    const own = { ...FIXED_SKY, seed: 99 };
    expect(lookFor('system', own)).toBe(own);
    expect(lookFor('fixed', own)).toBe(FIXED_SKY);
    expect(skyShown('off')).toBe(false);
    expect(skyShown('fixed')).toBe(true);
  });

  describe('nebulae', () => {
    const nebulaMeshes = (backdrop: SkyBackdrop) =>
      (backdrop.group.children[1] as unknown as {
        children: { visible: boolean; geometry: { dispose(): void }; material: { dispose(): void; uniforms: { uMap: { value: { dispose(): void } | null } } } }[];
      }).children;
    const look = { ...FIXED_SKY, seed: 91 };

    it('creates one mesh per nebula for the system sky, textured at once without a worker', () => {
      const backdrop = new SkyBackdrop(options);
      backdrop.setSky('system', look);
      const meshes = nebulaMeshes(backdrop);
      expect(meshes.length).toBe(skyNebulae(look).length);
      expect(backdrop.nebulaCount).toBe(meshes.length);
      expect(meshes.length).toBeGreaterThan(0);
      expect(meshes.every((m) => m.visible && m.material.uniforms.uMap.value)).toBe(true);
    });

    it('creates none for the fixed sky nor for no sky', () => {
      const backdrop = new SkyBackdrop(options);
      backdrop.setSky('fixed', look);
      expect(backdrop.nebulaCount).toBe(0);
      backdrop.setSky('system', look);
      expect(backdrop.nebulaCount).toBeGreaterThan(0);
      backdrop.setSky('off', look);
      expect(backdrop.nebulaCount).toBe(0);
    });

    it('swaps the textures with the exposure without rebuilding the catalogue', () => {
      const backdrop = new SkyBackdrop(options);
      backdrop.setSky('system', look);
      const before = nebulaMeshes(backdrop).map((m) => m);
      const textures = before.map((m) => m.material.uniforms.uMap.value);
      const bake = vi.spyOn(clouds, 'bakeSkyClouds');
      backdrop.setExposure('eye');
      const after = nebulaMeshes(backdrop);
      expect(after.every((m, k) => m === before[k])).toBe(true);
      expect(after.some((m, k) => m.material.uniforms.uMap.value !== textures[k])).toBe(true);
      expect(bake).toHaveBeenCalledWith(look, 'eye');
      // Back to the first exposure: its parts are cached, nothing is baked again.
      bake.mockClear();
      backdrop.setExposure('photo');
      expect(bake).not.toHaveBeenCalled();
      bake.mockRestore();
    });

    it('frees every nebula texture on dispose', () => {
      const backdrop = new SkyBackdrop(options);
      backdrop.setSky('system', look);
      const spies = nebulaMeshes(backdrop).map((m) => vi.spyOn(m.material.uniforms.uMap.value!, 'dispose'));
      expect(spies.length).toBeGreaterThan(0);
      backdrop.dispose();
      expect(spies.every((spy) => spy.mock.calls.length > 0)).toBe(true);
      expect(backdrop.nebulaCount).toBe(0);
    });
  });
});

describe('nebula patches', () => {
  /** The triangles' normals (counter-clockwise front faces, Three's default) of a patch. */
  const normals = (n: Nebula): Vector3[] => {
    const geometry = nebulaGeometry(n);
    const pos = geometry.getAttribute('position');
    const index = geometry.getIndex()!;
    const at = (i: number) => new Vector3().fromBufferAttribute(pos, index.getX(i));
    const out: Vector3[] = [];
    for (let t = 0; t < index.count; t += 3) {
      const [a, b, c] = [at(t), at(t + 1), at(t + 2)];
      out.push(b.clone().sub(a).cross(c.clone().sub(a)));
    }
    return out;
  };

  // The camera sits at the sphere's centre: a front face must face it, or
  // the default FrontSide culls every patch (as it did on develop 8b6f7be).
  it('face the centre of the sky sphere, whatever the direction and the roll', () => {
    const next = random(12345);
    for (let k = 0; k < 200; k++) {
      const z = 2 * next() - 1;
      const phi = 2 * Math.PI * next();
      const s = Math.sqrt(1 - z * z);
      const dir: Vec3 = [s * Math.cos(phi), z, s * Math.sin(phi)];
      const n: Nebula = { kind: 'emission', dir, radius: 0.02 + 0.2 * next(), roll: 2 * Math.PI * next(), seed: k, tint: 0.5 };
      for (const normal of normals(n)) expect(normal.dot(new Vector3(...dir))).toBeLessThan(0);
    }
  });

  it('face the centre for every nebula of a real sky, and draw from both sides', () => {
    const backdrop = new SkyBackdrop(options);
    const look = skyLook({ source: 'drawn', position_kpc: [4.5, 0.01, -0.14], axes: [[0, 1, 0], [-1, 0, 0], [0, 0, 1]], sky_seed: 2725666414 });
    for (const n of skyNebulae(look)) for (const normal of normals(n)) expect(normal.dot(new Vector3(...n.dir))).toBeLessThan(0);
    backdrop.setSky('system', look);
    const patches = (backdrop.group.children[1] as unknown as { children: { material: { side: number } }[] }).children;
    expect(patches.length).toBeGreaterThan(0);
    // DoubleSide (2): a mirrored basis (determinant -1) cannot cull them either.
    for (const p of patches) expect(p.material.side).toBe(2);
    backdrop.dispose();
  });
});

describe('the bake reaching the screen', () => {
  // The app renders on demand: a part that arrives must ask for a frame,
  // and its upload must not land all at once (60-130 ms long tasks).
  const look = { ...FIXED_SKY, seed: 77 };
  type Patch = { visible: boolean; material: { uniforms: { uMap: { value: { generateMipmaps: boolean; minFilter: number } | null } } } };
  const parts = (backdrop: SkyBackdrop) => {
    const cube = backdrop.group.children[0] as unknown as { visible: boolean };
    const patches = (backdrop.group.children[1] as unknown as { children: Patch[] }).children;
    return { cube, patches, shown: () => patches.filter((p) => p.visible).length };
  };
  const manual = () => {
    const ticks: (() => void)[] = [];
    return { ticks, schedule: (f: () => void) => void ticks.push(f), run: () => ticks.shift()?.() };
  };

  it('asks for a render as the parts arrive, a few each frame', () => {
    const frames = manual();
    const requestRender = vi.fn();
    const backdrop = new SkyBackdrop({ ...options, requestRender, schedule: frames.schedule });
    backdrop.setSky('system', look);
    const { cube, patches, shown } = parts(backdrop);
    // Baked (synchronously here), not yet shown: nothing until a frame runs.
    expect(cube.visible).toBe(false);
    expect(frames.ticks.length).toBe(1);
    frames.run();
    // The cube alone in its frame.
    expect(cube.visible).toBe(true);
    expect(shown()).toBe(0);
    expect(requestRender).toHaveBeenCalledTimes(1);
    let frame = 1;
    while (frames.ticks.length) {
      const before = shown();
      frames.run();
      frame += 1;
      const added = patches.filter((p) => p.visible).length - before;
      expect(added).toBeGreaterThan(0);
      expect(requestRender).toHaveBeenCalledTimes(frame);
    }
    expect(shown()).toBe(patches.length);
    // Spread over several frames, not one.
    expect(frame).toBeGreaterThan(3);
    backdrop.dispose();
  });

  it('drops the parts still queued for a sky no longer wanted', () => {
    const frames = manual();
    const backdrop = new SkyBackdrop({ ...options, schedule: frames.schedule });
    backdrop.setSky('system', look);
    backdrop.setSky('off', look);
    while (frames.ticks.length) frames.run();
    const { cube, patches } = parts(backdrop);
    expect(cube.visible).toBe(false);
    expect(patches.length).toBe(0);
    backdrop.dispose();
  });

  it('builds the nebula textures without mipmaps: no generateMipmap burst at upload', () => {
    const backdrop = new SkyBackdrop(options);
    backdrop.setSky('system', look);
    const { patches } = parts(backdrop);
    expect(patches.length).toBeGreaterThan(0);
    for (const p of patches) {
      expect(p.material.uniforms.uMap.value!.generateMipmaps).toBe(false);
      expect(p.material.uniforms.uMap.value!.minFilter).toBe(LinearFilter);
    }
    backdrop.dispose();
  });
});
