import { describe, expect, it } from 'vitest';
import { downsample, fitFactor, releaseOnUpload, type UploadedTexture } from './textures';

describe('fitFactor', () => {
  it('is 1 when the image fits', () => {
    expect(fitFactor(8192, 4096, 16384)).toBe(1);
    expect(fitFactor(4096, 2048, 4096)).toBe(1);
  });

  it('shrinks the widest side under the limit', () => {
    expect(fitFactor(8192, 4096, 4096)).toBe(2);
    expect(fitFactor(8193, 4096, 4096)).toBe(3);
  });
});

describe('downsample', () => {
  it('returns the same array at factor 1', () => {
    const data = Uint8Array.from([1, 2, 3, 4]);
    expect(downsample(data, 2, 2, 1, 1).data).toBe(data);
  });

  it('keeps the centre pixel of each block, every channel, with the same type', () => {
    // 4 × 2 pixels of two channels: value = 10 × pixel index + channel.
    const data = Uint16Array.from({ length: 16 }, (_, k) => 10 * Math.floor(k / 2) + (k % 2));
    const { data: out, width, height } = downsample(data, 4, 2, 2, 2);
    expect(out).toBeInstanceOf(Uint16Array);
    expect([width, height]).toEqual([2, 1]);
    // Block centres: pixel (1, 1) = index 5 and (3, 1) = index 7.
    expect(Array.from(out)).toEqual([50, 51, 70, 71]);
  });

  it('clamps the last partial block to the image', () => {
    const data = Uint8Array.from([0, 1, 2]);
    const { data: out, width } = downsample(data, 3, 1, 1, 2);
    expect(width).toBe(2);
    expect(Array.from(out)).toEqual([1, 2]);
  });
});

describe('releaseOnUpload', () => {
  const upload = (texture: UploadedTexture) => (texture.onUpdate as ((t: unknown) => void) | null)?.(texture);

  it('drops the pixels a data texture owns after its upload, once', () => {
    const texture: UploadedTexture = { image: { data: new Uint8Array(4), width: 2, height: 2 }, onUpdate: null };
    releaseOnUpload(texture);
    expect((texture.image as { data: unknown }).data).not.toBeNull();
    upload(texture);
    expect(texture.image).toEqual({ data: null, width: 2, height: 2 });
    expect(texture.onUpdate).toBeNull();
  });

  it('keeps pixels shared with the decoded world', () => {
    const data = new Uint8Array(4);
    const texture: UploadedTexture = { image: { data, width: 2, height: 2 }, onUpdate: null };
    releaseOnUpload(texture, false);
    upload(texture);
    expect((texture.image as { data: unknown }).data).toBe(data);
  });
});
