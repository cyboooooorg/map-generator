// What a map size costs: generation time (learnt from earlier generations in
// this browser) and memory, shown next to the size in the generation form.

/**
 * Memory the server peaks at during a generation, measured in Docker (see
 * "Web app" in the repository README): [pixels, bytes]. Sizes in between
 * are interpolated, larger ones extrapolated from the last two points.
 */
const GIB = 2 ** 30;
const MIB = 2 ** 20;
const SERVER_PEAK: readonly [number, number][] = [
  [0, 0],
  [1920 * 1080, 0.16 * GIB],
  [4096 * 2048, 0.55 * GIB],
  [8192 * 4096, 2.0 * GIB],
];

/**
 * Bytes per pixel a decoded world takes in the page, with the layers of
 * format version 2 and later (web/README.md), which every new world has.
 */
export const VIEWER_BYTES_PER_PIXEL = 19;

/** The server's memory peak while generating a map of `pixels`, in bytes. */
export function serverPeakBytes(pixels: number): number {
  for (let i = 1; i < SERVER_PEAK.length; i += 1) {
    const [x1, y1] = SERVER_PEAK[i]!;
    const [x0, y0] = SERVER_PEAK[i - 1]!;
    if (pixels <= x1 || i === SERVER_PEAK.length - 1) {
      return y0 + ((pixels - x0) * (y1 - y0)) / (x1 - x0);
    }
  }
  return 0;
}

/** The page's memory for a decoded world of `pixels`, in bytes. */
export function viewerBytes(pixels: number): number {
  return pixels * VIEWER_BYTES_PER_PIXEL;
}

/**
 * Seconds a generation of `megapixels` should take, from the seconds per
 * megapixel learnt for each stage; null until every stage has been timed.
 */
export function estimateSeconds(
  secondsPerMegapixel: Readonly<Partial<Record<string, number>>>,
  stages: readonly string[],
  megapixels: number,
): number | null {
  let total = 0;
  for (const stage of stages) {
    const perMp = secondsPerMegapixel[stage];
    if (perMp === undefined) {
      return null;
    }
    total += perMp * megapixels;
  }
  return total;
}

/** An estimated duration, rounded as estimates deserve: "about 8 s", "about 2 min 30 s". */
export function formatEstimate(seconds: number): string {
  if (seconds < 1) {
    return 'under a second';
  }
  if (seconds < 60) {
    return `about ${Math.round(seconds)} s`;
  }
  const rounded = Math.round(seconds / 10) * 10;
  const minutes = Math.floor(rounded / 60);
  const rest = rounded % 60;
  return rest === 0 ? `about ${minutes} min` : `about ${minutes} min ${rest} s`;
}

/** A memory size, in binary units as the README quotes them: "160 MiB", "1.2 GiB". */
export function formatBytes(bytes: number): string {
  if (bytes >= GIB) {
    return `${(bytes / GIB).toFixed(1)} GiB`;
  }
  return `${Math.max(10, Math.round(bytes / MIB / 10) * 10)} MiB`;
}
