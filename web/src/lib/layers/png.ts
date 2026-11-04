// A decoder for the PNGs of a world's raw data: 8 or 16 bits per channel,
// not interlaced, greyscale as mapgen writes them (grey with alpha, RGB and
// RGBA are read too, their channels interleaved).
//
// fast-png inflates the image data with fflate's streaming `Unzlib`, which
// corrupts its output past 512 KiB on some files (fflate 0.8.2 and 0.8.3):
// the southern half of a 960 × 540 elevation layer came out as noise. The
// one-shot `unzlibSync` inflates the same data correctly, so the image data
// is gathered and inflated in one go here.
import { unzlibSync } from 'fflate';

export interface RawPng {
  width: number;
  height: number;
  depth: 8 | 16;
  /** Channels per pixel: 1 grey, 2 grey and alpha, 3 RGB, 4 RGBA. */
  channels: number;
  /** Row-major samples, row 0 at the top, channels interleaved. */
  data: Uint8Array | Uint16Array;
}

/** Channels of each PNG colour type (palettes are not supported). */
const CHANNELS: Record<number, number> = { 0: 1, 2: 3, 4: 2, 6: 4 };

const SIGNATURE = [137, 80, 78, 71, 13, 10, 26, 10];

/** Decodes a PNG of 8 or 16 bits per channel (see the module comment). */
export function decodePng(file: ArrayBuffer | Uint8Array): RawPng {
  const bytes = file instanceof Uint8Array ? file : new Uint8Array(file);
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  if (!SIGNATURE.every((b, i) => bytes[i] === b)) {
    throw new Error('not a PNG file');
  }
  let width = 0;
  let height = 0;
  let depth = 0;
  let channels = 0;
  const parts: Uint8Array[] = [];
  let size = 0;
  for (let at = 8; at + 8 <= bytes.length; ) {
    const length = view.getUint32(at);
    const type = String.fromCharCode(bytes[at + 4]!, bytes[at + 5]!, bytes[at + 6]!, bytes[at + 7]!);
    const body = bytes.subarray(at + 8, at + 8 + length);
    if (type === 'IHDR') {
      width = view.getUint32(at + 8);
      height = view.getUint32(at + 12);
      depth = body[8]!;
      const colour = body[9]!;
      const interlace = body[12]!;
      channels = CHANNELS[colour] ?? 0;
      if (channels === 0 || (depth !== 8 && depth !== 16) || interlace !== 0) {
        throw new Error(`PNG of colour type ${colour}, ${depth} bits, interlace ${interlace}: not supported`);
      }
    } else if (type === 'IDAT') {
      parts.push(body);
      size += body.length;
    } else if (type === 'IEND') {
      break;
    }
    at += 12 + length;
  }
  const compressed = new Uint8Array(size);
  let offset = 0;
  for (const part of parts) {
    compressed.set(part, offset);
    offset += part.length;
  }
  const raw = unzlibSync(compressed);
  const bpp = (depth / 8) * channels;
  const stride = width * bpp;
  if (raw.length < (stride + 1) * height) {
    throw new Error(`PNG data is ${raw.length} bytes, expected ${(stride + 1) * height}`);
  }
  // Undo the row filters in place, row by row (each row after its filter byte).
  const pixels = new Uint8Array(stride * height);
  for (let y = 0; y < height; y += 1) {
    const filter = raw[y * (stride + 1)]!;
    const src = y * (stride + 1) + 1;
    const row = y * stride;
    const up = row - stride;
    for (let x = 0; x < stride; x += 1) {
      const a = x >= bpp ? pixels[row + x - bpp]! : 0;
      const b = y > 0 ? pixels[up + x]! : 0;
      const c = x >= bpp && y > 0 ? pixels[up + x - bpp]! : 0;
      let value = raw[src + x]!;
      if (filter === 1) {
        value += a;
      } else if (filter === 2) {
        value += b;
      } else if (filter === 3) {
        value += (a + b) >> 1;
      } else if (filter === 4) {
        const p = a + b - c;
        const pa = Math.abs(p - a);
        const pb = Math.abs(p - b);
        const pc = Math.abs(p - c);
        value += pa <= pb && pa <= pc ? a : pb <= pc ? b : c;
      } else if (filter !== 0) {
        throw new Error(`unknown PNG filter ${filter}`);
      }
      pixels[row + x] = value & 0xff;
    }
  }
  if (depth === 8) {
    return { width, height, depth: 8, channels, data: pixels };
  }
  // 16 bits: big-endian samples.
  const data = new Uint16Array(width * height * channels);
  for (let i = 0; i < data.length; i += 1) {
    data[i] = (pixels[2 * i]! << 8) | pixels[2 * i + 1]!;
  }
  return { width, height, depth: 16, channels, data };
}
