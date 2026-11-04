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

/** A body's own x, y (spin axis) and z axes, in another body's frame. */
export type Axes = [Vec3, Vec3, Vec3];

/** The axes of the frame itself. */
export const IDENTITY_AXES: Axes = [
  [1, 0, 0],
  [0, 1, 0],
  [0, 0, 1],
];

/**
 * A body's axes turned by `angle` (radians) on its spin axis `y`, from
 * the frame's: the longitude facing a fixed direction decreases as the
 * body turns (prograde, from z towards x).
 */
export function spinAxes(angle: number): Axes {
  return [
    [Math.cos(angle), 0, -Math.sin(angle)],
    [0, 1, 0],
    [Math.sin(angle), 0, Math.cos(angle)],
  ];
}

/**
 * Rotation period of a moon, hours, or null for one locked to its planet
 * (it turns once per orbit). A captured moon keeps the spin it came with:
 * world.json does not store it, so its seed draws one, from 6 to 24 hours,
 * as the irregular moons of the giants (Himalia 7.8 h, Phoebe 9.3 h,
 * Nereid 11.6 h).
 */
export function moonSpinHours(moon: Pick<Moon, 'tidally_locked' | 'migration' | 'seed'>): number | null {
  if (moon.tidally_locked || moon.migration === 'locked') return null;
  const share = (Math.abs(Math.trunc(moon.seed)) % 9973) / 9973;
  return 6 + 18 * share;
}

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
   * Its own axes (its x, its spin axis, its z: its map's frame) in the
   * frame of the body shown, as it turns on itself; null for the frame of
   * the body shown.
   */
  axes: Axes | null;
  /** Its cloud map (`clouds.png`, grey), or null for none. */
  clouds?: string | null;
  /** Linear colour of its clouds. */
  cloudTint?: Vec3;
  /** Its atmosphere, seen from afar: a halo at the limb (see `halo`). */
  halo?: Halo | null;
  /** Axial tilt, degrees, for its tropics and polar circles. */
  axialTiltDeg?: number;
  /** What is drawn of it: all of it when absent; its grid and tropics only when asked. */
  show?: SkyBodyShow;
}

/** How the planet looks in a moon's sky: its clouds, its air, its marks and their switches. */
export type PlanetLook = Pick<SkyBody, 'clouds' | 'cloudTint' | 'halo' | 'axialTiltDeg' | 'show'>;

