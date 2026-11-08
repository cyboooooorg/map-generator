// Decodes the raw_data/ layer PNGs into compact typed arrays. Runs in a Web
// Worker (see worker.ts) but has no browser dependency, so it is unit-tested
// directly.
//
// Browsers decode PNGs to 8 bits per channel: the 16-bit layers need this
// JavaScript decoder to keep their precision.
//
// Memory: the layers stay packed as the GPU textures need them (half
// floats), and values are converted on read (see ../sample.ts). Only the
// elevation keeps its exact 16-bit samples, for the readout of the height
// above the coastline. That is 11 bytes per pixel on the main thread
// (19 with the layers of format version 2), instead of 4-byte floats per
// layer next to the GPU copies.
import { temperatureSpan } from '../temperatureRange';
import { decodePng } from './png';
import { DataUtils } from 'three';
import type { ScalarLayerInfo } from '../api/types/ScalarLayerInfo';
import type { WorldMetadata } from '../api/types/WorldMetadata';
import { downsample, fitFactor } from '../viewer/textures';

/** A 16-bit layer as stored: pixel `p` is `min + data[p] × scale`. */
export interface Quantized {
  data: Uint16Array;
  min: number;
  scale: number;
}

/** Every pixel value of a world, row-major, row 0 = north. */
export interface WorldLayers {
  width: number;
  height: number;
  /** Terrain height in [-1, 1], before the sea-level shift, exact. */
  elevation: Quantized;
  /** Biome id of each pixel (index into WorldMetadata.biomes). */
  biomes: Uint8Array;
  /**
   * The four scalar layers packed for the GPU as RGBA half floats:
   * elevation, temperature in °C, moisture in [-1, 1],
   * log10(1 + drainage area in km²).
   */
  surface: Uint16Array;
  /** The coldest and hottest temperatures of the world, °C (see `temperatureSpan`). */
  temperatureSpan: { min: number; max: number };
  /**
   * Layers of format version 2 and later, null on older worlds (or when a
   * world misses one of them).
   */
  extra: ExtraLayers | null;
}

/** The biome-selection layers exported since `raw_data` format version 2. */
export interface ExtraLayers {
  /**
   * RGBA half floats: volcanic zone in [0, 1], distance to open water in km
   * (capped at `coastMaxKm`), floodplain weight in [0, 1] (format version 3;
   * zeros before), local shift of the highland line in [-1, 1].
   */
  packed: Uint16Array;
  coastMaxKm: number;
}

/** Channels of WorldLayers.surface. */
export const SURFACE = { elevation: 0, temperature: 1, moisture: 2, logFlow: 3 } as const;
/** Channels of ExtraLayers.packed. */
export const EXTRA = { volcanic: 0, coastKm: 1, floodplain: 2, highlandOffset: 3 } as const;

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
  const png = decodePng(file);
  if (png.width !== width || png.height !== height) {
    throw new Error(`layer is ${png.width}×${png.height}, expected ${width}×${height}`);
  }
  if (png.channels !== 1 || png.depth !== depth) {
    throw new Error(`layer has ${png.channels} channel(s) of ${png.depth} bits, expected 1 of ${depth}`);
  }
  return png.data;
}

function decode16(file: ArrayBuffer, width: number, height: number): Uint16Array {
  const pixels = decodeGrey(file, width, height, 16);
  return pixels instanceof Uint16Array ? pixels : Uint16Array.from(pixels);
}

/**
 * The array itself when it owns its whole buffer, a copy otherwise, so that
 * transferring its buffer to the main thread moves nothing else.
 */
function owned<T extends Uint8Array | Uint16Array>(array: T, copy: (a: T) => T): T {
  return array.byteOffset === 0 && array.buffer.byteLength === array.byteLength ? array : copy(array);
}

/**
 * Writes a 16-bit layer into one channel of an RGBA half-float pack. `map`
 * turns the decoded value into the packed one. A 16-bit layer has at most
 * 65 536 values: they are converted once, through a lookup table.
 */
function packChannel(
  packed: Uint16Array,
  channel: number,
  pixels: Uint16Array,
  info: ScalarLayerInfo,
  map: (value: number) => number = (v) => v,
): void {
  const scale = (info.max - info.min) / 65535;
  const table = new Uint16Array(65536);
  for (let p = 0; p < 65536; p += 1) {
    table[p] = DataUtils.toHalfFloat(map(info.min + p * scale));
  }
  for (let i = 0; i < pixels.length; i += 1) {
    packed[i * 4 + channel] = table[pixels[i]!]!;
  }
}

/** log10(1 + flow) of a flow layer, whatever its encoding. */
function logFlow(info: ScalarLayerInfo): (value: number) => number {
  return info.encoding === 'gray16-log10' ? (v) => v : (v) => Math.log10(1 + v);
}

