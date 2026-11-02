// Decodes the raw_data/ layer PNGs into typed arrays. Runs in a Web Worker
// (see worker.ts) but has no browser dependency, so it is unit-tested directly.
//
// Browsers decode PNGs to 8 bits per channel: the 16-bit layers need this
// JavaScript decoder to keep their precision.
import { decode } from 'fast-png';
import { DataUtils } from 'three';
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
  /**
   * The four scalar layers packed for the GPU as RGBA half floats:
   * elevation, temperature, moisture, log10(1 + flow km²).
   */
  surface: Uint16Array;
  /**
   * Layers of format version 2 and later, null on older worlds (or when a
   * world misses one of them).
   */
  extra: ExtraLayers | null;
}

/** The biome-selection layers exported since `raw_data` format version 2. */
export interface ExtraLayers {
  /** Volcanic zone in [0, 1], as biome selection sees it. */
  volcanic: Float32Array;
  /** Distance to open water in km, capped at `coastMaxKm`. */
  coastKm: Float32Array;
  coastMaxKm: number;
  /** Local shift of the highland line, in [-1, 1]. */
  highlandOffset: Float32Array;
  /** Floodplain weight of the river valleys in [0, 1] (format version 3; zeros before). */
  floodplain: Float32Array;
  /** The four packed for the GPU as RGBA half floats, in the order above. */
  packed: Uint16Array;
}

/** The PNG files of a world, as fetched. */
export interface LayerFiles {
  elevation: ArrayBuffer;
  moisture: ArrayBuffer;
  temperature: ArrayBuffer;
  flow: ArrayBuffer;
  biomes: ArrayBuffer;
  volcanic_zone?: ArrayBuffer;
  coast_distance?: ArrayBuffer;
  highland_offset?: ArrayBuffer;
  floodplain?: ArrayBuffer;
}

/** Optional layer files the worker fetches when the metadata lists them. */
export const EXTRA_FILES = ['volcanic_zone', 'coast_distance', 'highland_offset', 'floodplain'] as const;

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

/** Packs the scalar layers as RGBA half floats (see WorldLayers.surface). */
export function packSurface(
  elevation: Float32Array,
  temperature: Float32Array,
  moisture: Float32Array,
  flowKm2: Float32Array,
): Uint16Array {
  const surface = new Uint16Array(elevation.length * 4);
  for (let i = 0; i < elevation.length; i += 1) {
    surface[i * 4] = DataUtils.toHalfFloat(elevation[i] ?? 0);
    surface[i * 4 + 1] = DataUtils.toHalfFloat(temperature[i] ?? 0);
    surface[i * 4 + 2] = DataUtils.toHalfFloat(moisture[i] ?? 0);
    surface[i * 4 + 3] = DataUtils.toHalfFloat(Math.log10(1 + (flowKm2[i] ?? 0)));
  }
  return surface;
}

function packHalf(a: Float32Array, b: Float32Array, c: Float32Array, d: Float32Array): Uint16Array {
  const packed = new Uint16Array(a.length * 4);
  for (let i = 0; i < a.length; i += 1) {
    packed[i * 4] = DataUtils.toHalfFloat(a[i] ?? 0);
    packed[i * 4 + 1] = DataUtils.toHalfFloat(b[i] ?? 0);
    packed[i * 4 + 2] = DataUtils.toHalfFloat(c[i] ?? 0);
    packed[i * 4 + 3] = DataUtils.toHalfFloat(d[i] ?? 0);
  }
  return packed;
}

function decodeExtra(meta: WorldMetadata, files: LayerFiles): ExtraLayers | null {
  const { width, height, layers } = meta;
  const volcanicInfo = layers.volcanic_zone;
  const coastInfo = layers.coast_distance;
  const highlandInfo = layers.highland_offset;
  if (!volcanicInfo || !coastInfo || !highlandInfo || !files.volcanic_zone || !files.coast_distance || !files.highland_offset) {
    return null;
  }
  const volcanic = decodeScalar(files.volcanic_zone, volcanicInfo, width, height);
  const coastKm = decodeScalar(files.coast_distance, coastInfo, width, height);
  const highlandOffset = decodeScalar(files.highland_offset, highlandInfo, width, height);
  const floodplain =
    layers.floodplain && files.floodplain
      ? decodeScalar(files.floodplain, layers.floodplain, width, height)
      : new Float32Array(width * height);
  return {
    volcanic,
    coastKm,
    coastMaxKm: coastInfo.max,
    highlandOffset,
    floodplain,
    packed: packHalf(volcanic, coastKm, floodplain, highlandOffset),
  };
}

export function decodeLayers(meta: WorldMetadata, files: LayerFiles): WorldLayers {
  const { width, height, layers } = meta;
  const biomes = decodeGrey(files.biomes, width, height, 8);
  const elevation = decodeScalar(files.elevation, layers.elevation, width, height);
  const moisture = decodeScalar(files.moisture, layers.moisture, width, height);
  const temperature = decodeScalar(files.temperature, layers.temperature, width, height);
  const flowKm2 = decodeScalar(files.flow, layers.flow, width, height);
  return {
    width,
    height,
    elevation,
    moisture,
    temperature,
    flowKm2,
    biomes: Uint8Array.from(biomes),
    surface: packSurface(elevation, temperature, moisture, flowKm2),
    extra: decodeExtra(meta, files),
  };
}
