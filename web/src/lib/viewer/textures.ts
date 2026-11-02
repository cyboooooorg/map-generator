// Fitting world-sized images into the GPU's texture size limit: an 8192 ×
// 4096 world exceeds the 4096 pixels some GPUs allow per side.

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
