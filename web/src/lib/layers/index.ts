// Loads the decoded layers of a world through Web Workers.
import type { WorldMetadata } from '../api/types/WorldMetadata';
import { worldFile } from '../api/client';
import type { LoadProgress } from '../progress';
import type { WorldStats } from '../stats';
import { epochLayers, type Grey8, type WorldLayers } from './decode';
import type { EpochRequest, EpochResponse } from './epochWorker';
import type { GreyRequest, GreyResponse } from './greyWorker';
import type { WorkerRequest, WorkerResponse } from './worker';

export type { Grey8, WorldLayers } from './decode';

/** A world's decoded layers and the biome statistics computed from them. */
export interface DecodedWorld {
  layers: WorldLayers;
  stats: WorldStats;
}

type Answer<Response> = Extract<Response, { ok: true }>;

/**
 * Posts one request to a worker and resolves with its answer, terminating
 * the worker afterwards. Progress messages before the answer go to
 * `onProgress`. Aborting `signal` terminates the worker at once (its
 * downloads and decoding stop) and rejects with the abort reason.
 */
function runOnce<Request, Response extends { ok: true } | { ok: false; error: string } | { progress: LoadProgress }>(
  worker: Worker,
  request: Request,
  signal?: AbortSignal,
  onProgress?: (progress: LoadProgress) => void,
): Promise<Answer<Response>> {
  return new Promise<Answer<Response>>((resolve, reject) => {
    const finish = () => {
      worker.terminate();
      signal?.removeEventListener('abort', abort);
    };
    const abort = () => {
      finish();
      reject(signal?.reason as Error);
    };
    signal?.addEventListener('abort', abort, { once: true });
    worker.onmessage = (event: MessageEvent<Response>) => {
      const data = event.data;
      if ('progress' in data) {
        if (!signal?.aborted) onProgress?.(data.progress);
        return;
      }
      finish();
      if (data.ok) {
        resolve(data as Answer<Response>);
      } else {
        reject(new Error((data as { error: string }).error));
      }
    };
    worker.onerror = (event) => {
      finish();
      reject(new Error(event.message));
    };
    worker.postMessage(request);
  });
}

/**
 * Fetches and decodes the layers of a world at resolution `size`, and
 * computes its biome statistics, in a Web Worker. `onProgress` follows the downloads (bytes
 * of all the files) and then the decoding steps. Aborting `signal`
 * terminates the worker (its downloads and decoding stop) and rejects with
 * the abort reason.
 */
export async function loadLayers(
  id: string,
  size: string,
  meta: WorldMetadata,
  signal?: AbortSignal,
  onProgress?: (progress: LoadProgress) => void,
): Promise<DecodedWorld> {
  signal?.throwIfAborted();
  const worker = new Worker(new URL('./worker.ts', import.meta.url), { type: 'module' });
  const request: WorkerRequest = { base: worldFile(id, size, 'raw_data/'), meta };
  const { layers, stats } = await runOnce<WorkerRequest, WorkerResponse>(worker, request, signal, onProgress);
  return { layers, stats };
}

/**
 * Fetches and decodes an 8-bit grey image (such as `clouds.png`) in a Web
 * Worker, one byte per pixel, shrunk to fit `maxSize` pixels per side.
 * Aborting `signal` terminates the worker and rejects with the abort reason.
 */
export async function loadGrey8(url: string, maxSize: number, signal?: AbortSignal): Promise<Grey8> {
  signal?.throwIfAborted();
  const worker = new Worker(new URL('./greyWorker.ts', import.meta.url), { type: 'module' });
  const { image } = await runOnce<GreyRequest, GreyResponse>(worker, { url, maxSize }, signal);
  return image;
}

/**
 * The layers of a world at an epoch: its elevation and biomes, fetched from
 * `elevation` and `biomes` and decoded in a Web Worker, in place of those
 * of `present`. Aborting `signal` terminates the worker and rejects with
 * the abort reason.
 */
export async function loadEpochLayers(
  urls: { elevation: string; biomes: string },
  meta: WorldMetadata,
  present: WorldLayers,
  signal?: AbortSignal,
): Promise<WorldLayers> {
  signal?.throwIfAborted();
  const worker = new Worker(new URL('./epochWorker.ts', import.meta.url), { type: 'module' });
  const { epoch } = await runOnce<EpochRequest, EpochResponse>(worker, { ...urls, meta }, signal);
  return epochLayers(present, meta, epoch);
}
