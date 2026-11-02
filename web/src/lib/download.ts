// Downloads of a world's files, including a ZIP of its raw data built in the
// browser (the server only serves the files one by one).
import { zipSync, type Zippable } from 'fflate';
import { worldFile } from './api/client';
import type { WorldMetadata } from './api/types/WorldMetadata';
import { saveBlob } from './format';

/** Files of `raw_data/` listed by a world's metadata. */
export function rawDataFiles(meta: WorldMetadata): string[] {
  const files = new Set<string>(['world.json']);
  for (const layer of Object.values(meta.layers)) {
    if (layer && typeof layer === 'object' && 'file' in layer) {
      files.add(layer.file);
    }
  }
  if (meta.rivers) {
    files.add(meta.rivers);
  }
  return [...files];
}

/** Fetches every raw data file of a world and saves them as `<id>-raw_data.zip`. */
export async function downloadRawData(id: string, meta: WorldMetadata): Promise<void> {
  const entries = await Promise.all(
    rawDataFiles(meta).map(async (file): Promise<[string, Uint8Array]> => {
      const response = await fetch(worldFile(id, `raw_data/${file}`));
      if (!response.ok) {
        throw new Error(`raw_data/${file}: ${response.status} ${response.statusText}`);
      }
      return [file, new Uint8Array(await response.arrayBuffer())];
    }),
  );
  const tree: Zippable = {};
  for (const [file, data] of entries) {
    // PNGs are compressed already: store them as they are.
    tree[file] = [data, { level: file.endsWith('.png') ? 0 : 6 }];
  }
  const zip = zipSync({ [`${id}/raw_data`]: tree });
  saveBlob(new Blob([zip.buffer as ArrayBuffer], { type: 'application/zip' }), `${id}-raw_data.zip`);
}

/** Saves a JSON value as a file. */
export function downloadJson(value: unknown, name: string): void {
  saveBlob(new Blob([JSON.stringify(value)], { type: 'application/json' }), name);
}
