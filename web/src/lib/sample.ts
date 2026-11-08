// Every value of a world at one point, for the point information panel and
// the readout under the pointer.
import type { BiomeInfo } from './api/types/BiomeInfo';
import type { HexMap } from './api/types/HexMap';
import type { WorldMetadata } from './api/types/WorldMetadata';
import { lonLatToPixel, type LonLat } from './geo';
import { hexAt, type HexCell } from './hex';
import type { WorldLayers } from './layers';
import { EXTRA, halfAt, quantizedAt, SURFACE } from './layers/decode';
import type { RiverHit, RiverIndex } from './rivers';

export interface PointSample {
  x: number;
  y: number;
  biome: BiomeInfo | undefined;
  /** Elevation relative to the coastline: positive is dry land. */
  relief: number;
  temperature: number;
  moisture: number;
  flowKm2: number;
  /** The extra layers; null while they are not loaded. */
  coastKm: number | null;
  coastCapped: boolean;
  volcanic: number | null;
  floodplain: number | null;
  highlandOffset: number | null;
}

/** Elevation of the coastline: the shifted sea level plus the preset waterline. */
export function coastline(meta: WorldMetadata): number {
  return meta.sea_level + meta.preset.rules.waterline;
}

export function samplePoint(point: LonLat, meta: WorldMetadata, layers: WorldLayers): PointSample {
  const [x, y] = lonLatToPixel(point, layers.width, layers.height);
  const i = y * layers.width + x;
  const extra = layers.extra;
  const coastKm = extra ? halfAt(extra.packed, i, EXTRA.coastKm) : null;
  return {
    x,
    y,
    biome: meta.biomes[layers.biomes[i] ?? 0],
    relief: quantizedAt(layers.elevation, i) - coastline(meta),
    temperature: halfAt(layers.surface, i, SURFACE.temperature),
    moisture: halfAt(layers.surface, i, SURFACE.moisture),
    flowKm2: Math.max(0, 10 ** halfAt(layers.surface, i, SURFACE.logFlow) - 1),
    coastKm,
    coastCapped: extra !== null && coastKm !== null && coastKm >= extra.coastMaxKm * 0.999,
    volcanic: extra ? halfAt(extra.packed, i, EXTRA.volcanic) : null,
    // Version 2 worlds have no floodplain layer: decoded as zeros, shown as missing.
    floodplain: extra && meta.layers.floodplain ? halfAt(extra.packed, i, EXTRA.floodplain) : null,
    highlandOffset: extra ? halfAt(extra.packed, i, EXTRA.highlandOffset) : null,
  };
}

/** The river at a point, searching a few map pixels around it. */
export function riverAt(point: LonLat, rivers: RiverIndex, mapWidth: number): RiverHit | null {
  return rivers.nearest(point, (2.5 * 360) / mapWidth);
}

export interface HexSample {
  cell: HexCell;
  biome: BiomeInfo | undefined;
  elevation: number;
  moisture: number;
  temperature: number;
  maxFlowKm2: number;
  river: boolean;
}

export function sampleHex(point: LonLat, map: HexMap): HexSample {
  const cell = hexAt(map, point);
  const c = map.cells;
  return {
    cell,
    biome: map.biomes[c.biome[cell.index] ?? 0],
    elevation: c.elevation[cell.index] ?? 0,
    moisture: c.moisture[cell.index] ?? 0,
    temperature: c.temperature[cell.index] ?? 0,
    maxFlowKm2: c.max_flow_km2[cell.index] ?? 0,
    river: c.river[cell.index] ?? false,
  };
}
