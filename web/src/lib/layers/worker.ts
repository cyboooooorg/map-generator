// Web Worker: fetches and decodes the layers of one world off the main
// thread, and computes its biome statistics there too (a pass over every
// pixel).
import { decodeLayers, EXTRA_FILES, type LayerFiles, type WorldLayers } from './decode';
import type { WorldMetadata } from '../api/types/WorldMetadata';
import { worldStats, type WorldStats } from '../stats';

export interface WorkerRequest {
  /** Base URL of the world's raw_data/ directory, ending with '/'. */
  base: string;
  meta: WorldMetadata;
}

export type WorkerResponse =
  | { ok: true; layers: WorldLayers; stats: WorldStats }
  | { ok: false; error: string };

async function fetchFile(url: string): Promise<ArrayBuffer> {
  // Revalidated: a world regenerated under the same id keeps its file URLs.
  const response = await fetch(url, { cache: 'no-cache' });
  if (!response.ok) {
    throw new Error(`${url}: ${response.status} ${response.statusText}`);
  }
  return response.arrayBuffer();
}

self.onmessage = async (event: MessageEvent<WorkerRequest>) => {
  const { base, meta } = event.data;
  try {
    const l = meta.layers;
    const [elevation, moisture, temperature, flow, biomes] = await Promise.all(
      [l.elevation.file, l.moisture.file, l.temperature.file, l.flow.file, l.biomes.file].map((file) =>
        fetchFile(base + file),
      ),
    );
    const files: LayerFiles = {
      elevation: elevation!,
      moisture: moisture!,
      temperature: temperature!,
      flow: flow!,
      biomes: biomes!,
    };
    // Layers of later format versions: fetched when listed, skipped otherwise.
    await Promise.all(
      EXTRA_FILES.map(async (name) => {
        const info = l[name];
        if (info) {
          files[name] = await fetchFile(base + info.file);
        }
      }),
    );
    const layers = decodeLayers(meta, files);
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
