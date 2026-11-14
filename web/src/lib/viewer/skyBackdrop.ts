// The sky behind a view: the baked gas and dust clouds (a cube map drawn at
// the far plane) and the star field, in one group so that a single matrix
// orients both. Shared by the planet, star and star-body views.
//
// The look is in the system frame; `setAxes` gives the group the system's
// axes in the view's frame. Without axes (a standalone world) the group is
// free for the view to turn about y, as the stars were before.

import {
  AdditiveBlending,
  BackSide,
  BoxGeometry,
  BufferAttribute,
  BufferGeometry,
  CubeTexture,
  DataTexture,
  Group,
  Mesh,
  Points,
  RGBAFormat,
  ShaderMaterial,
  SRGBColorSpace,
  UnsignedByteType,
  Vector3,
} from 'three';
import type { Vec3 } from '../skyBodies';
import { bakeSkyClouds, dustAt, type SkyFaces } from './skyClouds';
import { FIXED_SKY, type SkyLook } from './skyLook';
import { skyBackdropFragment, skyBackdropVertex, starsFragment, starsVertex } from './shaders';
import { starField } from './stars';

/** Which sky a view draws: the system's own, today's fixed one, or none. */
export type SkyChoice = 'system' | 'fixed' | 'off';

/** The look that `choice` draws, given the system's own. */
export function lookFor(choice: SkyChoice, own: SkyLook): SkyLook {
  return choice === 'fixed' ? FIXED_SKY : own;
}

/**
 * The sky a click on the switch's main button brings back after it turned
 * the sky off: the choice shown now, else the one remembered (the system's
 * own at first).
 */
export function skyToRestore(shown: SkyChoice, remembered: SkyChoice): SkyChoice {
  return shown !== 'off' ? shown : remembered;
}

/** Whether `choice` draws a sky at all. */
export function skyShown(choice: SkyChoice): boolean {
  return choice !== 'off';
}

function lookKey(look: SkyLook): string {
  return [look.seed, look.brightness, look.bandWidth, look.bulge, ...look.north, ...look.centre].join('|');
}

function cubeOf(faces: SkyFaces): CubeTexture {
  const images = faces.faces.map((data) => {
    const face = new DataTexture(data, faces.size, faces.size, RGBAFormat, UnsignedByteType);
    face.colorSpace = SRGBColorSpace;
    return face;
  });
  const cube = new CubeTexture(images);
  cube.colorSpace = SRGBColorSpace;
  cube.generateMipmaps = false;
  cube.needsUpdate = true;
  return cube;
}

export class SkyBackdrop {
  /** The cube backdrop and the star points. */
  readonly group = new Group();
  readonly stars: Points<BufferGeometry, ShaderMaterial>;
  private readonly cube: Mesh<BoxGeometry, ShaderMaterial>;
  private texture: CubeTexture | null = null;
  private faces: SkyFaces | null = null;
  private key = '';
  private choice: SkyChoice = 'off';
  private axes: [Vec3, Vec3, Vec3] | null = null;

  constructor(options: { hideHostStar: boolean; renderOrder: number }) {
    const defines: Record<string, number> = options.hideHostStar ? { HIDE_HOST_STAR: 1 } : {};
    const uniforms: ShaderMaterial['uniforms'] = {
      uPixelRatio: { value: 1 },
      uOpacity: { value: 1 },
    };
    if (options.hideHostStar) {
      // The host star's disc hides the stars behind it (see starsVertex).
      uniforms['uHostDir'] = { value: new Vector3(0, 0, 1) };
      uniforms['uHostCos'] = { value: 2 };
    }
    this.stars = new Points(
      new BufferGeometry(),
      new ShaderMaterial({
        vertexShader: starsVertex,
        fragmentShader: starsFragment,
        defines,
        uniforms,
        blending: AdditiveBlending,
        transparent: true,
        depthWrite: false,
      }),
    );
    // Directions, not positions: the geometry bounds are meaningless.
    this.stars.frustumCulled = false;
    this.stars.renderOrder = options.renderOrder;

    this.cube = new Mesh(
      new BoxGeometry(2, 2, 2),
      new ShaderMaterial({
        vertexShader: skyBackdropVertex,
        fragmentShader: skyBackdropFragment,
        uniforms: { uSky: { value: null }, uOpacity: { value: 1 } },
        side: BackSide,
        blending: AdditiveBlending,
        transparent: true,
        depthWrite: false,
      }),
    );
    this.cube.frustumCulled = false;
    this.cube.renderOrder = options.renderOrder - 1;

    // Nothing is baked here: the first look shown builds what it needs.
    this.cube.visible = false;
    this.group.add(this.cube, this.stars);
  }

