// Web Worker: fetches and decodes the elevation and biomes of a world at an
// epoch (the time slider) off the main thread.
import type { WorldMetadata } from '../api/types/WorldMetadata';
import { decodeEpochFiles, type EpochFiles } from './decode';

export interface EpochRequest {
  elevation: string;
  biomes: string;
  meta: WorldMetadata;
}

export type EpochResponse = { ok: true; epoch: EpochFiles } | { ok: false; error: string };

async function fetchFile(url: string): Promise<ArrayBuffer> {
  const response = await fetch(url);
  if (!response.ok) {
    throw new Error(`${url}: ${response.status} ${response.statusText}`);
  }
  return response.arrayBuffer();
}

self.onmessage = async (event: MessageEvent<EpochRequest>) => {
  const { elevation, biomes, meta } = event.data;
  try {
    const [e, b] = await Promise.all([fetchFile(elevation), fetchFile(biomes)]);
    const epoch = decodeEpochFiles(meta, e, b);
    const message: EpochResponse = { ok: true, epoch };
    self.postMessage(message, { transfer: [epoch.elevation.buffer as ArrayBuffer, epoch.biomes.buffer as ArrayBuffer] });
  } catch (error) {
    const message: EpochResponse = { ok: false, error: String(error) };
    self.postMessage(message);
  }
};
