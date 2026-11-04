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
  /** Apparent diameter from the centre of the body shown, degrees. */
  angularDiameterDeg: number;
  /**
   * Its centre and radius, in radii of the body shown, when it is near
   * enough for the view to place it from the camera rather than at
   * infinity from the centre (where it would slip off its orbit line as
   * the camera turns round the globe).
   */
  position?: Vec3;
  radius?: number;
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
 * Angle of a moon's periapsis past its orbit's node, radians: world.json
 * does not store it, so its seed draws it.
 */
export function periapsisLongitude(moon: Pick<Moon, 'seed'>): number {
  return ((Math.abs(Math.trunc(moon.seed)) % 7919) / 7919) * 2 * Math.PI;
}

/**
 * Where a moon's orbit lies in its planet's frame: the turn of its node
 * line (radians, 0 along the frame's x axis) and the longitude of its
 * periapsis. Both are fixed among the stars, the node drawn by the seed
 * as the periapsis is: the planet's frame is the planet at the hour of
 * its view (its star at `sunLongitudeRad`), which turns against the stars
 * once a year, so they turn back in it as the seasons go (and with the
 * hour). An inclined moon thus sees the sun climb and fall over its
 * equator through the year.
 */
function orbitOrientation(
  moon: Pick<Moon, 'seed'>,
  season: number,
  sunLongitudeRad: number,
): { node: number; periapsis: number } {
  const node = ((Math.abs(Math.trunc(moon.seed)) % 6421) / 6421) * 2 * Math.PI;
  const turned = node - 2 * Math.PI * season + sunLongitudeRad;
  return { node: turned, periapsis: turned + periapsisLongitude(moon) };
}

/** A vector turned by `angle` (radians) about the frame's y axis, from z towards x. */
function turnY(v: Vec3, angle: number): Vec3 {
  const c = Math.cos(angle);
  const s = Math.sin(angle);
  return [v[0] * c + v[2] * s, v[1], v[2] * c - v[0] * s];
}

/**
 * A point of an orbit tilted by `tilt` about its node line (turned by
 * `node`) at longitude `angle` (radians) on it, as a unit vector of the
 * planet's frame.
 */
