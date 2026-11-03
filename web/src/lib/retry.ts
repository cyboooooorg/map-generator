// Retrying a request while the server says it is busy, abortable at any time.

/** Resolves after `ms`, or rejects with the abort reason when `signal` aborts first. */
export function sleep(ms: number, signal: AbortSignal): Promise<void> {
  return new Promise((resolve, reject) => {
    if (signal.aborted) {
      reject(signal.reason as Error);
      return;
    }
    const onAbort = (): void => {
      clearTimeout(timer);
      reject(signal.reason as Error);
    };
    const timer = setTimeout(() => {
      signal.removeEventListener('abort', onAbort);
      resolve();
    }, ms);
    signal.addEventListener('abort', onAbort, { once: true });
  });
}

export interface BusyRetry {
  /** Aborts the request and the waits between attempts. */
  signal: AbortSignal;
  /** Whether a failure means "busy, try again later". */
  isBusy: (error: unknown) => boolean;
  /** Wait before the next attempt, ms. */
  delayMs: number;
  /** Called on each busy answer, before the wait. */
  onBusy?: (error: unknown) => void;
}

/**
 * Runs `attempt` until it succeeds or fails for another reason than being
 * busy, waiting `delayMs` after each busy answer. Once `signal` aborts, it
 * rejects with the abort reason, whatever the attempt did.
 */
export async function retryWhileBusy<T>(attempt: (signal: AbortSignal) => Promise<T>, options: BusyRetry): Promise<T> {
  const { signal } = options;
  for (;;) {
    signal.throwIfAborted();
    try {
      const result = await attempt(signal);
      signal.throwIfAborted();
      return result;
    } catch (e) {
      signal.throwIfAborted();
      if (!options.isBusy(e)) {
        throw e;
      }
      options.onBusy?.(e);
    }
    await sleep(options.delayMs, signal);
  }
}
