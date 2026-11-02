// Web Worker: fetches and decodes the layers of one world off the main thread.
import { decodeLayers, type LayerFiles } from './decode';
import type { WorldMetadata } from '../api/types/WorldMetadata';

export interface WorkerRequest {
  /** Base URL of the world's raw_data/ directory, ending with '/'. */
  base: string;
  meta: WorldMetadata;
}

export type WorkerResponse =
  | { ok: true; layers: import('./decode').WorldLayers }
  | { ok: false; error: string };

async function fetchFile(url: string): Promise<ArrayBuffer> {
  const response = await fetch(url);
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
    const layers = decodeLayers(meta, files);
    const response: WorkerResponse = { ok: true, layers };
    self.postMessage(response, {
      transfer: [
        layers.elevation.buffer,
        layers.moisture.buffer,
        layers.temperature.buffer,
        layers.flowKm2.buffer,
        layers.biomes.buffer,
      ],
    });
  } catch (error) {
    const response: WorkerResponse = { ok: false, error: String(error) };
    self.postMessage(response);
  }
};
