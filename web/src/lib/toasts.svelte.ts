// Short notifications at the bottom of the screen, optionally with an action
// (undo, open). Each one disappears after a while or when dismissed.

export interface Toast {
  id: number;
  message: string;
  kind: 'info' | 'error';
  action?: { label: string; run: () => void };
}

let nextId = 1;

class Toasts {
  list = $state<Toast[]>([]);

  /** Shows a toast for `ms` milliseconds; returns its id. */
  show(message: string, options: { kind?: Toast['kind']; action?: Toast['action']; ms?: number } = {}): number {
    const id = nextId;
    nextId += 1;
    const toast: Toast = { id, message, kind: options.kind ?? 'info' };
    if (options.action) {
      toast.action = options.action;
    }
    this.list = [...this.list, toast];
    setTimeout(() => this.dismiss(id), options.ms ?? 4000);
    return id;
  }

  error(message: string): number {
    return this.show(message, { kind: 'error', ms: 8000 });
  }

  dismiss(id: number): void {
    this.list = this.list.filter((t) => t.id !== id);
  }
}

export const toasts = new Toasts();