/** The display switches of a body in the sky (its own, not the body shown's). */
export interface SkyBodyShow {
  atmosphere: boolean;
  clouds: boolean;
  grid: boolean;
  tropics: boolean;
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

/** What places a moon on its orbit. */
type OrbitingMoon = Pick<
  Moon,
  'inclination_deg' | 'period_days' | 'migration' | 'semi_major_axis_km' | 'eccentricity' | 'seed'
>;

/**
 * Longitude of a moon's periapsis on its orbit, radians: world.json does
 * not store it, so its seed draws it.
 */
export function periapsisLongitude(moon: Pick<Moon, 'seed'>): number {
  return ((Math.abs(Math.trunc(moon.seed)) % 7919) / 7919) * 2 * Math.PI;
}

/** A point of an orbit at longitude `angle` (radians) on it, as a unit vector of the planet's frame. */
function onOrbit(angle: number, tilt: number): Vec3 {
  return [Math.sin(angle), -Math.cos(angle) * Math.sin(tilt), Math.cos(angle) * Math.cos(tilt)];
}

/**
 * Where a moon is, from its planet's centre, in the planet's frame: its
 * direction and its distance (km). Its mean longitude is opposite the
 * sun's when the season starts (full, but for its eccentricity), the next
 * moons spread round their orbits, each going round at its period
 * relative to the year as the seasons turn; on its ellipse (Kepler's
 * equation), nearer and faster at periapsis. A binary's moon stays put.
 */
export function moonPlace(
  moon: OrbitingMoon,
  index: number,
  season: number,
  yearDays: number,
  sunLongitudeRad: number,
): { direction: Vec3; distanceKm: number } {
  // A binary: the planet turns with its moon, which hangs over the same
  // point, at longitude 180° (as a moon keeps its planet there).
  if (moon.migration === 'locked') {
    return { direction: [0, 0, -1], distanceKm: moon.semi_major_axis_km };
  }
  const turns = (season * yearDays) / Math.max(moon.period_days, 1e-3);
  const meanLongitude = sunLongitudeRad + Math.PI + index * 2.4 + 2 * Math.PI * (turns - Math.floor(turns));
  const e = Math.min(Math.max(moon.eccentricity, 0), 0.95);
  const periapsis = periapsisLongitude(moon);
  // Kepler's equation, E - e sin E = M, by Newton's method.
  const mean = meanLongitude - periapsis;
  let eccentric = e < 0.8 ? mean : Math.PI;
  for (let i = 0; i < 8; i += 1) {
    eccentric -= (eccentric - e * Math.sin(eccentric) - mean) / (1 - e * Math.cos(eccentric));
  }
  const trueAnomaly =
    2 * Math.atan2(Math.sqrt(1 + e) * Math.sin(eccentric / 2), Math.sqrt(1 - e) * Math.cos(eccentric / 2));
  const tilt = (moon.inclination_deg * Math.PI) / 180;
  return {
    direction: onOrbit(periapsis + trueAnomaly, tilt),
    distanceKm: moon.semi_major_axis_km * (1 - e * Math.cos(eccentric)),
  };
}

/** The direction of a moon from its planet's centre (see moonPlace). */
export function moonDirection(
  moon: OrbitingMoon,
  index: number,
  season: number,
  yearDays: number,
  sunLongitudeRad: number,
): Vec3 {
  return moonPlace(moon, index, season, yearDays, sunLongitudeRad).direction;
}

const dot = (a: Vec3, b: Vec3) => a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
const sub = (a: Vec3, b: Vec3): Vec3 => [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
const scale = (a: Vec3, k: number): Vec3 => [a[0] * k, a[1] * k, a[2] * k];
const cross = (a: Vec3, b: Vec3): Vec3 => [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
const unit = (a: Vec3): Vec3 => scale(a, 1 / Math.max(Math.hypot(...a), 1e-12));

/** What places a moon and turns it on itself. */
type PlacedMoon = OrbitingMoon & Pick<Moon, 'tidally_locked'>;

/**
 * The frame of moon `k` in its planet's frame, its own (its map's): `y`
 * along its orbit's normal (its spin axis); `z` away from the planet (the
 * planet at longitude 180°, as a locked moon keeps it), or, on a moon that
 * is not locked, turned from there as it spins (see moonSpinHours); `x`
 * completing it; and its centre, in km.
 */
function moonFrame(
  moon: PlacedMoon,
  k: number,
  season: number,
  yearDays: number,
  sunLongitudeRad: number,
): { x: Vec3; y: Vec3; z: Vec3; centre: Vec3 } {
  const { direction: z, distanceKm } = moonPlace(moon, k, season, yearDays, sunLongitudeRad);
  const tilt = (moon.inclination_deg * Math.PI) / 180;
  // The normal of the orbit of moonDirection, made orthogonal to z (a
  // binary's moon hangs at [0, 0, -1] whatever its inclination).
  const normal: Vec3 = [0, Math.cos(tilt), Math.sin(tilt)];
  const y = unit(sub(normal, scale(z, dot(normal, z))));
  const x = cross(y, z);
  const centre = scale(z, distanceKm);
  const spin = moonSpinHours(moon);
  if (spin === null) return { x, y, z, centre };
  // Turned on itself against the orbit's frame, which turns once per orbit.
  const days = season * yearDays;
  const turns = days * (24 / spin - 1 / Math.max(moon.period_days, 1e-3));
  const a = 2 * Math.PI * (turns - Math.floor(turns));
  const c = Math.cos(a);
  const s = Math.sin(a);
  return {
    x: sub(scale(x, c), scale(z, s)),
    y,
    z: [z[0] * c + x[0] * s, z[1] * c + x[1] * s, z[2] * c + x[2] * s],
    centre,
  };
}

/** A frame's axes in the frame given by `local`. */
function axesIn(frame: { x: Vec3; y: Vec3; z: Vec3 }, local: (v: Vec3) => Vec3): Axes {
  return [local(frame.x), local(frame.y), local(frame.z)];
}

/**
 * Longitude (degrees, 0 to 360) over which the sun stands on moon `k`
 * (from 0), as its planet's sky places the moons (see moonDirection): it
 * turns once per orbit on a moon keeping its face to its planet, once per
 * day of its own on one that does not (see moonSpinHours). The sun
 * is taken in its planet's equatorial plane.
 */
export function sunLongitudeOnMoon(
  moon: PlacedMoon,
  k: number,
  season: number,
  yearDays: number,
  sunLongitudeRad: number,
): number {
  const frame = moonFrame(moon, k, season, yearDays, sunLongitudeRad);
  const sun: Vec3 = [Math.sin(sunLongitudeRad), 0, Math.cos(sunLongitudeRad)];
  const degrees = (Math.atan2(dot(sun, frame.x), dot(sun, frame.z)) * 180) / Math.PI;
  return (degrees + 360) % 360;
}

/** Points sampled round each orbit line. */
const ORBIT_SAMPLES = 160;

/** The great circle of a frame's x-z plane, as an orbit line. */
function circle(): Vec3[] {
  return Array.from({ length: ORBIT_SAMPLES }, (_, i) => {
    const a = (2 * Math.PI * i) / ORBIT_SAMPLES;
    return [Math.sin(a), 0, Math.cos(a)] as Vec3;
  });
}

/**
 * The orbits of a planet's moons as lines in the sky, at infinity: for
 * each moon, directions to points of its orbit seen from the planet's
 * centre, or, with `shown` (from 0), from that moon's centre in its frame
 * (as moonSystemSky places the bodies). A binary's moon, which does not
 * move in its planet's sky, has none from the planet. The moon shown sees
 * its own orbit as a great circle through its planet, the way it goes.
 */
export function orbitLoops(
  moons: readonly Moon[],
  shown: number | null,
  season: number,
  yearDays: number,
  sunLongitudeRad: number,
): Vec3[][] {
  const self = shown === null ? null : moons[shown];
  const here =
    self && shown !== null
      ? moonFrame(self, shown, season, yearDays, sunLongitudeRad)
      : null;
  const loops: Vec3[][] = [];
  moons.forEach((moon, k) => {
    if (k === shown && here) {
      // Its orbit's plane, through the moon's centre, is its frame's x-z
      // plane (y is the orbit's normal).
      loops.push(circle());
      return;
    }
    if (moon.migration === "locked") return;
    const tilt = (moon.inclination_deg * Math.PI) / 180;
    const e = Math.min(Math.max(moon.eccentricity, 0), 0.95);
    const periapsis = periapsisLongitude(moon);
    const loop: Vec3[] = [];
    for (let i = 0; i < ORBIT_SAMPLES; i += 1) {
      const nu = (2 * Math.PI * i) / ORBIT_SAMPLES;
      // moonPlace's ellipse, in km.
      const p = scale(
        onOrbit(periapsis + nu, tilt),
        (moon.semi_major_axis_km * (1 - e * e)) / (1 + e * Math.cos(nu)),
      );
      if (!here) {
        loop.push(unit(p));
      } else {
        const v = sub(p, here.centre);
        loop.push(unit([dot(v, here.x), dot(v, here.y), dot(v, here.z)]));
      }
    }
    loops.push(loop);
  });
  return loops;
}

/**
 * The orbits of a giant's other regular moons in the sky of one of them
 * (see siblingSky): circles in the giant's equatorial plane round the
 * giant, at longitude 180°, `worldDistanceKm` away.
 */
export function siblingOrbitLoops(
  siblings: readonly Pick<Sibling, "distance_km">[],
  worldDistanceKm: number,
): Vec3[][] {
  return siblings.map((sibling) => {
    const loop: Vec3[] = [];
    for (let i = 0; i < ORBIT_SAMPLES; i += 1) {
      const angle = (2 * Math.PI * i) / ORBIT_SAMPLES;
      loop.push(
        unit([
          sibling.distance_km * Math.sin(angle),
          0,
          -worldDistanceKm + sibling.distance_km * Math.cos(angle),
        ]),
      );
    }
    return loop;
  });
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
    const { direction, distanceKm } = moonPlace(moon, k, season, yearDays, sunLongitudeRad);
    bodies.push({
      direction,
      // Its apparent size at its mean distance, larger near periapsis.
      angularDiameterDeg: (moon.angular_diameter_deg * moon.semi_major_axis_km) / distanceKm,
      colour: moon.composition === 'ice' ? ICE : ROCK,
      texture: textures(k),
      axes: axesIn(moonFrame(moon, k, season, yearDays, sunLongitudeRad), (v) => v),
    });
    const distance = distanceKm / planetRadiusKm;
    casters.push({ position: direction.map((c) => c * distance) as Vec3, radius: moon.radius_km / planetRadiusKm });
  });
  // The largest shadows first, when there are more moons than casters.
  casters.sort((a, b) => b.radius - a.radius);
  return { bodies, casters: casters.slice(0, MAX_CASTERS) };
}

/**
 * The planet in the sky of one of its moons, and its shadow on the moon:
 * towards `direction` (by default longitude 180°, the face a locked moon
 * keeps towards its planet), its own axes `axes` as it turns.
 */
export function moonSky(
  moon: Pick<Moon, 'radius_km' | 'semi_major_axis_km'>,
  planetRadiusKm: number,
  planetTexture: string | null,
  look: PlanetLook = {},
  direction: Vec3 = [0, 0, -1],
  axes: Axes = IDENTITY_AXES,
): { bodies: SkyBody[]; casters: ShadowCaster[] } {
  const angle = 2 * Math.atan(planetRadiusKm / Math.max(moon.semi_major_axis_km - moon.radius_km, 1));
  const distance = moon.semi_major_axis_km / moon.radius_km;
  return {
    bodies: [
      {
        direction,
        angularDiameterDeg: (angle * 180) / Math.PI,
        colour: PLANET,
        texture: planetTexture,
        axes,
        ...look,
      },
    ],
    casters: [{ position: scale(direction, distance), radius: planetRadiusKm / moon.radius_km }],
  };
}

/**
 * The sky of moon `shown` (from 0) of a planet: the planet (see moonSky)
 * and the planet's other moons, where they are on their orbits as the
 * seasons play (as planetSky places them), at their size from the moon,
 * mapped with their surfaces (`textures(k)`), and the largest shadows.
 * `look` is the planet's clouds and atmosphere; `planetTurn` (radians) how
 * far the planet has turned on itself since the hour of its own view (see
 * bodyTurn), which shows it spinning in the sky.
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
  look: PlanetLook = {},
  planetTurn = 0,
): { bodies: SkyBody[]; casters: ShadowCaster[] } {
  const self = moons[shown];
  if (!self) return { bodies: [], casters: [] };
  const here = moonFrame(self, shown, season, yearDays, sunLongitudeRad);
  // Planet frame to the shown moon's.
  const local = (v: Vec3): Vec3 => [dot(v, here.x), dot(v, here.y), dot(v, here.z)];
  const [px, py, pz] = spinAxes(planetTurn);
  // The planet from where the moon is on its ellipse.
  const sky = moonSky(
    { radius_km: self.radius_km, semi_major_axis_km: Math.hypot(...here.centre) },
    planetRadiusKm,
    planetTexture,
    look,
    unit(local(scale(here.centre, -1))),
    axesIn({ x: px, y: py, z: pz }, local),
  );
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
      axes: axesIn(there, local),
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
 * phase of its own, as the seasons play; each mapped with its surface
 * (`textures(k)`: its own world in `host/siblings/`), the face it keeps
 * towards the giant (longitude 180°) turned to it.
 */
export function siblingSky(
  siblings: readonly Pick<Sibling, 'radius_km' | 'distance_km' | 'period_days' | 'icy'>[],
  worldRadiusKm: number,
  worldDistanceKm: number,
  worldPeriodDays: number,
  season: number,
  yearDays: number,
  textures: (k: number) => string | null = () => null,
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
      texture: textures(k),
      // Locked to the giant: its z away from it, its spin axis the giant's.
      axes: (() => {
        const z = unit(sub(v, [0, 0, -worldDistanceKm]));
        const y: Vec3 = [0, 1, 0];
        return [cross(y, z), y, z] as Axes;
      })(),
    });
    casters.push({ position: v.map((c) => c / worldRadiusKm) as Vec3, radius: sibling.radius_km / worldRadiusKm });
  });
  return { bodies, casters };
}

/**
 * How far a body has turned on itself, radians, after `days` against a
 * frame that turns once per `framePeriodDays` (its orbit round its star,
 * as the sun stands still in its own view, or its moon's orbit), at one
 * turn per `dayHours`; 0 for a body locked to that frame.
 */
export function bodyTurn(days: number, dayHours: number, framePeriodDays: number): number {
  const turns = days * (24 / Math.max(dayHours, 1e-3) - 1 / Math.max(framePeriodDays, 1e-3));
  return 2 * Math.PI * (turns - Math.floor(turns));
}

/** Two skies together (the giant and its other moons), the largest shadows kept. */
export function joinSkies(
  ...skies: { bodies: SkyBody[]; casters: ShadowCaster[] }[]
): { bodies: SkyBody[]; casters: ShadowCaster[] } {
  const casters = skies.flatMap((s) => s.casters).sort((a, b) => b.radius - a.radius);
  return { bodies: skies.flatMap((s) => s.bodies), casters: casters.slice(0, MAX_CASTERS) };
}
