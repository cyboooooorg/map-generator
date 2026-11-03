// Download and decode progress of the files of a world, for the loading
// bar of the viewer.

/** Where the loading of a set of files stands. */
export interface LoadProgress {
  /** 'download' while bytes arrive, 'decode' once every file is in. */
  stage: 'download' | 'decode';
  /** The file (or decoding step) being worked on, as shown to the user. */
  label: string;
  /** Bytes received (download), or steps done (decode). */
  done: number;
  /** Bytes expected, or steps in all; null while a download's size is unknown. */
  total: number | null;
}

/** One file's download: bytes received, and expected when the server said so. */
export interface FileProgress {
  received: number;
  total: number | null;
}

/**
 * The size of a response body as it will be read, from Content-Length, or
 * null when unknown. A compressed body is read decompressed, so its length
 * on the wire says nothing about the bytes received.
 */
export function expectedLength(headers: Headers): number | null {
  const encoding = headers.get('Content-Encoding');
  if (encoding && encoding !== 'identity') {
    return null;
  }
  const length = Number(headers.get('Content-Length'));
  return headers.has('Content-Length') && Number.isFinite(length) && length >= 0 ? length : null;
}

/**
 * Reads a response body to the end, calling `onProgress` with the bytes
 * received so far as chunks arrive. Aborting the request's signal rejects.
 */
export async function readWithProgress(
  response: Response,
  onProgress: (progress: FileProgress) => void,
): Promise<ArrayBuffer> {
  const total = expectedLength(response.headers);
  onProgress({ received: 0, total });
  if (!response.body) {
    const buffer = await response.arrayBuffer();
    onProgress({ received: buffer.byteLength, total });
    return buffer;
  }
  const reader = response.body.getReader();
  const chunks: Uint8Array[] = [];
  let received = 0;
  for (;;) {
    const { done, value } = await reader.read();
    if (done) {
      break;
    }
    chunks.push(value);
    received += value.byteLength;
    onProgress({ received, total });
  }
  // A body longer than announced must not leave a bar stuck past 100 %.
  if (total !== null && received !== total) {
    onProgress({ received, total: received });
  }
  if (chunks.length === 1 && chunks[0]!.byteLength === chunks[0]!.buffer.byteLength) {
    return chunks[0]!.buffer as ArrayBuffer;
  }
  const bytes = new Uint8Array(received);
  let offset = 0;
  for (const chunk of chunks) {
    bytes.set(chunk, offset);
    offset += chunk.byteLength;
  }
  return bytes.buffer;
}

/**
 * The progress of several downloads at once: the bytes of all of them, and
 * the label of the one that moved last. The total is known only once every
 * file's size is.
 */
export function combineDownloads(files: ReadonlyMap<string, FileProgress>, last: string): LoadProgress {
  let done = 0;
  let total: number | null = 0;
  for (const file of files.values()) {
    done += file.received;
    total = total === null || file.total === null ? null : total + file.total;
  }
  return { stage: 'download', label: last, done, total };
}

/** The share done, in [0, 1], or null when the total is unknown. */
export function progressShare(progress: LoadProgress): number | null {
  if (progress.total === null) {
    return null;
  }
  return progress.total === 0 ? 1 : Math.min(progress.done / progress.total, 1);
}

/** "12.3 / 45.6 MB", "12.3 MB" while the size is unknown, or "2 / 5" for steps. */
export function progressText(progress: LoadProgress): string {
  if (progress.stage === 'decode') {
    return progress.total === null ? '' : `${progress.done} / ${progress.total}`;
  }
  const mb = (bytes: number) => (bytes / 1e6).toFixed(1);
  return progress.total === null ? `${mb(progress.done)} MB` : `${mb(progress.done)} / ${mb(progress.total)} MB`;
}
