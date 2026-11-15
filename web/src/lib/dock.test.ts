import { afterEach, describe, expect, it, vi } from 'vitest';
import { infoDockIds } from './bodyCards';
import { clampToView, INFO_PANEL_BOTTOM, INFO_PANEL_TOP, INFO_PANEL_WIDTH, infoPanelAt, loadLayout } from './dock';

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

  it('keeps a floating card id offered by a panel', () => {
    storage({ active: 'info', folded: false, floating: { 'info.orbit': { x: 10, y: 20 } } });
    expect(loadLayout('star', ['info', 'info.orbit'], 'info').floating).toEqual({ 'info.orbit': { x: 10, y: 20 } });
  });

  it('keeps the info panel and its floating cards while they are not offered yet', () => {
    storage({ active: 'info', folded: false, floating: { 'info.legend': { x: 10, y: 20 } } });
    const layout = loadLayout('star', ['map', ...infoDockIds()], 'map');
    expect(layout.active).toBe('info');
    expect(layout.floating).toEqual({ 'info.legend': { x: 10, y: 20 } });
  });

  it('drops a floating card no longer offered', () => {
    storage({ active: 'info', folded: false, floating: { 'info.orbit': { x: 10, y: 20 } } });
    expect(loadLayout('star', ['info'], 'info').floating).toEqual({});
  });

  it('loads a layout stored before cards existed', () => {
    storage({ active: 'info', folded: true, floating: { map: { x: 1, y: 2 } } });
    expect(loadLayout('star', ['info', 'map', 'info.orbit'], 'info')).toEqual({ active: 'info', folded: true, floating: { map: { x: 1, y: 2 } } });
  });

  it('keeps a floating panel inside the view, its header reachable', () => {
    expect(clampToView({ x: -50, y: -20 }, 200, 100, 800, 600)).toEqual({ x: 8, y: 8 });
    expect(clampToView({ x: 790, y: 590 }, 200, 100, 800, 600)).toEqual({ x: 592, y: 492 });
    // Taller than the view: its top stays on screen.
    expect(clampToView({ x: 100, y: 300 }, 200, 900, 800, 600)).toEqual({ x: 100, y: 8 });
  });
});

describe('infoPanelAt', () => {
  it('centres the panel under the toolbar by default', () => {
    expect(infoPanelAt(null, 400, 1400, 900)).toEqual({ x: 550, y: INFO_PANEL_TOP });
  });

  it('keeps a dragged place inside the view, above the playback bar', () => {
    expect(infoPanelAt({ x: 200, y: 100 }, 300, 1400, 900)).toEqual({ x: 200, y: 100 });
    expect(infoPanelAt({ x: 5000, y: 5000 }, 300, 1400, 900)).toEqual({ x: 1400 - INFO_PANEL_WIDTH - 8, y: 900 - INFO_PANEL_BOTTOM - 300 - 8 });
  });
});
