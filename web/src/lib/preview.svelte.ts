// Live preview of the generate form: what the request would draw, asked of
// the server once the form has been still for a moment. The same answer
// validates the form, so the form never disagrees with Generate.
import { api, ApiFailure } from './api/client';
import type { ErrorDetail } from './api/types/ErrorDetail';
import type { Quantity } from './api/types/Quantity';
import type { ResolvedSystem } from './api/types/ResolvedSystem';
import type { SystemRequest } from './api/types/SystemRequest';
import { changedQuantities } from './causes';

/** Quiet time after the last change before the server is asked, ms. */
export const PREVIEW_DEBOUNCE_MS = 300;

export class Preview {
  /** The last answer. Kept while a newer one is awaited or the server is unreachable. */
  result = $state<ResolvedSystem | null>(null);
  /** Why the server refused the last request, null when it did not. */
  refusal = $state<ErrorDetail | null>(null);
  /** The server could not answer (not a refusal of the request). */
  unavailable = $state(false);
  /** A request waits for its debounce or its answer. */
  pending = $state(false);
  /** The roots of the explanation that moved by more than 1 % since the previous answer. */
  changed = $state<ReadonlySet<Quantity>>(new Set());

  private timer: ReturnType<typeof setTimeout> | null = null;
  private controller: AbortController | null = null;
  private latest = 0;

  /** Asks for `next` once no other request comes within the debounce; older answers are dropped. */
  request(next: SystemRequest): void {
    const id = ++this.latest;
    this.pending = true;
    this.controller?.abort();
    this.controller = null;
    if (this.timer !== null) clearTimeout(this.timer);
    this.timer = setTimeout(() => {
      this.timer = null;
      void this.ask(id, next);
    }, PREVIEW_DEBOUNCE_MS);
  }

  /** Cancels what is waiting or in flight. */
  dispose(): void {
    this.latest++;
    if (this.timer !== null) clearTimeout(this.timer);
    this.timer = null;
    this.controller?.abort();
    this.controller = null;
    this.pending = false;
  }

  private async ask(id: number, request: SystemRequest): Promise<void> {
    const controller = new AbortController();
    this.controller = controller;
    try {
      const answer = await api.resolveSystem(request, controller.signal);
      if (id !== this.latest) return;
      this.changed = changedQuantities(explanationOf(this.result), explanationOf(answer));
      this.result = answer;
      this.refusal = null;
      this.unavailable = false;
    } catch (error) {
      if (id !== this.latest) return;
      if (error instanceof ApiFailure && error.status === 400 && error.detail) {
        this.refusal = error.detail;
        this.unavailable = false;
      } else {
        this.unavailable = true;
      }
    }
    if (id === this.latest) {
      this.pending = false;
      this.controller = null;
    }
  }
}

function explanationOf(result: ResolvedSystem | null) {
  const focus = result?.system.focus;
  return (focus ? result?.extras[focus]?.explanation : null) ?? null;
}
