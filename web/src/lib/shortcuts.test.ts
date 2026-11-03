import { describe, expect, it } from 'vitest';
import { altAllowed, shortcutFor, SHORTCUT_HELP, stepRelief, stepWorld } from './shortcuts';

describe('shortcutFor', () => {
  it('maps digits to layers and letters in either case', () => {
    expect(shortcutFor('1')).toEqual({ action: 'layer', index: 0 });
    expect(shortcutFor('8')).toEqual({ action: 'layer', index: 7 });
    expect(shortcutFor('9')).toBeNull();
    expect(shortcutFor('0')).toBeNull();
    expect(shortcutFor('S')).toEqual({ action: 'seasons' });
    expect(shortcutFor('w')).toEqual({ action: 'drift' });
    expect(shortcutFor('Escape')).toEqual({ action: 'escape' });
    expect(shortcutFor('x')).toBeNull();
    expect(shortcutFor('F1')).toBeNull();
  });

  it('maps the brackets and the relief keys, with or without Shift', () => {
    expect(shortcutFor('[')).toEqual({ action: 'previousWorld' });
    expect(shortcutFor(']')).toEqual({ action: 'nextWorld' });
    for (const key of ['+', '=']) expect(shortcutFor(key)).toEqual({ action: 'reliefUp' });
    for (const key of ['-', '_']) expect(shortcutFor(key)).toEqual({ action: 'reliefDown' });
  });

  it('lets Alt type symbols but not letters or digits', () => {
    expect(altAllowed('[')).toBe(true);
    expect(altAllowed(']')).toBe(true);
    expect(altAllowed('g')).toBe(false);
    expect(altAllowed('é')).toBe(false);
    expect(altAllowed('3')).toBe(false);
    expect(altAllowed('Escape')).toBe(false);
  });

  it('lists every key in the help', () => {
    const keys = SHORTCUT_HELP.map(([key]) => key).join(' ');
    for (const key of ['G', 'M', 'R', 'H', 'L', 'S', 'A', 'C', 'W', 'N', 'T', 'K', 'J', 'D', 'P', '?', '[', ']', '+']) {
      expect(keys).toContain(key);
    }
  });
});

describe('stepRelief', () => {
  it('steps by tenths within [0, 1]', () => {
    expect(stepRelief(0, 1)).toBe(0.1);
    expect(stepRelief(0.35, 1)).toBe(0.45);
    expect(stepRelief(0.2, -1)).toBe(0.1);
    expect(stepRelief(0.95, 1)).toBe(1);
    expect(stepRelief(0.05, -1)).toBe(0);
    expect(stepRelief(0, -1)).toBe(0);
  });
});

describe('stepWorld', () => {
  const order = ['a', 'b', 'c'];
  it('moves through the list, wrapping around', () => {
    expect(stepWorld(order, 'a', 1)).toBe('b');
    expect(stepWorld(order, 'c', 1)).toBe('a');
    expect(stepWorld(order, 'a', -1)).toBe('c');
  });

  it('starts at an end when the current world is not listed', () => {
    expect(stepWorld(order, null, 1)).toBe('a');
    expect(stepWorld(order, 'z', -1)).toBe('c');
  });

  it('does nothing without another world', () => {
    expect(stepWorld([], 'a', 1)).toBeNull();
    expect(stepWorld(['a'], 'a', 1)).toBeNull();
  });
});
