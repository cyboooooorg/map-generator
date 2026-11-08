/**
 * Frame-time probe, on when the page URL has `?frames=1`. It exists so that a
 * visual change can be checked against the frame rate: see "Measuring frame
 * time" in web/README.md. When off, `measure` only calls `draw`.
 */

/** On when the page URL has `?frames=1` (the search, not the hash: the hash is the app's route). */
export function frameStatsEnabled(search: string): boolean {
  return new URLSearchParams(search).get('frames') === '1';
}

export interface FrameSummary {
  view: string;
  frames: number;
  medianMs: number;
  p95Ms: number;
}

const registry = new Map<string, FrameStats>();
const LOG_PERIOD_MS = 5000;
let logTimer: ReturnType<typeof setInterval> | null = null;

/** Every view's FrameStats register here; logged with console.table every 5 s and readable as window.__frames. */
export function frameRegistry(): Map<string, FrameStats> {
  return registry;
}

function summaries(): FrameSummary[] {
  const out: FrameSummary[] = [];
  for (const stats of registry.values()) {
    const s = stats.summary();
    if (s) out.push(s);
  }
  return out;
}

function startLogging(): void {
  if (logTimer !== null || typeof window === 'undefined') return;
  (window as unknown as { __frames?: () => FrameSummary[] }).__frames = summaries;
  logTimer = setInterval(() => {
    const rows = summaries();
    if (rows.length > 0) console.table(rows);
  }, LOG_PERIOD_MS);
}

function stopLogging(): void {
  if (logTimer === null) return;
  clearInterval(logTimer);
  logTimer = null;
  delete (window as unknown as { __frames?: unknown }).__frames;
}

/** Linear-interpolated quantile of a sorted array. */
function quantile(sorted: Float64Array, q: number): number {
  const pos = q * (sorted.length - 1);
  const lo = Math.floor(pos);
  const hi = Math.min(lo + 1, sorted.length - 1);
  const a = sorted[lo] ?? 0;
  const b = sorted[hi] ?? a;
  return a + (b - a) * (pos - lo);
}

export class FrameStats {
  private readonly samples: Float64Array;
  private readonly pixel: Uint8Array | null;
  private count = 0;
  private next = 0;

  /**
   * @param view name in the registry ('system', 'planet')
   * @param windowFrames how many of the latest frames the summary covers
   * @param now clock in ms (injectable for tests)
   * @param enabled read once, by default from `location.search`
   */
  constructor(
    private readonly view: string,
    private readonly windowFrames = 300,
    private readonly now: () => number = () => performance.now(),
    private readonly enabled: boolean = typeof location !== 'undefined' &&
      frameStatsEnabled(location.search),
  ) {
    this.samples = new Float64Array(enabled ? windowFrames : 0);
    this.pixel = enabled ? new Uint8Array(4) : null;
    if (enabled) {
      registry.set(view, this);
      startLogging();
    }
  }

  /** Times `draw`; when enabled, forces the GPU to finish (1-pixel readPixels) so the time includes it. */
  measure(gl: WebGLRenderingContext | WebGL2RenderingContext, draw: () => void): void {
    if (!this.enabled || this.pixel === null) {
      draw();
      return;
    }
    const start = this.now();
    draw();
    gl.readPixels(0, 0, 1, 1, gl.RGBA, gl.UNSIGNED_BYTE, this.pixel);
    this.samples[this.next] = this.now() - start;
    this.next = (this.next + 1) % this.windowFrames;
    if (this.count < this.windowFrames) this.count += 1;
  }

  summary(): FrameSummary | null {
    if (this.count === 0) return null;
    const sorted = this.samples.slice(0, this.count).sort();
    return {
      view: this.view,
      frames: this.count,
      medianMs: quantile(sorted, 0.5),
      p95Ms: quantile(sorted, 0.95),
    };
  }

  /** Leaves the registry and stops the log timer once no view is left. */
  dispose(): void {
    if (registry.get(this.view) === this) registry.delete(this.view);
    if (registry.size === 0) stopLogging();
  }
}