  /**
   * Draws the sky of `choice` (the system's own look `own`, today's fixed
   * one, or none). Work is done lazily: nothing while the choice is 'off',
   * and clouds only for the system's own sky. The fixed sky is today's
   * exactly: the stars alone, no dust, not oriented by the system axes.
   * An equal sky changes nothing.
   */
  setSky(choice: SkyChoice, own: SkyLook): void {
    this.choice = choice;
    this.applyAxes();
    if (choice === 'off') return;
    const look = lookFor(choice, own);
    const clouds = choice === 'system';

    const key = `${clouds ? 'system' : 'fixed'}|${lookKey(look)}`;
    if (key !== this.key) {
      this.key = key;
      const field = starField(look, clouds ? (dir) => dustAt(look, dir) : undefined);
      // A new geometry: replacing the attributes of the old one would leave their buffers on the GPU.
      const geometry = new BufferGeometry();
      geometry.setAttribute('position', new BufferAttribute(field.directions, 3));
      geometry.setAttribute('colour', new BufferAttribute(field.colours, 3));
      geometry.setAttribute('size', new BufferAttribute(field.sizes, 1));
      const old = this.stars.geometry;
      this.stars.geometry = geometry;
      old.dispose();
    }

    this.cube.visible = clouds;
    if (clouds) {
      const faces = bakeSkyClouds(look);
      if (faces !== this.faces) {
        this.faces = faces;
        this.texture?.dispose();
        this.texture = cubeOf(faces);
        this.cube.material.uniforms['uSky']!.value = this.texture;
      }
    }
  }

  /**
   * The system's x, y and z axes in the view's frame: a system direction d
   * becomes d.x * ax + d.y * ay + d.z * az. Null leaves the group on its
   * own rotation, which the view may turn. The fixed sky ignores them: it
   * stays tied to the body, as it was before the system's own.
   */
  setAxes(axes: [Vec3, Vec3, Vec3] | null): void {
    this.axes = axes;
    this.applyAxes();
  }

  /** Whether the system's axes orient the sky now (not for the fixed sky). */
  get orientedByAxes(): boolean {
    return this.axes !== null && this.choice !== 'fixed';
  }

  private applyAxes(): void {
    const group = this.group;
    const axes = this.orientedByAxes ? this.axes : null;
    if (axes) {
      group.matrixAutoUpdate = false;
      group.matrix.makeBasis(new Vector3(...axes[0]), new Vector3(...axes[1]), new Vector3(...axes[2]));
    } else {
      group.matrixAutoUpdate = true;
      group.matrix.identity();
      group.updateMatrix();
    }
    group.matrixWorldNeedsUpdate = true;
  }

  setOpacity(o: number): void {
    this.stars.material.uniforms['uOpacity']!.value = o;
    this.cube.material.uniforms['uOpacity']!.value = o;
  }

  setPixelRatio(r: number): void {
    this.stars.material.uniforms['uPixelRatio']!.value = r;
  }

  dispose(): void {
    this.stars.geometry.dispose();
    this.stars.material.dispose();
    this.cube.geometry.dispose();
    this.cube.material.dispose();
    this.texture?.dispose();
    this.texture = null;
  }
}
