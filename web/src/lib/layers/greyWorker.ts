// Web Worker: fetches and decodes one 8-bit grey image (the cloud cover)
// off the main thread, as one byte per pixel.
import { decodeGrey8, type Grey8 } from './decode';

export interface GreyRequest {
  url: string;
  /** Largest side the GPU takes: bigger images are shrunk to fit. */
  maxSize: number;
}

export type GreyResponse = { ok: true; image: Grey8 } | { ok: false; error: string };

self.onmessage = async (event: MessageEvent<GreyRequest>) => {
  const { url, maxSize } = event.data;
  try {
    const response = await fetch(url);
    if (!response.ok) {
      throw new Error(`${url}: ${response.status} ${response.statusText}`);
    }
    const image = decodeGrey8(await response.arrayBuffer(), maxSize);
    const message: GreyResponse = { ok: true, image };
    self.postMessage(message, { transfer: [image.data.buffer as ArrayBuffer] });
  } catch (error) {
    const message: GreyResponse = { ok: false, error: String(error) };
    self.postMessage(message);
  }
};
