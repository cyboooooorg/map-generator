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
import type { Sibling } from './api/types/Sibling';
import type { Sky } from './sky';

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
  /**
   * Longitude of its map facing the body shown, degrees (a moon keeps
   * 180° towards its planet), or null to map it in the frame of the body
   * shown (the planet seen from a moon: it spins under it).
   */
  faceLonDeg: number | null;
  /** Its cloud map (`clouds.png`, grey), or null for none. */
  clouds?: string | null;
  /** Linear colour of its clouds. */
  cloudTint?: Vec3;
  /** Its atmosphere, seen from afar: a halo at the limb (see `halo`). */
  halo?: Halo | null;
}

/** An atmosphere seen from afar. */
export interface Halo {
  /** Linear colour of the light it scatters, brightest channel 1. */
  colour: Vec3;
  /** Height of the halo, in radii of the body. */
  thickness: number;
  /** Brightness of the halo at the limb, 0 to 1. */
  strength: number;
}

/**
 * The halo of an atmosphere (see sky.ts), null for an airless body: the
 * colour of the light its gas and aerosols scatter (their columns), and a
 * brightness growing with their optical depth.
 */
export function halo(sky: Sky | null): Halo | null {
  if (!sky) return null;
  const column = sky.betaR.map((b, i) => b * sky.heightR + sky.betaM * sky.heightM * sky.mieAlbedo[i]!) as Vec3;
  const brightest = Math.max(...column, 1e-9);
  const depth = (column[0] + column[1] + column[2]) / 3;
  return {
    colour: column.map((c) => c / brightest) as Vec3,
    thickness: sky.top - 1,
    strength: Math.min(1, 0.9 * (1 - Math.exp(-4 * depth))),
  };
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

const dot = (a: Vec3, b: Vec3) => a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
const sub = (a: Vec3, b: Vec3): Vec3 => [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
const scale = (a: Vec3, k: number): Vec3 => [a[0] * k, a[1] * k, a[2] * k];
const cross = (a: Vec3, b: Vec3): Vec3 => [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
const unit = (a: Vec3): Vec3 => scale(a, 1 / Math.max(Math.hypot(...a), 1e-12));

/**
 * The frame of moon `k` in its planet's frame: `z` away from the planet
 * (the planet at longitude 180°, as a moon keeps it), `y` along its orbit's
 * normal (its spin axis), `x` completing it; and its centre, in km.
 */
function moonFrame(
  moon: Pick<Moon, 'inclination_deg' | 'period_days' | 'migration' | 'semi_major_axis_km'>,
  k: number,
  season: number,
  yearDays: number,
  sunLongitudeRad: number,
): { x: Vec3; y: Vec3; z: Vec3; centre: Vec3 } {
  const z = moonDirection(moon, k, season, yearDays, sunLongitudeRad);
  const tilt = (moon.inclination_deg * Math.PI) / 180;
  // The normal of the orbit of moonDirection, made orthogonal to z (a
  // binary's moon hangs at [0, 0, -1] whatever its inclination).
  const normal: Vec3 = [0, Math.cos(tilt), Math.sin(tilt)];
  const y = unit(sub(normal, scale(z, dot(normal, z))));
  return { x: cross(y, z), y, z, centre: scale(z, moon.semi_major_axis_km) };
}

/** Longitude (degrees) of a moon's map facing `from` (a point, km, in its planet's frame). */
function faceLongitude(frame: ReturnType<typeof moonFrame>, from: Vec3): number {
  const u = sub(from, frame.centre);
  return (Math.atan2(dot(u, frame.x), dot(u, frame.z)) * 180) / Math.PI;
}

/**
 * The moons in the sky of their planet, and their shadows on it; each
 * mapped with its surface (`textures(k)`, null for a plain colour), its
 * face towards the planet.
 */
export function planetSky(
  moons: readonly Moon[],
  planetRadiusKm: number,
  season: number,
  yearDays: number,
  sunLongitudeRad: number,
  textures: (k: number) => string | null = () => null,
): { bodies: SkyBody[]; casters: ShadowCaster[] } {
  const bodies: SkyBody[] = [];
  const casters: ShadowCaster[] = [];
  moons.forEach((moon, k) => {
    const direction = moonDirection(moon, k, season, yearDays, sunLongitudeRad);
    bodies.push({
      direction,
      angularDiameterDeg: moon.angular_diameter_deg,
      colour: moon.composition === 'ice' ? ICE : ROCK,
      texture: textures(k),
      faceLonDeg: 180,
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
  look: Pick<SkyBody, 'clouds' | 'cloudTint' | 'halo'> = {},
): { bodies: SkyBody[]; casters: ShadowCaster[] } {
  // Longitude 180°: the face the moon keeps towards its planet.
  const direction: Vec3 = [0, 0, -1];
  const angle = 2 * Math.atan(planetRadiusKm / Math.max(moon.semi_major_axis_km - moon.radius_km, 1));
  const distance = moon.semi_major_axis_km / moon.radius_km;
  return {
    bodies: [
      {
        direction,
        angularDiameterDeg: (angle * 180) / Math.PI,
        colour: PLANET,
        texture: planetTexture,
        faceLonDeg: null,
        ...look,
      },
    ],
    casters: [{ position: [0, 0, -distance], radius: planetRadiusKm / moon.radius_km }],
  };
}

/**
 * The sky of moon `shown` (from 0) of a planet: the planet (see moonSky)
 * and the planet's other moons, where they are on their orbits as the
 * seasons play (as planetSky places them), at their size from the moon,
 * mapped with their surfaces (`textures(k)`), and the largest shadows.
 * `look` is the planet's clouds and atmosphere.
 */
export function moonSystemSky(
  moons: readonly Moon[],
  shown: number,
  planetRadiusKm: number,
  planetTexture: string | null,
  textures: (k: number) => string | null,
  season: number,
  yearDays: number,
  sunLongitudeRad: number,
  look: Pick<SkyBody, 'clouds' | 'cloudTint' | 'halo'> = {},
): { bodies: SkyBody[]; casters: ShadowCaster[] } {
  const self = moons[shown];
  if (!self) return { bodies: [], casters: [] };
  const sky = moonSky(self, planetRadiusKm, planetTexture, look);
  const here = moonFrame(self, shown, season, yearDays, sunLongitudeRad);
  // Planet frame to the shown moon's.
  const local = (v: Vec3): Vec3 => [dot(v, here.x), dot(v, here.y), dot(v, here.z)];
  moons.forEach((moon, k) => {
    if (k === shown) return;
    const there = moonFrame(moon, k, season, yearDays, sunLongitudeRad);
    const v = local(sub(there.centre, here.centre));
    const distance = Math.max(Math.hypot(...v), moon.radius_km + self.radius_km);
    sky.bodies.push({
      direction: unit(v),
      angularDiameterDeg: (2 * Math.atan(moon.radius_km / distance) * 180) / Math.PI,
      colour: moon.composition === 'ice' ? ICE : ROCK,
      texture: textures(k),
      faceLonDeg: faceLongitude(there, here.centre),
    });
    sky.casters.push({ position: scale(v, 1 / self.radius_km), radius: moon.radius_km / self.radius_km });
  });
  const casters = sky.casters.sort((a, b) => b.radius - a.radius).slice(0, MAX_CASTERS);
  return { bodies: sky.bodies, casters };
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

/**
 * The giant's other regular moons in the sky of one of them (the world
 * shown, `worldRadiusKm` in radius, at `worldDistanceKm` from the giant):
 * their phases and shadows. In the world's frame the giant hangs at
 * longitude 180°; each sibling turns around it at its period relative to
 * the world's (both prograde, in the giant's equatorial plane), from a
 * phase of its own, as the seasons play.
 */
export function siblingSky(
  siblings: readonly Pick<Sibling, 'radius_km' | 'distance_km' | 'period_days' | 'icy'>[],
  worldRadiusKm: number,
  worldDistanceKm: number,
  worldPeriodDays: number,
  season: number,
  yearDays: number,
): { bodies: SkyBody[]; casters: ShadowCaster[] } {
  const days = season * yearDays;
  const bodies: SkyBody[] = [];
  const casters: ShadowCaster[] = [];
  siblings.forEach((sibling, k) => {
    // Angle from the world's side of the giant.
    const angle =
      k * 2.1 + 1.3 + 2 * Math.PI * days * (1 / Math.max(sibling.period_days, 1e-3) - 1 / Math.max(worldPeriodDays, 1e-3));
    const v: Vec3 = [
      sibling.distance_km * Math.sin(angle),
      0,
      -worldDistanceKm + sibling.distance_km * Math.cos(angle),
    ];
    const distance = Math.hypot(...v);
    bodies.push({
      direction: v.map((c) => c / distance) as Vec3,
      angularDiameterDeg: (2 * Math.atan(sibling.radius_km / distance) * 180) / Math.PI,
      colour: sibling.icy ? ICE : ROCK,
      texture: null,
      faceLonDeg: null,
    });
    casters.push({ position: v.map((c) => c / worldRadiusKm) as Vec3, radius: sibling.radius_km / worldRadiusKm });
  });
  return { bodies, casters };
}

/** Two skies together (the giant and its other moons), the largest shadows kept. */
export function joinSkies(
  ...skies: { bodies: SkyBody[]; casters: ShadowCaster[] }[]
): { bodies: SkyBody[]; casters: ShadowCaster[] } {
  const casters = skies.flatMap((s) => s.casters).sort((a, b) => b.radius - a.radius);
  return { bodies: skies.flatMap((s) => s.bodies), casters: casters.slice(0, MAX_CASTERS) };
}
