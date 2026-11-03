// Web Worker: fetches and decodes the layers of one world off the main
// thread, and computes its biome statistics there too (a pass over every
// pixel). It reports its progress as it goes: bytes received per file
// (streamed), then each decoding step.
import { decodeLayers, EXTRA_FILES, type LayerFiles, type WorldLayers } from './decode';
import type { WorldMetadata } from '../api/types/WorldMetadata';
import { combineDownloads, type FileProgress, type LoadProgress, readWithProgress } from '../progress';
import { worldStats, type WorldStats } from '../stats';

export interface WorkerRequest {
  /** Base URL of the world's raw_data/ directory, ending with '/'. */
  base: string;
  meta: WorldMetadata;
}

export type WorkerResponse =
  | { ok: true; layers: WorldLayers; stats: WorldStats }
  | { ok: false; error: string }
  /** Sent any number of times before the answer. */
  | { progress: LoadProgress };

/** Least time between two download progress messages, ms. */
const PROGRESS_MS = 100;

function report(progress: LoadProgress): void {
  const message: WorkerResponse = { progress };
  self.postMessage(message);
}

self.onmessage = async (event: MessageEvent<WorkerRequest>) => {
  const { base, meta } = event.data;
  try {
    const l = meta.layers;
    // Every file to download, known up front so the total covers them all.
    const names = [l.elevation.file, l.moisture.file, l.temperature.file, l.flow.file, l.biomes.file];
    for (const name of EXTRA_FILES) {
      const info = l[name];
      if (info) names.push(info.file);
    }
    const downloads = new Map<string, FileProgress>(names.map((name) => [name, { received: 0, total: null }]));
    let reported = 0;
    const fetchFile = async (file: string): Promise<ArrayBuffer> => {
      const url = base + file;
      // Revalidated: a world regenerated under the same id keeps its file URLs.
      const response = await fetch(url, { cache: 'no-cache' });
      if (!response.ok) {
        throw new Error(`${url}: ${response.status} ${response.statusText}`);
      }
      return readWithProgress(response, (progress) => {
        downloads.set(file, progress);
        const now = performance.now();
        if (now - reported >= PROGRESS_MS || progress.received === progress.total) {
          reported = now;
          report(combineDownloads(downloads, file));
        }
      });
    };
    const buffers = new Map(await Promise.all(names.map(async (file) => [file, await fetchFile(file)] as const)));
    const buffer = (file: string) => buffers.get(file)!;
    const files: LayerFiles = {
      elevation: buffer(l.elevation.file),
      moisture: buffer(l.moisture.file),
      temperature: buffer(l.temperature.file),
      flow: buffer(l.flow.file),
      biomes: buffer(l.biomes.file),
    };
    // Layers of later format versions: fetched when listed, skipped otherwise.
    for (const name of EXTRA_FILES) {
      const info = l[name];
      if (info) {
        files[name] = buffer(info.file);
      }
    }
    // Each file, then the statistics.
    const steps = names.length + 1;
    let step = 0;
    const layers = decodeLayers(meta, files, (file) => {
      report({ stage: 'decode', label: file, done: step, total: steps });
      step += 1;
    });
    // The statistics step has no file: the bar names it.
    report({ stage: 'decode', label: '', done: step, total: steps });
    const stats = worldStats(layers.biomes, layers.width, layers.height, meta.biomes, meta.circumference_km);
    const response: WorkerResponse = { ok: true, layers, stats };
    const transfer: ArrayBufferLike[] = [layers.elevation.data.buffer, layers.biomes.buffer, layers.surface.buffer];
    if (layers.extra) {
      transfer.push(layers.extra.packed.buffer);
    }
    self.postMessage(response, { transfer: transfer as Transferable[] });
  } catch (error) {
    const response: WorkerResponse = { ok: false, error: String(error) };
    self.postMessage(response);
  }
};
