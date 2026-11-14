// What baking one sky means, as a sequence of parts: the cloud cube first
// (the biggest, and what the eye sees most), then each nebula's texture in
// the catalogue's order. Pure: the worker (skyBake.worker.ts) and the
// synchronous fallback (skyBakeClient.ts) both walk the same sequence, and
// the tests call it directly.

import { bakeNebula, type NebulaTexture } from './nebulaTextures';
import { bakeSkyClouds, type SkyExposure, type SkyFaces } from './skyClouds';
import type { SkyLook } from './skyLook';
import { skyNebulae } from './skyNebulae';

/** One finished piece of a sky's bake. */
export type SkyPart =
  | { part: 'cube'; faces: SkyFaces }
  | { part: 'nebula'; index: number; texture: NebulaTexture };

/** The parts of the look's bake in an exposure; `index` is the nebula's place in `skyNebulae(look)`. */
export function* skyBakeParts(look: SkyLook, exposure: SkyExposure): Generator<SkyPart> {
  yield { part: 'cube', faces: bakeSkyClouds(look, exposure) };
  const list = skyNebulae(look);
  for (let index = 0; index < list.length; index++) {
    yield { part: 'nebula', index, texture: bakeNebula(list[index]!, exposure) };
  }
}
