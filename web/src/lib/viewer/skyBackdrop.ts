// The sky behind a view: the baked gas and dust clouds (a cube map drawn at
// the far plane), the nebulae (one patch of the far sphere each) and the star
// field, in one group so that a single matrix orients all of them. Shared by the planet, star and star-body views.
//
// The look is in the system frame; `setAxes` gives the group the system's
// axes in the view's frame. Without axes (a standalone world) the group is
// free for the view to turn about y, as the stars were before.
//
// The stars show at once; the clouds and nebulae come when the worker's bake
// arrives (skyBakeClient.ts), one nebula at a time. A result for a sky or an
// exposure no longer wanted is dropped.

import {
  AdditiveBlending,
  BackSide,
  BoxGeometry,
  BufferAttribute,
  BufferGeometry,
  CubeTexture,
  CustomBlending,
  DataTexture,
  DoubleSide,
  Float32BufferAttribute,
  Group,
  LinearFilter,
  LinearMipmapLinearFilter,
  Mesh,
  OneFactor,
  OneMinusSrcAlphaFactor,
  Points,
  RGBAFormat,
  ShaderMaterial,
  SRGBColorSpace,
  UnsignedByteType,
  Vector3,
  ZeroFactor,
} from 'three';
import type { Vec3 } from '../skyBodies';
import type { NebulaTexture } from './nebulaTextures';
import { dustAt, type SkyExposure, type SkyFaces } from './skyClouds';
import { acquireSkyBaker, releaseSkyBaker, subscribeSkyBake } from './skyBakeClient';
import type { SkyPart } from './skyBake';
import { FIXED_SKY, type SkyLook } from './skyLook';
import { nebulaBasis, skyNebulae, type Nebula } from './skyNebulae';
import {
  skyBackdropFragment,
  skyBackdropVertex,
  skyNebulaFragment,
  skyNebulaVertex,
  starsFragment,
  starsVertex,
} from './shaders';
import { PHOTO_STAR_DENSITY, starField } from './stars';

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

function nebulaTextureOf(texture: NebulaTexture): DataTexture {
  const map = new DataTexture(texture.rgba, texture.size, texture.size, RGBAFormat, UnsignedByteType);
  map.colorSpace = SRGBColorSpace;
  map.magFilter = LinearFilter;
  map.minFilter = LinearMipmapLinearFilter;
  map.generateMipmaps = true;
  map.needsUpdate = true;
  return map;
}

/**
 * A nebula's patch: a quad tangent to the sky sphere at `dir`, turned by
 * `roll` (see nebulaBasis), `radius` wide each way in angle. Its corners are
 * directions in the system frame, so the group's matrix orients it like the
 * cube; the vertex shader sends them to the far plane. Row 0 of the texture
 * is the top: v runs down.
 *
 * The camera is at the sphere's centre, looking out: the triangles turn
 * clockwise in (right, up), since right x up points outward (along `dir`),
 * so that they are counter-clockwise, front faces, seen from the centre.
 */
export function nebulaGeometry(n: Nebula): BufferGeometry {
  const { right, up } = nebulaBasis(n);
  const half = Math.tan(n.radius);
  const corners: [number, number][] = [
    [-1, -1],
    [1, -1],
    [1, 1],
    [-1, 1],
  ];
  const positions: number[] = [];
  const uvs: number[] = [];
  for (const [x, y] of corners) {
    for (let k = 0; k < 3; k++) positions.push(n.dir[k]! + (x * right[k]! + y * up[k]!) * half);
    uvs.push((x + 1) / 2, (1 - y) / 2);
  }
  const geometry = new BufferGeometry();
  geometry.setAttribute('position', new Float32BufferAttribute(positions, 3));
  geometry.setAttribute('uv', new Float32BufferAttribute(uvs, 2));
  geometry.setIndex([0, 2, 1, 0, 3, 2]);
  return geometry;
}

/** One nebula of the catalogue: its patch, hidden until its texture is baked. */
interface NebulaPatch {
  mesh: Mesh<BufferGeometry, ShaderMaterial>;
  texture: DataTexture | null;
}

export class SkyBackdrop {
  /** The cube backdrop and the star points. */
  readonly group = new Group();
  readonly stars: Points<BufferGeometry, ShaderMaterial>;
  private readonly cube: Mesh<BoxGeometry, ShaderMaterial>;
  /** The nebulae's patches, in the order of `skyNebulae(look)`; empty unless the system's sky shows. */
  private readonly nebulae = new Group();
  private patches: NebulaPatch[] = [];
  private readonly renderOrder: number;
  private opacity = 1;
  private exposure: SkyExposure = 'photo';
  private texture: CubeTexture | null = null;
  private faces: SkyFaces | null = null;
  /** The look the system's sky draws now, null for the fixed sky and none. */
  private baked: SkyLook | null = null;
  /** Ends the bake being waited for. */
  private unsubscribe: (() => void) | null = null;
  /** Bumped at every new request: a part carrying an older one is stale. */
  private request = 0;
  private starKey = '';
  private choice: SkyChoice = 'off';
  private axes: [Vec3, Vec3, Vec3] | null = null;

  constructor(options: { hideHostStar: boolean; renderOrder: number }) {
    acquireSkyBaker();
    this.renderOrder = options.renderOrder;
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
    this.group.add(this.cube, this.nebulae, this.stars);
  }

