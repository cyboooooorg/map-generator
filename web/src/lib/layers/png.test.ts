import { describe, expect, it } from 'vitest';
import { encode } from 'fast-png';
import { zlibSync } from 'fflate';
import { decodePng } from './png';

describe('decodePng', () => {
  it('reads 8- and 16-bit greyscale and colour images', () => {
    const grey16 = encode({ width: 3, height: 2, channels: 1, depth: 16, data: Uint16Array.from([0, 1, 65535, 300, 40000, 7]) });
    expect(decodePng(grey16)).toMatchObject({ width: 3, height: 2, depth: 16, channels: 1 });
    expect(Array.from(decodePng(grey16).data)).toEqual([0, 1, 65535, 300, 40000, 7]);
    const rgb = encode({ width: 2, height: 1, channels: 3, depth: 8, data: Uint8Array.from([1, 2, 3, 4, 5, 6]) });
    expect(Array.from(decodePng(rgb).data)).toEqual([1, 2, 3, 4, 5, 6]);
  });

  it('decodes images past half a megabyte, which streaming inflation corrupted', () => {
    // A smooth 16-bit ramp of 1 MB, every row filtered with Paeth (type 4),
    // as the layers of a 960 × 540 world.
    const width = 960;
    const height = 540;
    const values = new Uint16Array(width * height);
    for (let y = 0; y < height; y++) {
      for (let x = 0; x < width; x++) {
        values[y * width + x] = Math.round(20_000 + 30 * y + 10 * Math.sin(x / 40) * 100);
      }
    }
    const png = encode({ width, height, channels: 1, depth: 16, data: values });
    expect(Array.from(decodePng(png).data).every((v, i) => v === values[i])).toBe(true);
    // An unsupported file is refused, not misread.
    expect(() => decodePng(zlibSync(Uint8Array.from([1, 2, 3])))).toThrow(/not a PNG/);
  });
});
