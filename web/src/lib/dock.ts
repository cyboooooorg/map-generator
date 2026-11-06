// The layout of a viewer's panels (see components/dock/Dock.svelte): which
// panel is open in the dock, whether the dock is folded to its tabs, and
// which panels float where. Remembered per kind of viewer, so both panes
// of a comparison share it.
import { loadSetting, saveSetting } from './settings';

export type DockKind = 'star' | 'planet';

/** Where a floating panel's top-left corner is, CSS px in the view. */
export interface FloatAt {
  x: number;
  y: number;
}

export interface DockLayout {
  /** The panel open in the dock, by id, or none (the dock shows its tabs only). */
  active: string | null;
  /** Whether the dock is folded to its icons. */
  folded: boolean;
  /** The panels taken out of the dock, by id. */
  floating: Record<string, FloatAt>;
}

const fallback = (active: string | null): DockLayout => ({ active, folded: false, floating: {} });

/**
 * The layout remembered for `kind`, kept to the panels `ids` (an unknown or
 * malformed one is dropped): `defaultActive` opens when nothing is.
 */
export function loadLayout(kind: DockKind, ids: readonly string[], defaultActive: string | null): DockLayout {
  const stored = loadSetting<unknown>(`dock.${kind}`, null);
  if (typeof stored !== 'object' || stored === null) return fallback(defaultActive);
  const raw = stored as Partial<Record<keyof DockLayout, unknown>>;
  const floating: Record<string, FloatAt> = {};
  if (typeof raw.floating === 'object' && raw.floating !== null) {
    for (const [id, at] of Object.entries(raw.floating as Record<string, unknown>)) {
      const point = at as Partial<FloatAt> | null;
      if (ids.includes(id) && point && Number.isFinite(point.x) && Number.isFinite(point.y)) {
        floating[id] = { x: point.x!, y: point.y! };
      }
    }
  }
  const active = typeof raw.active === 'string' && ids.includes(raw.active) && !floating[raw.active] ? raw.active : null;
  return {
    active: raw.active === null ? null : (active ?? defaultActive),
    folded: raw.folded === true,
    floating,
  };
}

export function saveLayout(kind: DockKind, layout: DockLayout): void {
  saveSetting(`dock.${kind}`, layout);
}

/**
 * Keeps a floating panel of size `width` × `height` inside a view of
 * `viewWidth` × `viewHeight` (CSS px), a `margin` from its edges: as much of
 * it as fits, its header always reachable.
 */
export function clampToView(at: FloatAt, width: number, height: number, viewWidth: number, viewHeight: number, margin = 8): FloatAt {
  const maxX = Math.max(margin, viewWidth - width - margin);
  const maxY = Math.max(margin, viewHeight - Math.min(height, viewHeight - 2 * margin) - margin);
  return {
    x: Math.min(Math.max(at.x, margin), maxX),
    y: Math.min(Math.max(at.y, margin), maxY),
  };
}

/** Width of the point panel (see components/InfoPanel.svelte), CSS px. */
export const INFO_PANEL_WIDTH = 300;
/** What the point panel stays clear of: the toolbar above, the playback bar below, CSS px. */
export const INFO_PANEL_TOP = 56;
export const INFO_PANEL_BOTTOM = 72;

/**
 * The point panel's place in a view of `viewWidth` × `viewHeight`: where the
 * user dragged it (`saved`), kept inside the view and above the playback
 * bar, or by default centred under the toolbar, clear of the icon bar on
 * the left and the dock on the right.
 */
export function infoPanelAt(saved: FloatAt | null, height: number, viewWidth: number, viewHeight: number): FloatAt {
  const at = saved ?? { x: (viewWidth - INFO_PANEL_WIDTH) / 2, y: INFO_PANEL_TOP };
  return clampToView(at, INFO_PANEL_WIDTH, height, viewWidth, viewHeight - INFO_PANEL_BOTTOM);
}

/** Narrower views (CSS px) have no floating panels: the dock opens its panels full width. */
export const FLOAT_MIN_WIDTH = 720;
