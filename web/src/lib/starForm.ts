// The star and orbit of the generation form: what the user picked, where
// it starts from (the world being varied, the preset, the server's
// fallback) and how it goes into a request.
import type { ParamDefault } from './api/types/ParamDefault';
import type { PresetInfo } from './api/types/PresetInfo';
import type { WorldRequest } from './api/types/WorldRequest';

/** Spectral classes, hottest first (main-sequence dwarfs, O3V to M9.5V). */
export const STAR_CLASSES = ['O', 'B', 'A', 'F', 'G', 'K', 'M'] as const;

/** The orbit is set by its position in the habitable zone, or by its distance. */
export type OrbitBy = 'position' | 'au';

/** The star and orbit as the form holds them. */
export interface StarChoice {
  /** Spectral class, or '' to let the preset or the server draw the star. */
  klass: string;
  /** Subtype, 0 to 9.5. */
  subtype: number;
  orbitBy: OrbitBy;
  /** Whether the orbit is left to the preset or the server. */
  orbitRandom: boolean;
  /** Position in the habitable zone (0 inner edge, 1 outer edge). */
  position: number;
  /** Distance, AU. */
  au: number;
}

/** A spectral type split into its class and subtype (`G2V`, `m4.5`, `K`), or null. */
export function parseStarType(text: string): { klass: string; subtype: number } | null {
  const match = /^\s*([OBAFGKM])(\d(?:\.\d)?)?V?\s*$/i.exec(text);
  if (!match) {
    return null;
  }
  const klass = match[1]!.toUpperCase();
  const subtype = match[2] === undefined ? 5 : Number(match[2]);
  if (klass === 'O' && subtype < 3) {
    return null;
  }
  return { klass, subtype };
}

/** The spectral type of a class and subtype: `G2V`, `M4.5V`. */
export function starType(klass: string, subtype: number): string {
  return `${klass}${subtype}V`;
}

function middle(d: ParamDefault): number {
  return typeof d === 'number' ? d : (d[0] + d[1]) / 2;
}

/**
 * Where the form starts: the varied world's star and orbit, else the
 * preset's, else a drawn star in the server's fallback range.
 */
export function initialStarChoice(
  preset: PresetInfo | undefined,
  from: WorldRequest | null,
  fallbackPosition: ParamDefault,
): StarChoice {
  const type = parseStarType(from?.star ?? preset?.star.type ?? '');
  const choice: StarChoice = {
    klass: type?.klass ?? '',
    subtype: type?.subtype ?? 2,
    orbitBy: 'position',
    orbitRandom: true,
    position: middle(preset?.star.position ?? fallbackPosition),
    au: 1,
  };
  if (from?.orbit_au != null) {
    return { ...choice, orbitBy: 'au', orbitRandom: false, au: from.orbit_au };
  }
  if (from?.hz_position != null) {
    return { ...choice, orbitRandom: false, position: from.hz_position };
  }
  // Absent fields may come as null from older servers or stored worlds.
  const presetAu = preset?.star.orbit_au;
  if (presetAu != null) {
    return { ...choice, orbitBy: 'au', orbitRandom: false, au: presetAu };
  }
  const presetPosition = preset?.star.position;
  if (typeof presetPosition === 'number') {
    return { ...choice, orbitRandom: false, position: presetPosition };
  }
  return choice;
}

/** Adds the star and orbit the user fixed to a request. */
export function applyStarChoice(request: WorldRequest, choice: StarChoice): void {
  if (choice.klass !== '') {
    request.star = starType(choice.klass, choice.subtype);
  }
  if (!choice.orbitRandom) {
    if (choice.orbitBy === 'au') {
      request.orbit_au = choice.au;
    } else {
      request.hz_position = choice.position;
    }
  }
}

/** A remembered choice, or null when it is not well formed. */
export function parseStarChoice(raw: unknown): StarChoice | null {
  if (typeof raw !== 'object' || raw === null) {
    return null;
  }
  const r = raw as Record<string, unknown>;
  const finite = (v: unknown): v is number => typeof v === 'number' && Number.isFinite(v);
  const klassOk = r.klass === '' || STAR_CLASSES.includes(r.klass as (typeof STAR_CLASSES)[number]);
  if (
    !klassOk ||
    !finite(r.subtype) ||
    (r.orbitBy !== 'position' && r.orbitBy !== 'au') ||
    typeof r.orbitRandom !== 'boolean' ||
    !finite(r.position) ||
    !finite(r.au)
  ) {
    return null;
  }
  return {
    klass: r.klass as string,
    subtype: r.subtype,
    orbitBy: r.orbitBy,
    orbitRandom: r.orbitRandom,
    position: r.position,
    au: r.au,
  };
}