function onOrbit(angle: number, tilt: number, node: number): Vec3 {
  const a = angle - node;
  return turnY([Math.sin(a), -Math.cos(a) * Math.sin(tilt), Math.cos(a) * Math.cos(tilt)], node);
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
): { direction: Vec3; distanceKm: number; trueAnomaly: number } {
  // A binary: the planet turns with its moon, which hangs over the same
  // point, at longitude 180° (as a moon keeps its planet there).
  if (moon.migration === 'locked') {
    return { direction: [0, 0, -1], distanceKm: moon.semi_major_axis_km, trueAnomaly: 0 };
  }
  const turns = (season * yearDays) / Math.max(moon.period_days, 1e-3);
  const meanLongitude = sunLongitudeRad + Math.PI + index * 2.4 + 2 * Math.PI * (turns - Math.floor(turns));
  const e = Math.min(Math.max(moon.eccentricity, 0), 0.95);
  const { node, periapsis } = orbitOrientation(moon, season, sunLongitudeRad);
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
    direction: onOrbit(periapsis + trueAnomaly, tilt, node),
    distanceKm: moon.semi_major_axis_km * (1 - e * Math.cos(eccentric)),
    trueAnomaly,
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
  const normal = turnY([0, Math.cos(tilt), Math.sin(tilt)], orbitOrientation(moon, season, sunLongitudeRad).node);
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
 * Where the sun stands over moon `k` (from 0): the longitude (degrees, 0
 * to 360) and latitude (degrees) under it, as its planet's sky places the
 * moons (see moonPlace). The longitude turns once per orbit on a moon
 * keeping its face to its planet, once per day of its own on one that
 * does not (see moonSpinHours). The sun is where the planet's view has
 * it: at longitude `sunLongitudeRad` over the planet, its latitude from
 * the planet's axial tilt `planetTiltDeg` and the season, as the planet
 * view lights it; the moon's orbit, inclined to the planet's equator,
 * adds or takes off its inclination.
 */
export function sunOnMoon(
  moon: PlacedMoon,
  k: number,
  season: number,
  yearDays: number,
  sunLongitudeRad: number,
  planetTiltDeg = 0,
): { longitudeDeg: number; latitudeDeg: number } {
  const frame = moonFrame(moon, k, season, yearDays, sunLongitudeRad);
  const declination = ((planetTiltDeg * Math.PI) / 180) * Math.sin(2 * Math.PI * season);
  const sun: Vec3 = [
    Math.cos(declination) * Math.sin(sunLongitudeRad),
    Math.sin(declination),
    Math.cos(declination) * Math.cos(sunLongitudeRad),
  ];
  const degrees = (Math.atan2(dot(sun, frame.x), dot(sun, frame.z)) * 180) / Math.PI;
  return {
    longitudeDeg: (degrees + 360) % 360,
    latitudeDeg: (Math.asin(Math.min(Math.max(dot(sun, frame.y), -1), 1)) * 180) / Math.PI,
  };
}

/** The longitude under the sun on moon `k`, degrees (see sunOnMoon). */
export function sunLongitudeOnMoon(
  moon: PlacedMoon,
  k: number,
  season: number,
  yearDays: number,
  sunLongitudeRad: number,
): number {
  return sunOnMoon(moon, k, season, yearDays, sunLongitudeRad).longitudeDeg;
}

/** Points sampled round each orbit line. */
const ORBIT_SAMPLES = 160;

/**
 * True anomalies to sample an orbit at: evenly round it, and, with
 * `near` (the place of a body on it and its radius, as a share of the
 * orbit's size), ever closer towards that place, so that the line passes
 * through the body however close the camera is to it.
 */
function anomalies(near: { at: number; size: number } | null): number[] {
  const even = Array.from({ length: ORBIT_SAMPLES }, (_, i) => (2 * Math.PI * i) / ORBIT_SAMPLES);
  if (!near) return even;
  const close: number[] = [near.at];
  for (let step = 0.2 * near.size; step < (2 * Math.PI) / ORBIT_SAMPLES; step *= 1.4) {
    close.push(near.at + step, near.at - step);
  }
  const wrap = (a: number) => a - 2 * Math.PI * Math.floor(a / (2 * Math.PI));
  return [...even, ...close].map(wrap).sort((a, b) => a - b);
}

/**
 * The orbits of a planet's moons as lines in the sky: for each moon,
 * points of its orbit in radii of the body shown, round the planet's
 * centre, or, with `shown` (from 0), round that moon's centre in its
 * frame (as moonSystemSky places the bodies); the view draws them from
 * the camera. A binary's moon, which does not move in its planet's sky,
 * has none from the planet. The moon shown has its own, which passes
 * through it (sampled ever closer to it).
 */
export function orbitLoops(
  moons: readonly Moon[],
  shown: number | null,
  season: number,
  yearDays: number,
  sunLongitudeRad: number,
  bodyRadiusKm: number,
): Vec3[][] {
  const self = shown === null ? null : moons[shown];
  const here =
    self && shown !== null
      ? moonFrame(self, shown, season, yearDays, sunLongitudeRad)
      : null;
  const unitKm = Math.max(here && self ? self.radius_km : bodyRadiusKm, 1e-6);
  const loops: Vec3[][] = [];
  moons.forEach((moon, k) => {
    const own = k === shown && here !== null;
    if (moon.migration === "locked" && !own) return;
    const tilt = (moon.inclination_deg * Math.PI) / 180;
    const e = Math.min(Math.max(moon.eccentricity, 0), 0.95);
    const { node, periapsis } = orbitOrientation(moon, season, sunLongitudeRad);
    const at = own ? moonPlace(moon, k, season, yearDays, sunLongitudeRad) : null;
    // A binary's moon hangs at [0, 0, -1]: its orbit is drawn through it.
    const locked = moon.migration === "locked";
    const nus = anomalies(
      at ? { at: at.trueAnomaly, size: moon.radius_km / moon.semi_major_axis_km } : null,
    );
    const loop = nus.map((nu): Vec3 => {
      // moonPlace's ellipse, in km.
      const p = locked
        ? scale(turnY([0, 0, -1], nu), moon.semi_major_axis_km)
        : scale(
            onOrbit(periapsis + nu, tilt, node),
            (moon.semi_major_axis_km * (1 - e * e)) / (1 + e * Math.cos(nu)),
          );
      if (!here) return scale(p, 1 / unitKm);
      const v = sub(p, here.centre);
      return scale([dot(v, here.x), dot(v, here.y), dot(v, here.z)], 1 / unitKm);
    });
    loops.push(loop);
  });
  return loops;
}

/**
 * The orbits of a giant's other regular moons in the sky of one of them
 * (see siblingSky), and the world's own: circles in the giant's
 * equatorial plane round the giant, at longitude 180°, `worldDistanceKm`
 * away, in radii of the world (`worldRadiusKm`); the world's own passes
 * through it.
 */
export function siblingOrbitLoops(
  siblings: readonly Pick<Sibling, "distance_km">[],
  worldDistanceKm: number,
  worldRadiusKm: number,
): Vec3[][] {
  const r = Math.max(worldRadiusKm, 1e-6);
  const circle = (distanceKm: number, nus: number[]) =>
    nus.map(
      (a): Vec3 => [
        (distanceKm * Math.sin(a)) / r,
        0,
        (-worldDistanceKm + distanceKm * Math.cos(a)) / r,
      ],
    );
  const own = circle(worldDistanceKm, anomalies({ at: 0, size: worldRadiusKm / worldDistanceKm }));
  return [...siblings.map((sibling) => circle(sibling.distance_km, anomalies(null))), own];
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
      position: scale(direction, distanceKm / planetRadiusKm),
      radius: moon.radius_km / planetRadiusKm,
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
        position: scale(direction, distance),
        radius: planetRadiusKm / moon.radius_km,
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
      position: scale(v, 1 / self.radius_km),
      radius: moon.radius_km / self.radius_km,
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
      position: scale(v, 1 / worldRadiusKm),
      radius: sibling.radius_km / worldRadiusKm,
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

/** Where a giant's regular moons are, round it (see giantSystemSky). */
interface GiantMoon {
  /** Centre, km, in the frame turning with the world's orbit (the world on +z). */
  centre: Vec3;
  radiusKm: number;
  distanceKm: number;
  icy: boolean;
  texture: string | null;
}

/**
 * The sky of the giant a world orbits, or of one of its other regular
 * moons: `from` 'host' for the giant, a sibling's index (from 0)
 * otherwise. The giant's moons are the world and its siblings, where
 * siblingSky has them as the seasons play (in the giant's equatorial
 * plane, in a frame turning with the world's orbit), each locked to the
 * giant (its longitude 180° towards it); the giant turns on itself by
 * `giantTurn` (radians, see bodyTurn) against that frame. Positions and
 * orbit loops are in radii of the body shown. `textures` maps the world
 * (`'world'`), the giant (`'host'`) and each sibling (its index).
 */
export function giantSystemSky(
  host: { distance_km: number; period_days: number; siblings?: readonly Sibling[] },
  worldRadiusKm: number,
  giantRadiusKm: number,
  from: 'host' | number,
  season: number,
  yearDays: number,
  giantTurn: number,
  textures: (body: 'world' | 'host' | number) => string | null,
): { bodies: SkyBody[]; casters: ShadowCaster[]; orbits: Vec3[][] } {
  const days = season * yearDays;
  const siblings = host.siblings ?? [];
  const moons: GiantMoon[] = [
    {
      centre: [0, 0, host.distance_km],
      radiusKm: worldRadiusKm,
      distanceKm: host.distance_km,
      icy: false,
      texture: textures('world'),
    },
    ...siblings.map((sibling, k): GiantMoon => {
      // As siblingSky turns them, from the world's side of the giant.
      const angle =
        k * 2.1 +
        1.3 +
        2 * Math.PI * days * (1 / Math.max(sibling.period_days, 1e-3) - 1 / Math.max(host.period_days, 1e-3));
      return {
        centre: [sibling.distance_km * Math.sin(angle), 0, sibling.distance_km * Math.cos(angle)],
        radiusKm: sibling.radius_km,
        distanceKm: sibling.distance_km,
        icy: sibling.icy,
        texture: textures(k),
      };
    }),
  ];
  const self = from === 'host' ? null : moons[from + 1]!;
  const unitKm = Math.max(self ? self.radiusKm : giantRadiusKm, 1e-6);
  const origin: Vec3 = self ? self.centre : [0, 0, 0];
  // The frame of the body shown: the giant's, turned on itself; a moon's,
  // its z away from the giant.
  const giantAxes = spinAxes(giantTurn);
  const lockedAxes = (centre: Vec3): Axes => {
    const z = unit(centre);
    const y: Vec3 = [0, 1, 0];
    return [cross(y, z), y, z];
  };
  const [ax, ay, az] = self ? lockedAxes(self.centre) : giantAxes;
  const local = (v: Vec3): Vec3 => [dot(v, ax), dot(v, ay), dot(v, az)];
  const place = (centre: Vec3) => scale(local(sub(centre, origin)), 1 / unitKm);
  const bodies: SkyBody[] = [];
  const casters: ShadowCaster[] = [];
  const add = (centre: Vec3, radiusKm: number, body: Omit<SkyBody, 'direction' | 'angularDiameterDeg'>) => {
    const position = place(centre);
    const distance = Math.max(Math.hypot(...position), (radiusKm / unitKm) * 1.001);
    bodies.push({
      ...body,
      direction: unit(position),
      angularDiameterDeg: (2 * Math.asin(Math.min(radiusKm / unitKm / distance, 1)) * 180) / Math.PI,
      position,
      radius: radiusKm / unitKm,
    });
    casters.push({ position, radius: radiusKm / unitKm });
  };
  if (self) {
    const [gx, gy, gz] = giantAxes;
    add([0, 0, 0], giantRadiusKm, {
      colour: PLANET,
      texture: textures('host'),
      axes: [local(gx), local(gy), local(gz)],
    });
  }
  for (const moon of moons) {
    if (moon === self) continue;
    const [mx, my, mz] = lockedAxes(moon.centre);
    add(moon.centre, moon.radiusKm, {
      colour: moon.icy ? ICE : ROCK,
      texture: moon.texture,
      axes: [local(mx), local(my), local(mz)],
    });
  }
  // Their orbits round the giant, in the giant's equatorial plane; the
  // shown moon's own passes through it.
  const orbits = moons.map((moon) => {
    const at = Math.atan2(moon.centre[0], moon.centre[2]);
    const nus = anomalies(moon === self ? { at, size: moon.radiusKm / moon.distanceKm } : null);
    return nus.map((a) => place([moon.distanceKm * Math.sin(a), 0, moon.distanceKm * Math.cos(a)]));
  });
  return {
    bodies,
    casters: casters.sort((a, b) => b.radius - a.radius).slice(0, MAX_CASTERS),
    orbits,
  };
}

/** Two skies together (the giant and its other moons), the largest shadows kept. */
export function joinSkies(
  ...skies: { bodies: SkyBody[]; casters: ShadowCaster[] }[]
): { bodies: SkyBody[]; casters: ShadowCaster[] } {
  const casters = skies.flatMap((s) => s.casters).sort((a, b) => b.radius - a.radius);
  return { bodies: skies.flatMap((s) => s.bodies), casters: casters.slice(0, MAX_CASTERS) };
}
