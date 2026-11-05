import { afterEach, describe, expect, it, vi } from 'vitest';
import { clampToView, loadLayout } from './dock';

const storage = (value: unknown) => {
  const store = new Map<string, string>([['mapgen.dock.star', JSON.stringify(value)]]);
  vi.stubGlobal('localStorage', { getItem: (k: string) => store.get(k) ?? null, setItem: () => {}, removeItem: () => {} });
};

describe('dock layout', () => {
  afterEach(() => vi.unstubAllGlobals());

  it('opens the default panel when nothing is remembered', () => {
    storage(null);
    expect(loadLayout('star', ['star', 'planet'], 'star')).toEqual({ active: 'star', folded: false, floating: {} });
  });

  it('keeps only the known panels, with finite places', () => {
    storage({ active: 'gone', folded: true, floating: { planet: { x: 40, y: 60 }, gone: { x: 1, y: 2 }, star: { x: 'a', y: 2 } } });
    expect(loadLayout('star', ['star', 'planet'], 'star')).toEqual({ active: 'star', folded: true, floating: { planet: { x: 40, y: 60 } } });
  });

  it('remembers a dock closed to its tabs, and never opens a floating panel in it', () => {
    storage({ active: null, folded: false, floating: {} });
    expect(loadLayout('star', ['star'], 'star').active).toBeNull();
    storage({ active: 'planet', folded: false, floating: { planet: { x: 0, y: 0 } } });
    expect(loadLayout('star', ['star', 'planet'], 'star').active).toBe('star');
  });

  it('keeps a floating panel inside the view, its header reachable', () => {
    expect(clampToView({ x: -50, y: -20 }, 200, 100, 800, 600)).toEqual({ x: 8, y: 8 });
    expect(clampToView({ x: 790, y: 590 }, 200, 100, 800, 600)).toEqual({ x: 592, y: 492 });
    // Taller than the view: its top stays on screen.
    expect(clampToView({ x: 100, y: 300 }, 200, 900, 800, 600)).toEqual({ x: 100, y: 8 });
  });
});
