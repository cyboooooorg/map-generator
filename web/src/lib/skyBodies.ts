// The other bodies of a world's system as the planet view shows them: in
// the sky, with their phase (the planet seen from one of its moons, the
// moons seen from their planet), and as shadow casters for eclipses.
//
// Directions are in the frame of the body shown, as the viewer's sphere is
// (`[cos lat sin lon, sin lat, cos lat cos lon]`, y along the spin axis);
// positions and radii are in radii of that body. The moons orbit in their
// planet's equatorial plane tilted by their inclination; a moon keeps its
// long axis towards its planet, at longitude 180°.
import type { Moon } from './api/types/Moon';

export type Vec3 = [number, number, number];

/** A body drawn in the sky of the body shown. */
export interface SkyBody {
  direction: Vec3;
  /** Apparent diameter, degrees. */
  angularDiameterDeg: number;
  /** Colour of its surface, linear RGB, when it has no map. */
  colour: Vec3;
  /** Its surface map (`surface.png`), or null for a plain colour. */
  texture: string | null;
}

/** A body whose shadow can fall on the body shown (an eclipse). */
export interface ShadowCaster {
  /** Its centre, in radii of the body shown. */
  position: Vec3;
  /** Its radius, in radii of the body shown. */
  radius: number;
}

/** Most shadow casters the surface shader takes. */
export const MAX_CASTERS = 4;

const ROCK: Vec3 = [0.32, 0.3, 0.28];
const ICE: Vec3 = [0.72, 0.76, 0.8];
/** The planet's colour seen from a moon before its map has loaded. */
const PLANET: Vec3 = [0.35, 0.45, 0.6];

/**
 * Direction of a moon from its planet's centre, in the planet's frame:
 * full when the season starts (opposite the sun's longitude), the next
 * ones spread around the orbit, each going round at its period relative
 * to the year as the seasons turn; a binary's moon stays put.
 */
export function moonDirection(
  moon: Pick<Moon, 'inclination_deg' | 'period_days' | 'migration'>,
  index: number,
  season: number,
  yearDays: number,
  sunLongitudeRad: number,
): Vec3 {
  // A binary: the planet turns with its moon, which hangs over the same
  // point, at longitude 180° (as a moon keeps its planet there).
  if (moon.migration === 'locked') {
    return [0, 0, -1];
  }
  const turns = (season * yearDays) / Math.max(moon.period_days, 1e-3);
  const theta = sunLongitudeRad + Math.PI + index * 2.4 + 2 * Math.PI * turns;
  const tilt = (moon.inclination_deg * Math.PI) / 180;
  return [Math.sin(theta), -Math.cos(theta) * Math.sin(tilt), Math.cos(theta) * Math.cos(tilt)];
}

/** The moons in the sky of their planet, and their shadows on it. */
export function planetSky(
  moons: readonly Moon[],
  planetRadiusKm: number,
  season: number,
  yearDays: number,
  sunLongitudeRad: number,
): { bodies: SkyBody[]; casters: ShadowCaster[] } {
  const bodies: SkyBody[] = [];
  const casters: ShadowCaster[] = [];
  moons.forEach((moon, k) => {
    const direction = moonDirection(moon, k, season, yearDays, sunLongitudeRad);
    bodies.push({
      direction,
      angularDiameterDeg: moon.angular_diameter_deg,
      colour: moon.composition === 'ice' ? ICE : ROCK,
      texture: null,
    });
    const distance = moon.semi_major_axis_km / planetRadiusKm;
    casters.push({ position: direction.map((c) => c * distance) as Vec3, radius: moon.radius_km / planetRadiusKm });
  });
  // The largest shadows first, when there are more moons than casters.
  casters.sort((a, b) => b.radius - a.radius);
  return { bodies, casters: casters.slice(0, MAX_CASTERS) };
}

/** The planet in the sky of one of its moons, and its shadow on the moon. */
export function moonSky(
  moon: Pick<Moon, 'radius_km' | 'semi_major_axis_km'>,
  planetRadiusKm: number,
  planetTexture: string | null,
): { bodies: SkyBody[]; casters: ShadowCaster[] } {
  // Longitude 180°: the face the moon keeps towards its planet.
  const direction: Vec3 = [0, 0, -1];
  const angle = 2 * Math.atan(planetRadiusKm / Math.max(moon.semi_major_axis_km - moon.radius_km, 1));
  const distance = moon.semi_major_axis_km / moon.radius_km;
  return {
    bodies: [{ direction, angularDiameterDeg: (angle * 180) / Math.PI, colour: PLANET, texture: planetTexture }],
    casters: [{ position: [0, 0, -distance], radius: planetRadiusKm / moon.radius_km }],
  };
}

/**
 * Share of the sunlight a shadow caster hides at a point of the unit
 * sphere (0 to 1), the sun of angular radius `sunRadius` (radians) in
 * direction `sun`: as the surface shader computes it (umbra inside, a
 * penumbra growing with the distance behind the caster).
 */
export function eclipse(point: Vec3, sun: Vec3, caster: ShadowCaster, sunRadius: number): number {
  const c: Vec3 = [caster.position[0] - point[0], caster.position[1] - point[1], caster.position[2] - point[2]];
  const t = c[0] * sun[0] + c[1] * sun[1] + c[2] * sun[2];
  if (t <= 0) return 0;
  const d = Math.hypot(c[0] - t * sun[0], c[1] - t * sun[1], c[2] - t * sun[2]);
  const spread = t * Math.tan(sunRadius);
  const umbra = Math.max(caster.radius - spread, 0);
  const penumbra = caster.radius + spread;
  // Smoothstep from the penumbra's edge to the umbra's (or the centre).
  const x = Math.min(Math.max((penumbra - d) / Math.max(penumbra - umbra, 1e-6), 0), 1);
  const covered = x * x * (3 - 2 * x);
  // An annular eclipse never hides the whole sun.
  const most = caster.radius >= spread ? 1 : (caster.radius / spread) ** 2;
  return covered * most;
}
