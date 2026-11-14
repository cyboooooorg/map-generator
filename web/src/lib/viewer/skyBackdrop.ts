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

    this.group.add(this.cube, this.stars);
    this.setLook(FIXED_SKY);
  }

  /** Rebuilds the stars and uploads the clouds; an equal look changes nothing. */
  setLook(look: SkyLook): void {
    const key = lookKey(look);
    if (key === this.key) return;
    this.key = key;

    const field = starField(look, (dir) => dustAt(look, dir));
    const geometry = this.stars.geometry;
    geometry.setAttribute('position', new BufferAttribute(field.directions, 3));
    geometry.setAttribute('colour', new BufferAttribute(field.colours, 3));
    geometry.setAttribute('size', new BufferAttribute(field.sizes, 1));

    const faces = bakeSkyClouds(look);
    if (faces !== this.faces) {
      this.faces = faces;
      this.texture?.dispose();
      this.texture = cubeOf(faces);
      this.cube.material.uniforms['uSky']!.value = this.texture;
    }
  }

  /**
   * The system's x, y and z axes in the view's frame: a system direction d
   * becomes d.x * ax + d.y * ay + d.z * az. Null leaves the group on its
   * own rotation, which the view may turn.
   */
  setAxes(axes: [Vec3, Vec3, Vec3] | null): void {
    const group = this.group;
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
