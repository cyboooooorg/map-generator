// Loads the decoded layers of a world through a Web Worker.
import type { WorldMetadata } from '../api/types/WorldMetadata';
import { worldFile } from '../api/client';
import type { WorldLayers } from './decode';
import type { WorkerRequest, WorkerResponse } from './worker';

export type { WorldLayers } from './decode';

export function loadLayers(id: string, meta: WorldMetadata): Promise<WorldLayers> {
  const worker = new Worker(new URL('./worker.ts', import.meta.url), { type: 'module' });
  return new Promise<WorldLayers>((resolve, reject) => {
    worker.onmessage = (event: MessageEvent<WorkerResponse>) => {
      worker.terminate();
      if (event.data.ok) {
        resolve(event.data.layers);
      } else {
        reject(new Error(event.data.error));
      }
    };
    worker.onerror = (event) => {
      worker.terminate();
      reject(new Error(event.message));
    };
    const request: WorkerRequest = { base: worldFile(id, 'raw_data/'), meta };
    worker.postMessage(request);
  });
}
