// Loads the decoded layers of a world through a Web Worker.
import type { WorldMetadata } from '../api/types/WorldMetadata';
import { worldFile } from '../api/client';
import type { WorldLayers } from './decode';
import type { WorkerRequest, WorkerResponse } from './worker';

export type { WorldLayers } from './decode';

/**
 * Fetches and decodes the layers of a world in a Web Worker. Aborting
 * `signal` terminates the worker (its downloads and decoding stop) and
 * rejects with the abort reason.
 */
export function loadLayers(id: string, meta: WorldMetadata, signal?: AbortSignal): Promise<WorldLayers> {
  if (signal?.aborted) {
    return Promise.reject(signal.reason as Error);
  }
  const worker = new Worker(new URL('./worker.ts', import.meta.url), { type: 'module' });
  return new Promise<WorldLayers>((resolve, reject) => {
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
        resolve(event.data.layers);
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
