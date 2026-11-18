// Which of the viewer's two reference dialogs is open: at most one.

export type Dialog = 'help' | 'units';

/** The dialog open after `action`: it toggles its own, closes the other, and Escape closes any. */
export function toggleDialog(open: Dialog | null, action: Dialog | 'escape'): Dialog | null {
  if (action === 'escape') {
    return null;
  }
  return open === action ? null : action;
}
