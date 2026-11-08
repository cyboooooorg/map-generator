// The drift slider: the epoch of a world's tectonic plates, previewed at low
// resolution (mapgen::drift, `GET /api/systems/{id}/bodies/{body}/drift/…`)
// and generated at full size on request (`drift_ma` of the generation
// request). Unlike the crater epochs (epochs.ts), where positive means the
// past, drift follows `drift_ma`: Ma from the present, negative in the past.

import { bodyApi } from './api/client';
import type { WorldMetadata } from './api/types/WorldMetadata';
import type { WorldRequest } from './api/types/WorldRequest';
import { formatEpoch } from './epochs';
import { requestFromWorld } from './format';

/** Farthest epoch from the present, Ma (`plates::DRIFT_LIMIT_MA`). */
export const DRIFT_LIMIT_MA = 300;
/** Step of the slider, Ma: 61 stops, so that the previews seen are few and cached. */
export const DRIFT_STEP_MA = 10;
/** Widths of a preview (`drift::PREVIEW_WIDTHS`): coarse while dragging, finer at rest. */
export const PREVIEW_WIDTH = { coarse: 256, fine: 512 } as const;

/** `[past, future]` in Ma (negative in the past) the world's plates can be shown at, or null without plates. */
export function driftRange(meta: WorldMetadata): [number, number] | null {
  if (meta.giant || meta.preset.geology?.regime !== 'plates') return null;
  return [-DRIFT_LIMIT_MA, DRIFT_LIMIT_MA];
}

/** The slider's value nearest to `ma`, within the limits. */
export function snapDrift(ma: number): number {
  const snapped = Math.round(ma / DRIFT_STEP_MA) * DRIFT_STEP_MA;
  return Math.max(-DRIFT_LIMIT_MA, Math.min(DRIFT_LIMIT_MA, snapped));
}

/** A drift for display: "present", "150 Ma ago", "in 50 Ma". */
export function formatDrift(ma: number): string {
  return formatEpoch(-ma);
}

/** URL of the preview of a body at a drift (`…/drift/{ma}/surface.png`). */
export function driftPreviewUrl(id: string, ma: number, width: number, revision: number | null): string {
  const query = [`width=${width}`];
  if (revision !== null) query.unshift(`rev=${revision}`);
  return `${bodyApi(id)}/drift/${ma}/surface.png?${query.join('&')}`;
}

/** The request that generates the world as its plates carry it at `ma` (the present leaves `drift_ma` out). */
export function driftRequest(meta: WorldMetadata, ma: number): WorldRequest {
  const { drift_ma: _, ...request } = requestFromWorld(meta);
  return ma === 0 ? request : { ...request, drift_ma: ma };
}
