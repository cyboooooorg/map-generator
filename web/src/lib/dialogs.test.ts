import { describe, expect, it } from 'vitest';
import { toggleDialog } from './dialogs';

describe('toggleDialog', () => {
  it('opens and closes a dialog', () => {
    expect(toggleDialog(null, 'help')).toBe('help');
    expect(toggleDialog('help', 'help')).toBeNull();
    expect(toggleDialog(null, 'units')).toBe('units');
    expect(toggleDialog('units', 'units')).toBeNull();
  });
  it('leaves only Units open when it opens over help, and the reverse', () => {
    expect(toggleDialog('help', 'units')).toBe('units');
    expect(toggleDialog('units', 'help')).toBe('help');
  });
  it('closes whichever is open on Escape', () => {
    expect(toggleDialog('help', 'escape')).toBeNull();
    expect(toggleDialog('units', 'escape')).toBeNull();
    expect(toggleDialog(null, 'escape')).toBeNull();
  });
});
