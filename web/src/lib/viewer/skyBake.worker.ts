// Bakes a sky off the main thread: about 0.75 s for a system's cloud cube
// and its nebulae, far over a frame. A job is `{ id, look, exposure }`; the
// parts come back as they finish (see skyBake.ts) with their buffers
// transferred, then `{ id, part: 'done' }`. A `{ cancel: id }` message
// stops a job between two parts: the worker yields to its event queue after
// each so that the message can be heard.

import { skyBakeParts } from './skyBake';
import type { SkyExposure } from './skyClouds';
import type { SkyLook } from './skyLook';

export type BakeRequest = { id: number; look: SkyLook; exposure: SkyExposure } | { cancel: number };

interface Scope {
  onmessage: ((event: { data: BakeRequest }) => void) | null;
  postMessage(message: unknown, transfer: ArrayBuffer[]): void;
}
const scope = self as unknown as Scope;

const cancelled = new Set<number>();

async function run(id: number, look: SkyLook, exposure: SkyExposure): Promise<void> {
  for (const part of skyBakeParts(look, exposure)) {
    // Copies are posted: the bake functions memoise their results and a transferred buffer is detached.
    if (part.part === 'cube') {
      const faces = { size: part.faces.size, faces: part.faces.faces.map((f) => f.slice()) };
      scope.postMessage({ id, part: 'cube', faces }, faces.faces.map((f) => f.buffer as ArrayBuffer));
    } else {
      const texture = { ...part.texture, rgba: part.texture.rgba.slice() };
      scope.postMessage({ id, part: 'nebula', index: part.index, texture }, [texture.rgba.buffer as ArrayBuffer]);
    }
    await new Promise((resolve) => setTimeout(resolve, 0));
    if (cancelled.delete(id)) return;
  }
  cancelled.delete(id);
  scope.postMessage({ id, part: 'done' }, []);
}

scope.onmessage = ({ data }) => {
  if ('cancel' in data) cancelled.add(data.cancel);
  else void run(data.id, data.look, data.exposure);
};