  /** How many nebulae the sky draws (hidden or not): none but for the system's own sky. */
  get nebulaCount(): number {
    return this.patches.length;
  }

  /**
   * The rendering of the system's clouds and nebulae: as the eye sees them
   * or as a long exposure shows them. The catalogue stays; the textures are
   * swapped as the new bake arrives (those already baked at once).
   */
  setExposure(exposure: SkyExposure): void {
    if (exposure === this.exposure) return;
    this.exposure = exposure;
    if (this.baked) {
      this.buildStars(this.baked, true);
      this.bake(this.baked);
    }
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
    if (choice !== 'system') this.dropClouds();
    if (choice === 'off') return;
    const look = lookFor(choice, own);
    const clouds = choice === 'system';

    this.buildStars(look, clouds);

    if (clouds) {
      const wanted = lookKey(look);
      if (this.baked === null || lookKey(this.baked) !== wanted) {
        this.baked = look;
        this.faces = null;
        // The old sky's clouds must not linger under the new stars.
        this.cube.visible = false;
        this.setNebulae(look);
        this.bake(look);
      }
    }
  }

  /**
   * The star points of the look. The system's sky in a long exposure has
   * more stars than the naked eye sees; the fixed sky is today's in both.
   */
  private buildStars(look: SkyLook, clouds: boolean): void {
    const photo = clouds && look !== FIXED_SKY && this.exposure === 'photo';
    const key = `${clouds ? 'system' : 'fixed'}|${photo ? 'photo' : 'eye'}|${lookKey(look)}`;
    if (key === this.starKey) return;
    this.starKey = key;
    const field = starField(look, clouds ? (dir) => dustAt(look, dir) : undefined, photo ? PHOTO_STAR_DENSITY : 1);
    // A new geometry: replacing the attributes of the old one would leave their buffers on the GPU.
    const geometry = new BufferGeometry();
    geometry.setAttribute('position', new BufferAttribute(field.directions, 3));
    geometry.setAttribute('colour', new BufferAttribute(field.colours, 3));
    geometry.setAttribute('size', new BufferAttribute(field.sizes, 1));
    const old = this.stars.geometry;
    this.stars.geometry = geometry;
    old.dispose();
  }

  /** Asks for the look's parts in the exposure now wanted; the ones of an earlier request are dropped. */
  private bake(look: SkyLook): void {
    this.unsubscribe?.();
    const request = ++this.request;
    this.unsubscribe = subscribeSkyBake(look, this.exposure, (part) => {
      if (request === this.request) this.accept(part);
    });
  }

  private accept(part: SkyPart): void {
    if (part.part === 'cube') {
      if (part.faces === this.faces) return;
      this.faces = part.faces;
      this.texture?.dispose();
      this.texture = cubeOf(part.faces);
      this.cube.material.uniforms['uSky']!.value = this.texture;
      this.cube.visible = true;
      return;
    }
    const patch = this.patches[part.index];
    if (!patch) return;
    patch.texture?.dispose();
    patch.texture = nebulaTextureOf(part.texture);
    patch.mesh.material.uniforms['uMap']!.value = patch.texture;
    patch.mesh.visible = true;
  }

  /** One patch per nebula of the look, textureless and hidden until its bake arrives. */
  private setNebulae(look: SkyLook): void {
    this.clearNebulae();
    for (const n of skyNebulae(look)) {
      const dark = n.kind === 'dark';
      const material = new ShaderMaterial({
        vertexShader: skyNebulaVertex,
        fragmentShader: skyNebulaFragment,
        defines: dark ? { DARKEN: 1 } : {},
        uniforms: { uMap: { value: null }, uOpacity: { value: this.opacity } },
        transparent: true,
        depthWrite: false,
        blending: CustomBlending,
        // Premultiplied: a glow adds its colour, a dark nebula keeps (1 - alpha) of what is behind.
        blendSrc: dark ? ZeroFactor : OneFactor,
        blendDst: dark ? OneMinusSrcAlphaFactor : OneFactor,
        // A guard: the winding faces the centre (nebulaGeometry), but a culled patch draws nothing at all.
        side: DoubleSide,
      });
      const mesh = new Mesh(nebulaGeometry(n), material);
      mesh.frustumCulled = false;
      mesh.visible = false;
      // Between the cloud cube and the stars; dark patches after the glows, to dim them too.
      mesh.renderOrder = this.renderOrder - (dark ? 0.25 : 0.5);
      this.nebulae.add(mesh);
      this.patches.push({ mesh, texture: null });
    }
  }

  private clearNebulae(): void {
    for (const { mesh, texture } of this.patches) {
      texture?.dispose();
      mesh.geometry.dispose();
      mesh.material.dispose();
    }
    this.patches = [];
    this.nebulae.clear();
  }

  /** The fixed sky and none draw no clouds and no nebulae: ends any bake, frees the patches. */
  private dropClouds(): void {
    this.unsubscribe?.();
    this.unsubscribe = null;
    this.request++;
    this.baked = null;
    this.clearNebulae();
    this.cube.visible = false;
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
    this.opacity = o;
    for (const { mesh } of this.patches) mesh.material.uniforms['uOpacity']!.value = o;
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
    this.unsubscribe?.();
    this.unsubscribe = null;
    this.request++;
    this.baked = null;
    this.clearNebulae();
    releaseSkyBaker();
  }
}
