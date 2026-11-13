// The star and orbit of the generation form: what the user picked, where
// it starts from (the world being varied, the archetype, the server's
// fallback) and how it goes into a request.
import type { ParamDefault } from './api/types/ParamDefault';
import type { ArchetypeInfo } from './api/types/ArchetypeInfo';
import type { WorldRequest } from './api/types/WorldRequest';

/** Spectral classes, hottest first (main-sequence dwarfs, O3V to M9.5V). */
export const STAR_CLASSES = ['O', 'B', 'A', 'F', 'G', 'K', 'M'] as const;

/** The orbit is set by its position in the habitable zone, or by its distance. */
export type OrbitBy = 'position' | 'au';

/** The star and orbit as the form holds them. */
export interface StarChoice {
  /** Spectral class, or '' to let the archetype or the server draw the star. */
  klass: string;
  /** Subtype, 0 to 9.5. */
  subtype: number;
  orbitBy: OrbitBy;
  /** Whether the orbit is left to the archetype or the server. */
  orbitRandom: boolean;
  /** Position in the habitable zone (0 inner edge, 1 outer edge). */
  position: number;
  /** Distance, AU. */
  au: number;
  /** Whether the eccentricity is left to the archetype or the server. */
  eccentricityRandom: boolean;
  /** Eccentricity of the orbit, 0 to 0.9. */
  eccentricity: number;
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
 * Luminosity of a G2V star, L☉ (log L = 0.01 in the generator's star table,
 * `star.rs`): the only star an archetype names today, so the only one whose
 * insolation the form converts to a distance itself.
 */
const G2V_LUMINOSITY = 10 ** 0.01;

/**
 * The distance, AU, at which the archetype receives its insolation (the
 * middle of its range) round its named star, as the server places it:
 * `a = sqrt(L / S)`. Null when the archetype names no star, sets no
 * insolation (a hot Jupiter sits at its period) or names a star whose
 * luminosity the form does not know.
 */
function archetypeOrbitAu(archetype: ArchetypeInfo | undefined): number | null {
  const insolation = archetype?.orbit.insolation;
  const type = parseStarType(archetype?.orbit.star ?? '');
  if (insolation == null || type === null || starType(type.klass, type.subtype) !== 'G2V') {
    return null;
  }
  return Math.sqrt(G2V_LUMINOSITY / middle(insolation));
}

/**
 * Where the form starts: the varied world's star and orbit, else the
 * archetype's, else a drawn star in the server's fallback range.
 */
export function initialStarChoice(
  archetype: ArchetypeInfo | undefined,
  from: WorldRequest | null,
  fallbackPosition: ParamDefault,
): StarChoice {
  const type = parseStarType(from?.star ?? archetype?.orbit.star ?? '');
  const choice: StarChoice = {
    klass: type?.klass ?? '',
    subtype: type?.subtype ?? 2,
    orbitBy: 'position',
    orbitRandom: true,
    position: middle(fallbackPosition),
    au: 1,
    ...initialEccentricity(archetype, from),
  };
  if (from?.orbit_au != null) {
    return { ...choice, orbitBy: 'au', orbitRandom: false, au: from.orbit_au };
  }
  if (from?.hz_position != null) {
    return { ...choice, orbitRandom: false, position: from.hz_position };
  }
  const archetypeAu = from?.star === undefined || from.star === archetype?.orbit.star ? archetypeOrbitAu(archetype) : null;
  if (archetypeAu !== null) {
    return { ...choice, orbitBy: 'au', orbitRandom: false, au: Number(archetypeAu.toFixed(4)) };
  }
  return choice;
}

/** The eccentricity the form starts from: the varied world's, else the archetype's (fixed or the middle of its range). */
function initialEccentricity(
  archetype: ArchetypeInfo | undefined,
  from: WorldRequest | null,
): Pick<StarChoice, 'eccentricityRandom' | 'eccentricity'> {
  if (from?.eccentricity != null) {
    return { eccentricityRandom: false, eccentricity: from.eccentricity };
  }
  const d = archetype?.orbit.eccentricity;
  if (typeof d === 'number') {
    return { eccentricityRandom: false, eccentricity: d };
  }
  return { eccentricityRandom: true, eccentricity: d ? middle(d) : 0.1 };
}

/** Adds the star and orbit the user fixed to a request. */
export function applyStarChoice(request: WorldRequest, choice: StarChoice): void {
  if (!choice.eccentricityRandom) {
    request.eccentricity = choice.eccentricity;
  }
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
    // Forms remembered before eccentricities leave it random.
    eccentricityRandom: typeof r.eccentricityRandom === 'boolean' ? r.eccentricityRandom : true,
    eccentricity: finite(r.eccentricity) ? r.eccentricity : 0.1,
  };
}
