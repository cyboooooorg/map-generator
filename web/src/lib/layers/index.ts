// Loads the decoded layers of a world through a Web Worker.
import type { WorldMetadata } from '../api/types/WorldMetadata';
import { worldFile } from '../api/client';
import type { WorldStats } from '../stats';
import type { WorldLayers } from './decode';
import type { WorkerRequest, WorkerResponse } from './worker';

export type { WorldLayers } from './decode';

/** A world's decoded layers and the biome statistics computed from them. */
export interface DecodedWorld {
  layers: WorldLayers;
  stats: WorldStats;
}

/**
 * Fetches and decodes the layers of a world, and computes its biome
 * statistics, in a Web Worker. Aborting
 * `signal` terminates the worker (its downloads and decoding stop) and
 * rejects with the abort reason.
 */
export function loadLayers(id: string, meta: WorldMetadata, signal?: AbortSignal): Promise<DecodedWorld> {
  if (signal?.aborted) {
    return Promise.reject(signal.reason as Error);
  }
  const worker = new Worker(new URL('./worker.ts', import.meta.url), { type: 'module' });
  return new Promise<DecodedWorld>((resolve, reject) => {
    const finish = () => {
      worker.terminate();
      signal?.removeEventListener('abort', abort);
    };
    const abort = () => {
      finish();
      reject(signal?.reason as Error);
    };
    signal?.addEventListener('abort', abort, { once: true });
    worker.onmessage = (event: MessageEvent<WorkerResponse>) => {
      finish();
      if (event.data.ok) {
        resolve({ layers: event.data.layers, stats: event.data.stats });
      } else {
        reject(new Error(event.data.error));
      }
    };
    worker.onerror = (event) => {
      finish();
      reject(new Error(event.message));
    };
    const request: WorkerRequest = { base: worldFile(id, 'raw_data/'), meta };
    worker.postMessage(request);
  });
}
