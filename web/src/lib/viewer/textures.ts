// Fitting world-sized images into the GPU's texture size limit: an 8192 ×
// 4096 world exceeds the 4096 pixels some GPUs allow per side.
import { DataTexture, RGBAFormat, UnsignedByteType } from 'three';

/** Smallest integer factor that brings both sides within `max` pixels. */
export function fitFactor(width: number, height: number, max: number): number {
  let factor = 1;
  while (Math.ceil(width / factor) > max || Math.ceil(height / factor) > max) {
    factor += 1;
  }
  return factor;
}

/**
 * Shrinks a row-major image of `channels` values per pixel by `factor`,
 * keeping the pixel at the centre of each block (biome ids cannot be
 * averaged). Returns the array itself when `factor` is 1.
 */
export function downsample<T extends Uint8Array | Uint16Array>(
  data: T,
  width: number,
  height: number,
  channels: number,
  factor: number,
): { data: T; width: number; height: number } {
  if (factor <= 1) {
    return { data, width, height };
  }
  const w = Math.ceil(width / factor);
  const h = Math.ceil(height / factor);
  const out = new (data.constructor as new (length: number) => T)(w * h * channels);
  const half = Math.floor(factor / 2);
  for (let y = 0; y < h; y += 1) {
    const sy = Math.min(height - 1, y * factor + half);
    for (let x = 0; x < w; x += 1) {
      const sx = Math.min(width - 1, x * factor + half);
      const from = (sy * width + sx) * channels;
      const to = (y * w + x) * channels;
      for (let c = 0; c < channels; c += 1) {
        out[to + c] = data[from + c]!;
      }
    }
  }
  return { data: out, width: w, height: h };
}

/** The part of a three.js texture `releaseOnUpload` touches. */
export interface UploadedTexture {
  image: unknown;
  onUpdate: ((texture: never) => void) | null;
}

/**
 * Lets the CPU copy of a texture go once the GPU has it: closes its
 * ImageBitmap, or drops the pixel array of a data texture that owns it
 * (`ownsData`; an array shared with the decoded world stays).
 *
 * Nothing uploads the texture again: three.js uploads only when the
 * texture's version changes (`needsUpdate`, never set again here: a
 * sampling setting changed after the upload would need it), resizing the
 * canvas keeps the textures, and the WebGL context is only lost when the
 * view is disposed.
 */
export function releaseOnUpload(texture: UploadedTexture, ownsData = true): void {
  texture.onUpdate = () => {
    texture.onUpdate = null;
    const image = texture.image;
    if (typeof ImageBitmap !== 'undefined' && image instanceof ImageBitmap) {
      image.close();
    } else if (ownsData && typeof image === 'object' && image !== null && 'data' in image) {
      (image as { data: unknown }).data = null;
    }
  };
}

/** A 1 × 1 transparent black texture: the value of a texture uniform with nothing to show. */
export function emptyTexture(): DataTexture {
  const texture = new DataTexture(new Uint8Array(4), 1, 1, RGBAFormat, UnsignedByteType);
  texture.needsUpdate = true;
  return texture;
}
