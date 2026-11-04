// Error messages in the active locale: the server's reasons, translated
// from their code when it sent one, its English text otherwise.
import { ApiFailure } from '../api/client';
import type { ErrorDetail } from '../api/types/ErrorDetail';
import { formatInt, formatUpTo } from '../format';
import { t } from './index.svelte';

/** The message of a server error detail. */
export function detailText(detail: ErrorDetail): string {
  const field = (name: string) => t.errors.fields[name] ?? name;
  switch (detail.code) {
    case 'out_of_range':
      return t.errors.outOfRange({ field: field(detail.field), min: formatUpTo(detail.min, 3), max: formatUpTo(detail.max, 3) });
    case 'size_out_of_range':
      return t.errors.sizeOutOfRange({ ...detail, field: field(detail.field) });
    case 'sea_level_and_land_fraction':
      return t.errors.seaLevelAndLandFraction;
    case 'unknown_preset':
      return t.errors.unknownPreset({ preset: detail.preset });
    case 'too_many_pixels':
      return t.errors.tooManyPixels({ width: detail.width, height: detail.height, max: formatInt(detail.max_pixels) });
    case 'being_generated':
      return t.errors.beingGenerated({ world: detail.world });
    case 'world_exists':
      return t.errors.worldExists({ world: detail.world });
    case 'busy':
      return t.errors.busy;
    case 'unknown_star':
      return t.errors.unknownStar({ star: detail.star });
    case 'orbit_and_position':
      return t.errors.orbitAndPosition;
    case 'no_room_for_moons':
      return t.errors.noRoomForMoons({ count: detail.count });
  }
}

/** What to tell the user about a failed call or any other error. */
export function errorText(error: unknown): string {
  if (error instanceof ApiFailure) {
    return error.detail ? detailText(error.detail) : error.message;
  }
  return String(error);
}
