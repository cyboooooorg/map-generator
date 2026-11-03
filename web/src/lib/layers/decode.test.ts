import { encode } from 'fast-png';
import { DataUtils } from 'three';
import { describe, expect, it } from 'vitest';
import type { WorldMetadata } from '../api/types/WorldMetadata';
import { decodeLayers, EXTRA, halfAt, quantizedAt, SURFACE } from './decode';

const width = 2;
const height = 1;

function png16(values: number[]): ArrayBuffer {
  const bytes = encode({ width, height, channels: 1, depth: 16, data: Uint16Array.from(values) });
  return bytes.slice().buffer;
}

function png8(values: number[]): ArrayBuffer {
  const bytes = encode({ width, height, channels: 1, depth: 8, data: Uint8Array.from(values) });
  return bytes.slice().buffer;
}

// Only the fields decodeLayers reads; the rest of the metadata is irrelevant here.
const meta = {
  format_version: 4,
  width,
  height,
  layers: {
    elevation: { file: 'elevation.png', encoding: 'gray16', min: -1, max: 1 },
    moisture: { file: 'moisture.png', encoding: 'gray16', min: -1, max: 1 },
    temperature: { file: 'temperature.png', encoding: 'gray16', min: 0, max: 1 },
    flow: { file: 'flow.png', encoding: 'gray16-log10', min: 0, max: 8 },
    biomes: { file: 'biomes.png', encoding: 'gray8-biome-id' },
  },
} as unknown as WorldMetadata;

describe('decodeLayers', () => {
  it('rescales 16-bit layers and undoes the log scale of flow', () => {
    const layers = decodeLayers(meta, {
      elevation: png16([0, 65535]),
      moisture: png16([32768, 0]),
      temperature: png16([65535, 0]),
      // log10(1 + 999) = 3 → 3 / 8 of the range.
      flow: png16([0, Math.round((3 / 8) * 65535)]),
      biomes: png8([1, 29]),
    });
    expect(quantizedAt(layers.elevation, 0)).toBe(-1);
    expect(quantizedAt(layers.elevation, 1)).toBe(1);
    expect(halfAt(layers.surface, 0, SURFACE.moisture)).toBeCloseTo(0, 4);
    expect(halfAt(layers.surface, 0, SURFACE.temperature)).toBe(1);
    expect(halfAt(layers.surface, 0, SURFACE.logFlow)).toBe(0);
    expect(10 ** halfAt(layers.surface, 1, SURFACE.logFlow) - 1).toBeCloseTo(999, -1);
    expect(Array.from(layers.biomes)).toEqual([1, 29]);
    // Packed for the GPU: 4 half floats per pixel, flow as log10(1 + km²).
    expect(layers.surface.length).toBe(8);
    expect(DataUtils.fromHalfFloat(layers.surface[0]!)).toBe(-1);
    expect(DataUtils.fromHalfFloat(layers.surface[7]!)).toBeCloseTo(3, 2);
  });

  it('converts the [0, 1] temperatures of format version 3 to °C', () => {
    const v3 = { ...meta, format_version: 3 } as unknown as WorldMetadata;
    const layers = decodeLayers(v3, {
      elevation: png16([0, 0]),
      moisture: png16([0, 0]),
      temperature: png16([65535, 0]),
      flow: png16([0, 0]),
      biomes: png8([1, 1]),
    });
    expect(halfAt(layers.surface, 0, SURFACE.temperature)).toBeCloseTo(27, 1);
    expect(halfAt(layers.surface, 1, SURFACE.temperature)).toBeCloseTo(-18, 1);
  });

  it('packs a linear flow layer as log10(1 + km²)', () => {
    const linear = {
      ...meta,
      layers: { ...meta.layers, flow: { file: 'flow.png', encoding: 'gray16', min: 0, max: 65535 } },
    } as unknown as WorldMetadata;
    const layers = decodeLayers(linear, {
      elevation: png16([0, 0]),
      moisture: png16([0, 0]),
      temperature: png16([0, 0]),
      flow: png16([0, 999]),
      biomes: png8([0, 0]),
    });
    expect(halfAt(layers.surface, 1, SURFACE.logFlow)).toBeCloseTo(3, 2);
  });

  it('leaves the extra layers out on version 1 worlds', () => {
    const layers = decodeLayers(meta, {
      elevation: png16([0, 0]),
      moisture: png16([0, 0]),
      temperature: png16([0, 0]),
      flow: png16([0, 0]),
      biomes: png8([0, 0]),
    });
    expect(layers.extra).toBeNull();
  });

  it('decodes the extra layers of version 2 worlds, floodplain defaulting to zero', () => {
    const v2 = {
      ...meta,
      layers: {
        ...meta.layers,
        volcanic_zone: { file: 'volcanic_zone.png', encoding: 'gray16', min: 0, max: 1 },
        coast_distance: { file: 'coast_distance.png', encoding: 'gray16', min: 0, max: 2000 },
        highland_offset: { file: 'highland_offset.png', encoding: 'gray16', min: -1, max: 1 },
      },
    } as unknown as WorldMetadata;
    const layers = decodeLayers(v2, {
      elevation: png16([0, 0]),
      moisture: png16([0, 0]),
      temperature: png16([0, 0]),
      flow: png16([0, 0]),
      biomes: png8([0, 0]),
      volcanic_zone: png16([0, 65535]),
      coast_distance: png16([65535, 0]),
      highland_offset: png16([0, 65535]),
    });
    expect(layers.extra).not.toBeNull();
    const packed = layers.extra!.packed;
    expect([0, 1].map((i) => halfAt(packed, i, EXTRA.volcanic))).toEqual([0, 1]);
    expect([0, 1].map((i) => halfAt(packed, i, EXTRA.coastKm))).toEqual([2000, 0]);
    expect(layers.extra!.coastMaxKm).toBe(2000);
    expect([0, 1].map((i) => halfAt(packed, i, EXTRA.floodplain))).toEqual([0, 0]);
    expect([0, 1].map((i) => halfAt(packed, i, EXTRA.highlandOffset))).toEqual([-1, 1]);
  });

  it('rejects a layer of the wrong size or depth', () => {
    const files = {
      elevation: png8([0, 0]),
      moisture: png16([0, 0]),
      temperature: png16([0, 0]),
      flow: png16([0, 0]),
      biomes: png8([0, 0]),
    };
    expect(() => decodeLayers(meta, files)).toThrow(/expected 1 of 16/);
  });
});
