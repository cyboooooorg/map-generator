// Decodes the raw_data/ layer PNGs into typed arrays. Runs in a Web Worker
// (see worker.ts) but has no browser dependency, so it is unit-tested directly.
//
// Browsers decode PNGs to 8 bits per channel: the 16-bit layers need this
// JavaScript decoder to keep their precision.
import { decode } from 'fast-png';
import type { ScalarLayerInfo } from '../api/types/ScalarLayerInfo';
import type { WorldMetadata } from '../api/types/WorldMetadata';

/** Every pixel value of a world, row-major, row 0 = north. */
export interface WorldLayers {
  width: number;
  height: number;
  /** Terrain height in [-1, 1], before the sea-level shift. */
  elevation: Float32Array;
  /** Moisture in [-1, 1], planet offset included. */
  moisture: Float32Array;
  /** Temperature in [0, 1], planet offset included. */
  temperature: Float32Array;
  /** Drainage area in km² flowing through the pixel. */
  flowKm2: Float32Array;
  /** Biome id of each pixel (index into WorldMetadata.biomes). */
  biomes: Uint8Array;
}

/** The PNG files of a world, as fetched. */
export interface LayerFiles {
  elevation: ArrayBuffer;
  moisture: ArrayBuffer;
  temperature: ArrayBuffer;
  flow: ArrayBuffer;
  biomes: ArrayBuffer;
}

function decodeGrey(file: ArrayBuffer, width: number, height: number, depth: 8 | 16) {
  const png = decode(file);
  if (png.width !== width || png.height !== height) {
    throw new Error(`layer is ${png.width}×${png.height}, expected ${width}×${height}`);
  }
  if (png.channels !== 1 || png.depth !== depth) {
    throw new Error(`layer has ${png.channels} channel(s) of ${png.depth} bits, expected 1 of ${depth}`);
  }
  return png.data;
}

function decodeScalar(file: ArrayBuffer, info: ScalarLayerInfo, width: number, height: number): Float32Array {
  const pixels = decodeGrey(file, width, height, 16);
  const scale = (info.max - info.min) / 65535;
  const values = new Float32Array(pixels.length);
  const log = info.encoding === 'gray16-log10';
  for (let i = 0; i < pixels.length; i += 1) {
    const v = info.min + (pixels[i] ?? 0) * scale;
    values[i] = log ? 10 ** v - 1 : v;
  }
  return values;
}

export function decodeLayers(meta: WorldMetadata, files: LayerFiles): WorldLayers {
  const { width, height, layers } = meta;
  const biomes = decodeGrey(files.biomes, width, height, 8);
  return {
    width,
    height,
    elevation: decodeScalar(files.elevation, layers.elevation, width, height),
    moisture: decodeScalar(files.moisture, layers.moisture, width, height),
    temperature: decodeScalar(files.temperature, layers.temperature, width, height),
    flowKm2: decodeScalar(files.flow, layers.flow, width, height),
    biomes: Uint8Array.from(biomes),
  };
}
