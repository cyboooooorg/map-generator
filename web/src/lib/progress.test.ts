import { describe, expect, it } from 'vitest';
import {
  combineDownloads,
  expectedLength,
  type FileProgress,
  progressShare,
  progressText,
  readWithProgress,
} from './progress';

function streamed(chunks: number[][], headers: Record<string, string> = {}): Response {
  const body = new ReadableStream<Uint8Array>({
    start(controller) {
      for (const chunk of chunks) controller.enqueue(Uint8Array.from(chunk));
      controller.close();
    },
  });
  return new Response(body, { headers });
}

describe('expectedLength', () => {
  it('reads Content-Length', () => {
    expect(expectedLength(new Headers({ 'Content-Length': '1234' }))).toBe(1234);
  });

  it('is unknown without it, or when the body is compressed', () => {
    expect(expectedLength(new Headers())).toBeNull();
    expect(expectedLength(new Headers({ 'Content-Length': 'x' }))).toBeNull();
    expect(expectedLength(new Headers({ 'Content-Length': '10', 'Content-Encoding': 'gzip' }))).toBeNull();
    expect(expectedLength(new Headers({ 'Content-Length': '10', 'Content-Encoding': 'identity' }))).toBe(10);
  });
});

describe('readWithProgress', () => {
  it('reports the bytes as chunks arrive and returns the whole body', async () => {
    const seen: FileProgress[] = [];
    const buffer = await readWithProgress(streamed([[1, 2], [3], [4, 5, 6]], { 'Content-Length': '6' }), (p) =>
      seen.push(p),
    );
    expect([...new Uint8Array(buffer)]).toEqual([1, 2, 3, 4, 5, 6]);
    expect(seen.map((p) => p.received)).toEqual([0, 2, 3, 6]);
    expect(seen.every((p) => p.total === 6)).toBe(true);
  });

  it('ends at the bytes received when the announced length was wrong', async () => {
    const seen: FileProgress[] = [];
    await readWithProgress(streamed([[1, 2, 3]], { 'Content-Length': '10' }), (p) => seen.push(p));
    expect(seen.at(-1)).toEqual({ received: 3, total: 3 });
  });

  it('works without a known length', async () => {
    const seen: FileProgress[] = [];
    const buffer = await readWithProgress(streamed([[7], [8]]), (p) => seen.push(p));
    expect(buffer.byteLength).toBe(2);
    expect(seen.at(-1)).toEqual({ received: 2, total: null });
  });
});

describe('combineDownloads', () => {
  it('adds the files up, with a total once every size is known', () => {
    const files = new Map<string, FileProgress>([
      ['a.png', { received: 10, total: 100 }],
      ['b.png', { received: 5, total: 50 }],
    ]);
    expect(combineDownloads(files, 'b.png')).toEqual({ stage: 'download', label: 'b.png', done: 15, total: 150 });
    files.set('c.png', { received: 0, total: null });
    expect(combineDownloads(files, 'a.png').total).toBeNull();
  });
});

describe('progressShare and progressText', () => {
  it('give the share and the amounts', () => {
    const progress = { stage: 'download' as const, label: 'x', done: 1_500_000, total: 6_000_000 };
    expect(progressShare(progress)).toBe(0.25);
    expect(progressText(progress)).toBe('1.5 / 6.0 MB');
    expect(progressShare({ ...progress, total: null })).toBeNull();
    expect(progressText({ ...progress, total: null })).toBe('1.5 MB');
    expect(progressShare({ ...progress, done: 9, total: 0 })).toBe(1);
    expect(progressText({ stage: 'decode', label: 'y', done: 2, total: 9 })).toBe('2 / 9');
  });
});