function decodeExtra(meta: WorldMetadata, files: LayerFiles, step: (file: string) => void): ExtraLayers | null {
  const { width, height, layers } = meta;
  const volcanicInfo = layers.volcanic_zone;
  const coastInfo = layers.coast_distance;
  const highlandInfo = layers.highland_offset;
  if (!volcanicInfo || !coastInfo || !highlandInfo || !files.volcanic_zone || !files.coast_distance || !files.highland_offset) {
    return null;
  }
  // Zero-filled: half-float 0 is 0, a world without floodplain.
  const packed = new Uint16Array(width * height * 4);
  step(volcanicInfo.file);
  packChannel(packed, EXTRA.volcanic, decode16(files.volcanic_zone, width, height), volcanicInfo);
  step(coastInfo.file);
  packChannel(packed, EXTRA.coastKm, decode16(files.coast_distance, width, height), coastInfo);
  step(highlandInfo.file);
  packChannel(packed, EXTRA.highlandOffset, decode16(files.highland_offset, width, height), highlandInfo);
  if (layers.floodplain && files.floodplain) {
    step(layers.floodplain.file);
    packChannel(packed, EXTRA.floodplain, decode16(files.floodplain, width, height), layers.floodplain);
  }
  return { packed, coastMaxKm: coastInfo.max };
}

/**
 * Decodes the layer files of a world. `step` is called with the name of
 * each file before it is decoded (for a progress report).
 */
export function decodeLayers(
  meta: WorldMetadata,
  files: LayerFiles,
  step: (file: string) => void = () => {},
): WorldLayers {
  const { width, height, layers } = meta;
  step(layers.biomes.file);
  const biomes = decodeGrey(files.biomes, width, height, 8);
  step(layers.elevation.file);
  const elevation = decode16(files.elevation, width, height);
  const surface = new Uint16Array(width * height * 4);
  packChannel(surface, SURFACE.elevation, elevation, layers.elevation);
  step(layers.temperature.file);
  const temperature = decode16(files.temperature, width, height);
  packChannel(surface, SURFACE.temperature, temperature, layers.temperature);
  step(layers.moisture.file);
  packChannel(surface, SURFACE.moisture, decode16(files.moisture, width, height), layers.moisture);
  step(layers.flow.file);
  packChannel(surface, SURFACE.logFlow, decode16(files.flow, width, height), layers.flow, logFlow(layers.flow));
  return {
    width,
    height,
    elevation: {
      data: owned(elevation, (a) => a.slice()),
      min: layers.elevation.min,
      scale: (layers.elevation.max - layers.elevation.min) / 65535,
    },
    biomes: biomes instanceof Uint8Array ? owned(biomes, (a) => a.slice()) : Uint8Array.from(biomes),
    surface,
    temperatureSpan: temperatureSpan(temperature, layers.temperature),
    extra: decodeExtra(meta, files, step),
  };
}

/** The elevation and biomes of a world at an epoch, as served (`GET /api/systems/{id}/bodies/{body}/epoch/{ma}/...`). */
export interface EpochFiles {
  /** 16-bit, encoded as `raw_data/elevation.png`. */
  elevation: Uint16Array;
  biomes: Uint8Array;
}

/** Decodes the elevation and biome PNGs of a world at an epoch, of its base size. */
export function decodeEpochFiles(meta: WorldMetadata, elevation: ArrayBuffer, biomes: ArrayBuffer): EpochFiles {
  const ids = decodeGrey(biomes, meta.width, meta.height, 8);
  return {
    elevation: owned(decode16(elevation, meta.width, meta.height), (a) => a.slice()),
    biomes: ids instanceof Uint8Array ? owned(ids, (a) => a.slice()) : Uint8Array.from(ids),
  };
}

/**
 * The layers of a world at an epoch: its elevation and biomes in place of
 * the present ones (`present` is left as it is); the other layers are the
 * present's, which the epoch does not change (temperature aside, which
 * follows the relief by a fraction of a degree: kept).
 */
export function epochLayers(present: WorldLayers, meta: WorldMetadata, epoch: EpochFiles): WorldLayers {
  const surface = present.surface.slice();
  packChannel(surface, SURFACE.elevation, epoch.elevation, meta.layers.elevation);
  return {
    ...present,
    elevation: { ...present.elevation, data: epoch.elevation },
    biomes: epoch.biomes,
    surface,
  };
}

/** The value of one channel of a half-float pack at pixel `i`. */
export function halfAt(packed: Uint16Array, i: number, channel: number): number {
  return DataUtils.fromHalfFloat(packed[i * 4 + channel] ?? 0);
}

/** The value of a quantized layer at pixel `i`. */
export function quantizedAt(layer: Quantized, i: number): number {
  return layer.min + (layer.data[i] ?? 0) * layer.scale;
}

/** A single-channel 8-bit image, row-major, row 0 = north. */
export interface Grey8 {
  data: Uint8Array;
  width: number;
  height: number;
}

/**
 * Decodes an 8-bit grey PNG (such as `clouds.png`), shrunk to fit `maxSize`
 * pixels per side. Only the first channel of an image with more is kept.
 */
export function decodeGrey8(file: ArrayBuffer, maxSize: number): Grey8 {
  const png = decodePng(file);
  if (png.depth !== 8) {
    throw new Error(`image has ${png.depth}-bit channels, expected 8`);
  }
  const pixels = png.data as Uint8Array;
  let grey: Uint8Array;
  if (png.channels === 1) {
    grey = owned(pixels, (a) => a.slice());
  } else {
    grey = new Uint8Array(png.width * png.height);
    for (let i = 0; i < grey.length; i += 1) {
      grey[i] = pixels[i * png.channels]!;
    }
  }
  const factor = fitFactor(png.width, png.height, maxSize);
  return downsample(grey, png.width, png.height, 1, factor);
}
