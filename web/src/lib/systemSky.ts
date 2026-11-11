// The sky of any body of a planetary system (docs/system-spec.md), from the
// orbits and spins `system.json` stores: where every other body is, seen
// from the body shown at a time, in that body's frame (`[cos lat sin lon,
// sin lat, cos lat cos lon]`, y along its spin axis), as skyBodies places
// the bodies of a world's sky.
//
// The system's frame has the star at its centre, y along the star's spin
// axis (the plane of the planets near its equator). A planet's orbit is
// placed by its node, inclination and periapsis, its position by Kepler's
// equation from its mean anomaly at the system's epoch; a moon's round its
// planet, in the planet's equatorial plane, which (like its node, counted
// from the planet's equinox direction) is held fixed against the stars
// while the planet turns. A body's spin axis leans from
// its orbit's normal by its axial tilt, towards a fixed direction (the
// node line); it turns once per `spin.period_hours`, a locked body at a
// steady pace, its longitude 180° towards what it orbits on average: its
// host swings in its sky by the libration (true minus mean anomaly, about
// +-2e radians). A planet in the 3:2 spin-orbit resonance turns 1.5 times
// per orbit, its long axis (longitude 0 or 180°) towards the star at each
// periapsis, as Mercury.
import type { Body } from './api/types/Body';
import type { Orbit } from './api/types/Orbit';
import type { System } from './api/types/System';
import { resonanceTurns } from './spinResonance';
import { IDENTITY_AXES, pickShadowCasters, ringSeed, type Axes, type ShadowCaster, type SkyBody, type SurfaceMaps, type Vec3 } from './skyBodies';

const AU_KM = 1.495_978_707e8;
/** The Sun's radius, km. */
const SUN_RADIUS_KM = 695_700;
const DEG = Math.PI / 180;

const dot = (a: Vec3, b: Vec3) => a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
const add = (a: Vec3, b: Vec3): Vec3 => [a[0] + b[0], a[1] + b[1], a[2] + b[2]];
const sub = (a: Vec3, b: Vec3): Vec3 => [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
const scale = (a: Vec3, k: number): Vec3 => [a[0] * k, a[1] * k, a[2] * k];
const cross = (a: Vec3, b: Vec3): Vec3 => [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
const unit = (a: Vec3): Vec3 => scale(a, 1 / Math.max(Math.hypot(...a), 1e-12));

/** A vector turned by `angle` about `axis` (a unit vector), right-handed. */
function rotate(v: Vec3, axis: Vec3, angle: number): Vec3 {
  const c = Math.cos(angle);
  const s = Math.sin(angle);
  return add(add(scale(v, c), scale(cross(axis, v), s)), scale(axis, dot(axis, v) * (1 - c)));
}

/** The frame an orbit lies in: its node line, its normal, and the third axis. */
interface Plane {
  node: Vec3;
  normal: Vec3;
  third: Vec3;
}

/**
 * The plane of an orbit in its parent's frame `parent` (y the parent's
 * axis): turned by its node about the axis, tilted by its inclination
 * about the node line.
 */
function orbitPlane(orbit: Orbit, parent: Axes): Plane {
  const [px, py, pz] = parent;
  const node = add(scale(pz, Math.cos(orbit.node_deg * DEG)), scale(px, Math.sin(orbit.node_deg * DEG)));
  const normal = rotate(py, node, orbit.inclination_deg * DEG);
  return { node, normal, third: cross(normal, node) };
}

/** The mean, true and eccentric anomalies of an orbit after `days`, radians (Kepler's equation). */
export function anomalies(orbit: Orbit, days: number): { mean: number; true: number; eccentric: number } {
  const e = Math.min(Math.max(orbit.eccentricity, 0), 0.95);
  const mean = orbit.mean_anomaly_deg * DEG + (2 * Math.PI * days) / Math.max(orbit.period_days, 1e-6);
  let eccentric = e < 0.8 ? mean : Math.PI;
  for (let i = 0; i < 8; i += 1) {
    eccentric -= (eccentric - e * Math.sin(eccentric) - mean) / (1 - e * Math.cos(eccentric));
  }
  const nu = 2 * Math.atan2(Math.sqrt(1 + e) * Math.sin(eccentric / 2), Math.sqrt(1 - e) * Math.cos(eccentric / 2));
  return { mean, true: nu, eccentric };
}

/**
 * A body's place on its orbit after `days`, km, from its parent's centre:
 * the true anomaly of Kepler's equation, from the argument of periapsis,
 * prograde (from the node towards `third`).
 */
export function orbitPosition(orbit: Orbit, plane: Plane, days: number): Vec3 {
  const e = Math.min(Math.max(orbit.eccentricity, 0), 0.95);
  const { true: nu, eccentric } = anomalies(orbit, days);
  const r = orbit.semi_major_axis_km * (1 - e * Math.cos(eccentric));
  return pointOnOrbit(plane, orbit.periapsis_deg * DEG + nu, r);
}

/**
 * The amplitude of the libration of a locked body on an orbit of
 * eccentricity `e`, radians: the greatest true minus mean anomaly, exact
 * (Kepler's equation) rather than the first-order 2e. It is reached where
 * the orbital rate equals the mean motion, at r = a (1 - e^2)^(1/4) (from
 * Kepler's second law, r^2 dnu/dt = n a^2 sqrt(1 - e^2)): 28.9° at e = 0.25
 * against 2e = 28.6°.
 */
export function librationAmplitude(e: number): number {
  const ecc = Math.min(Math.max(e, 0), 0.95);
  if (ecc < 1e-9) return 0;
  const r = Math.pow(1 - ecc * ecc, 0.25);
  const eccentric = Math.acos(Math.min(Math.max((1 - r) / ecc, -1), 1));
  const mean = eccentric - ecc * Math.sin(eccentric);
  const nu = 2 * Math.atan2(Math.sqrt(1 + ecc) * Math.sin(eccentric / 2), Math.sqrt(1 - ecc) * Math.cos(eccentric / 2));
  return nu - mean;
}

/**
 * The libration of a body locked to what it orbits after `days`, radians in
 * (-pi, pi]: the true anomaly minus the mean anomaly. A locked body turns at
 * the mean pace, facing the point of its orbit at the mean anomaly, so its
 * host is turned from that face by this angle, positive when the host is
 * ahead of the mean direction in the sense of the orbit.
 */
export function libration(orbit: Orbit, days: number): number {
  const { mean, true: nu } = anomalies(orbit, days);
  return Math.atan2(Math.sin(nu - mean), Math.cos(nu - mean));
}

/** The point of an orbit's plane at angle `angle` from its node, `r` km out. */
function pointOnOrbit(plane: Plane, angle: number, r: number): Vec3 {
  return scale(add(scale(plane.node, Math.cos(angle)), scale(plane.third, Math.sin(angle))), r);
}

/** Where a body is and how it is turned, in the system's frame. */
export interface Placement {
  /** Its centre, km from the star. */
  centre: Vec3;
  /** Its own axes (its map's frame): x, y (its spin axis), z. */
  axes: Axes;
  /**
   * Its equatorial frame, not turning with its day: `[cross(y, start), y,
   * start]`, `y` its spin axis and `start` its equinox direction (its orbit's
   * node line projected on its equator). The frame its moons' orbits sit in.
   */
  equator: Axes;
  /** The plane of its orbit. */
  plane: Plane;
}

/**
 * Where every body of the system is after `days` from the system's epoch,
 * and how it is turned (see the head of this file); `spinOffset` turns the
 * body `turned` on itself by that much more (radians), as the hour of its
 * view sets it.
 */
export function placeSystem(
  system: System,
  days: number,
  turned: { id: string; spinOffset: number } | null = null,
): Map<string, Placement> {
  const placements = new Map<string, Placement>();
  const byId = new Map(system.bodies.map((b) => [b.id, b]));
  const place = (body: Body): Placement => {
    const known = placements.get(body.id);
    if (known) return known;
    const parent = body.parent ? byId.get(body.parent) : undefined;
    const parentPlace = parent ? place(parent) : null;
    const plane = orbitPlane(body.orbit, parentPlace?.equator ?? IDENTITY_AXES);
    const centre = add(parentPlace?.centre ?? [0, 0, 0], orbitPosition(body.orbit, plane, days));
    // Its axis leans from its orbit's normal by its tilt, about the node line.
    const y = unit(rotate(plane.normal, plane.node, body.spin.axial_tilt_deg * DEG));
    // Its longitude 0 at the start, then turning on itself; a locked body
    // turns steadily, its longitude 180° towards the point of its orbit at
    // the mean anomaly (its host is off that by the libration); one in the
    // 3:2 resonance turns 1.5 times as fast (2 in the 2:1) from the same face at periapsis.
    const start = unit(sub(plane.node, scale(y, dot(plane.node, y))));
    let z: Vec3;
    const resonance = resonanceTurns(body.spin);
    if (body.spin.locked || resonance > 0) {
      const turns = body.spin.locked ? 1 : resonance;
      z = pointOnOrbit(plane, body.orbit.periapsis_deg * DEG + turns * anomalies(body.orbit, days).mean, 1);
    } else {
      const turns = (days * 24) / Math.max(body.spin.period_hours, 1e-6);
      const offset = turned && turned.id === body.id ? turned.spinOffset : 0;
      z = rotate(start, y, 2 * Math.PI * (turns - Math.floor(turns)) + offset);
    }
    z = unit(sub(z, scale(y, dot(z, y))));
    const placement = { centre, axes: [cross(y, z), y, z] as Axes, equator: [cross(y, start), y, start] as Axes, plane };
    placements.set(body.id, placement);
    return placement;
  };
  for (const body of system.bodies) place(body);
  return placements;
}

/**
 * Points sampled evenly round an orbit line by default (the light
 * quality profile; see QualityProfile.orbitSamples): 2.25° apart, whose
 * corners show where a line passes near the camera.
 */
export const ORBIT_SAMPLES = 160;

/** Colours of a body drawn before its map loads (or when it has none). */
const ROCK: Vec3 = [0.32, 0.3, 0.28];

/**
 * The sky of body `shown` of a system after `days`: every other body,
 * placed from the camera by its position and radius in radii of the body
 * shown, with its axes, its map when it is generated (`textures(id)`, null
 * otherwise) or its preview; the largest shadows; the orbits of the planets
 * round the star and of the moons of the shown body's planet; and the
 * star's direction, for the light, and its place and radius (in radii of
 * the body shown), to draw it there. `spinOffset` turns the body shown on
 * itself (see placeSystem). Each orbit line is `orbitSamples` points
 * spread evenly round it, and more ever closer to its body (see below).
 */
export function systemSky(
  system: System,
  shown: string,
  days: number,
  textures: (id: string) => SurfaceMaps | null,
  spinOffset = 0,
  orbitSamples = ORBIT_SAMPLES,
): {
  bodies: SkyBody[];
  ids: string[];
  casters: ShadowCaster[];
  orbits: Vec3[][];
  /** The body each of `orbits` belongs to, and whether it goes round the planet of the body shown (a moon's orbit) rather than the star. */
  orbitBodies: { id: string; local: boolean }[];
  /** The index in `orbits` of the body shown's own orbit (dashed, so that the body slides along it), -1 when none. */
  ownOrbit: number;
  sun: Vec3;
  /**
   * The star's place and radius, and its axes in the frame of the body
   * shown: the system's +x, +y (the star's spin axis) and +z, the frame
   * its surface is drawn in (lib/viewer/hostStarSurface.ts).
   */
  star: { position: Vec3; radius: number; axes: [Vec3, Vec3, Vec3] };
} {
  const placements = placeSystem(system, days, { id: shown, spinOffset });
  const self = system.bodies.find((b) => b.id === shown);
  const here = placements.get(shown);
  if (!self || !here) {
    return { bodies: [], ids: [], casters: [], orbits: [], orbitBodies: [], ownOrbit: -1, sun: [0, 0, 1], star: { position: [0, 0, 1e9], radius: 1, axes: IDENTITY_AXES } };
  }
  const [ax, ay, az] = here.axes;
  const unitKm = Math.max(self.physical.radius_km, 1e-6);
  const local = (v: Vec3): Vec3 => [dot(v, ax), dot(v, ay), dot(v, az)];
  const place = (centre: Vec3) => scale(local(sub(centre, here.centre)), 1 / unitKm);
  const bodies: SkyBody[] = [];
  const ids: string[] = [];
  const casters: ShadowCaster[] = [];
  for (const body of system.bodies) {
    if (body.id === shown) continue;
    const there = placements.get(body.id)!;
    const position = place(there.centre);
    const radius = body.physical.radius_km / unitKm;
    const distance = Math.max(Math.hypot(...position), radius * 1.001);
    const [bx, by, bz] = there.axes;
    const maps = body.generated ? textures(body.id) : null;
    bodies.push({
      direction: unit(position),
      angularDiameterDeg: (2 * Math.asin(Math.min(radius / distance, 1)) * 180) / Math.PI,
      position,
      radius,
      colour: (body.preview.colours[0] as Vec3 | undefined) ?? ROCK,
      maps,
      axes: [local(bx), local(by), local(bz)],
      axialTiltDeg: body.spin.axial_tilt_deg,
      ringSeed: ringSeed(body.physical.mass_earth),
      // The star is at the origin: its light at that body, and its phase from here.
      phaseCos: dot(unit(scale(there.centre, -1)), unit(sub(here.centre, there.centre))),
      insolation: (Math.hypot(...here.centre) / Math.max(Math.hypot(...there.centre), 1)) ** 2,
      ...(maps ? {} : { preview: body.preview }),
    });
    ids.push(body.id);
    casters.push({ position, radius });
  }
  // The orbits: the planets' round the star, the moons' of the shown
  // body's planet round it (the shown body's own passing through it).
  const planet = self.parent ?? self.id;
  const orbiting = system.bodies.filter((b) => b.parent === null || b.parent === planet);
  const ownOrbit = orbiting.findIndex((b) => b.id === shown);
  const orbits = orbiting
    .map((body) => {
      const there = placements.get(body.id)!;
      const parent = body.parent ? placements.get(body.parent)!.centre : ([0, 0, 0] as Vec3);
      const e = Math.min(Math.max(body.orbit.eccentricity, 0), 0.95);
      const a = body.orbit.semi_major_axis_km;
      // Its angle now, from the node: each line sampled ever closer to its
      // body, through its centre, from a step a fifth of the body's
      // distance from the body shown (of the body shown's radius on its
      // own orbit), growing by 1.4 to the even one. A chord of the even
      // samples would miss it by up to 1.9e-4 of the orbit's radius at
      // 160 (a giant's own orbit round its star by half its radius,
      // beside it in its moon's sky), and corners would show near it.
      const rel = sub(there.centre, parent);
      const now = Math.atan2(dot(rel, there.plane.third), dot(rel, there.plane.node));
      const even = (2 * Math.PI) / orbitSamples;
      const angles: number[] = Array.from({ length: orbitSamples }, (_, i) => even * i);
      const from = Math.max(Math.hypot(...sub(there.centre, here.centre)), self.physical.radius_km);
      for (let step = (0.2 * from) / a; step < even; step *= 1.4) {
        angles.push(now + step, now - step);
      }
      angles.push(now);
      const wrap = (x: number) => x - 2 * Math.PI * Math.floor(x / (2 * Math.PI));
      const periapsis = body.orbit.periapsis_deg * DEG;
      return angles
        .map(wrap)
        .sort((p, q) => p - q)
        .map((angle) => {
          const nu = angle - periapsis;
          const r = (a * (1 - e * e)) / (1 + e * Math.cos(nu));
          return place(add(parent, pointOnOrbit(there.plane, angle, r)));
        });
    });
  const sun = unit(local(scale(here.centre, -1)));
  const starRadius = (system.star.info.radius * SUN_RADIUS_KM) / unitKm;
  const starDistance = Math.max(Math.hypot(...place([0, 0, 0])), starRadius * 1.001);
  return {
    bodies,
    ids,
    // Those whose shadow reaches the body shown now: a moon's on its
    // planet, the planet's on its moon, whatever the far giants' sizes.
    casters: pickShadowCasters(casters, sun, Math.asin(starRadius / starDistance)),
    orbits,
    orbitBodies: orbiting.map((b) => ({ id: b.id, local: b.parent === planet })),
    ownOrbit,
    sun: unit(local(scale(here.centre, -1))),
    star: {
      position: place([0, 0, 0]),
      radius: (system.star.info.radius * SUN_RADIUS_KM) / unitKm,
      axes: [local([1, 0, 0]), local([0, 1, 0]), local([0, 0, 1])],
    },
  };
}

/**
 * How far out the camera must be able to go, in radii of body `shown`
 * from its centre, to see its planet's system of moons whole: three times
 * the furthest a moon of that planet (or the planet, from a moon) gets
 * from the body shown, which puts that distance well inside the view's
 * 40° field. Null for a body with nothing round it to see.
 */
export function systemReach(system: System, shown: string): number | null {
  const self = system.bodies.find((b) => b.id === shown);
  if (!self) return null;
  const planet = self.parent ?? self.id;
  const apo = (o: Orbit) => o.semi_major_axis_km * (1 + Math.min(Math.max(o.eccentricity, 0), 0.95));
  // From a moon, its own distance to the planet plus the furthest moon's.
  const own = self.parent ? apo(self.orbit) : 0;
  const moons = system.bodies.filter((b) => b.parent === planet && b.id !== shown);
  const furthest = Math.max(own, ...moons.map((b) => own + apo(b.orbit)));
  if (furthest <= 0) return null;
  return (3 * furthest) / Math.max(self.physical.radius_km, 1e-6);
}

/**
 * How far out the camera must be able to go, in radii of the body shown
 * from its centre, to see a Hill sphere whole from outside: its farthest
 * point from the body, `|centre| + radius`, over the sine of the view's
 * half field (20 degrees of the 40 degree field, 2.9), plus a tenth for the
 * margin, rounded up to a step of 10 % so that a host moving on its orbit
 * does not move the limit every frame. `centre` and `radius` are in radii
 * of the body shown.
 */
export function hillReach(centre: readonly [number, number, number], radius: number): number {
  const farthest = Math.hypot(centre[0], centre[1], centre[2]) + Math.max(radius, 0);
  const reach = (farthest * 1.1) / Math.sin((20 * Math.PI) / 180);
  return reach > 0 ? 1.1 ** Math.ceil(Math.log(reach) / Math.log(1.1) - 1e-9) : 0;
}

/** Earth masses in a Sun's mass. */
const EARTH_PER_SUN = 332_946;

/**
 * The radius of a body's Hill sphere, km: how far round it its own gravity
 * holds moons against its primary's, `a (1 - e) cbrt(m / 3M)` at the
 * periapsis, the tightest. `mass` and `primaryMass` in the same unit.
 */
export function hillRadiusKm(semiMajorKm: number, eccentricity: number, mass: number, primaryMass: number): number {
  return semiMajorKm * (1 - Math.min(Math.max(eccentricity, 0), 0.95)) * Math.cbrt(mass / (3 * Math.max(primaryMass, 1e-12)));
}

/**
 * The Hill sphere to draw for body `shown`: its own, round the primary it
 * orbits (the star for a planet); from a moon, its host planet's (round the
 * star), the one the moon sits in. Null when the body is unknown.
 */
export function hillSphere(
  system: System,
  shown: string,
): { id: string; radiusKm: number; radiusInHostRadii: number } | null {
  const self = system.bodies.find((b) => b.id === shown);
  if (!self) return null;
  const host = self.parent ? system.bodies.find((b) => b.id === self.parent) : self;
  if (!host) return null;
  const primary = host.parent ? system.bodies.find((b) => b.id === host.parent) : undefined;
  const primaryMass = primary ? primary.physical.mass_earth : system.star.info.mass * EARTH_PER_SUN;
  const radiusKm = hillRadiusKm(host.orbit.semi_major_axis_km, host.orbit.eccentricity, host.physical.mass_earth, primaryMass);
  return { id: host.id, radiusKm, radiusInHostRadii: radiusKm / Math.max(host.physical.radius_km, 1e-6) };
}

/** The gravitational constant, m³ / (kg s²), and an Earth mass, kg. */
const G = 6.674e-11;
const EARTH_KG = 5.972e24;

/** A body's motion round what it orbits (its planet for a moon, the star for a planet), as the Vectors switch draws it. */
export interface SkyMotion {
  /** Towards its primary (unit), in the frame of the body shown. */
  pull: Vec3;
  /** Its direction of motion round its primary (unit), in that frame; null for a body that does not move. */
  velocity: Vec3 | null;
  /** The primary's pull on it, m/s², at its present distance. */
  pullMs2: number;
  /** Its speed round its primary, km/s, at its present distance (vis-viva). */
  speedKms: number;
  /** Its present distance from its primary, km. */
  distanceKm: number;
}

/**
 * The motion of every body of the system round its primary after `days`,
 * the body shown included, its directions in the frame of body `shown`
 * (see systemSky; `spinOffset` as there). Empty when `shown` is unknown.
 */
export function skyMotions(system: System, shown: string, days: number, spinOffset = 0): Map<string, SkyMotion> {
  const placements = placeSystem(system, days, { id: shown, spinOffset });
  const here = placements.get(shown);
  const motions = new Map<string, SkyMotion>();
  if (!here) return motions;
  const [ax, ay, az] = here.axes;
  const local = (v: Vec3): Vec3 => [dot(v, ax), dot(v, ay), dot(v, az)];
  const starKg = system.star.info.mass * EARTH_PER_SUN * EARTH_KG;
  for (const body of system.bodies) {
    const there = placements.get(body.id)!;
    const primary = body.parent ? system.bodies.find((b) => b.id === body.parent) : undefined;
    const primaryCentre = body.parent ? (placements.get(body.parent)?.centre ?? [0, 0, 0]) : [0, 0, 0];
    const primaryKg = primary ? primary.physical.mass_earth * EARTH_KG : starKg;
    const out = sub(there.centre, primaryCentre as Vec3);
    const distanceKm = Math.max(Math.hypot(...out), 1e-6);
    // Along the orbit, from its places a moment before and after.
    const dt = Math.max(body.orbit.period_days, 1e-6) * 1e-4;
    const step = sub(orbitPosition(body.orbit, there.plane, days + dt), orbitPosition(body.orbit, there.plane, days - dt));
    const r = distanceKm * 1000;
    const a = Math.max(body.orbit.semi_major_axis_km, 1e-6) * 1000;
    const mu = G * (primaryKg + body.physical.mass_earth * EARTH_KG);
    motions.set(body.id, {
      pull: unit(local(scale(out, -1))),
      velocity: Math.hypot(...step) > 0 ? unit(local(step)) : null,
      pullMs2: (G * primaryKg) / (r * r),
      speedKms: Math.sqrt(Math.max(mu * (2 / r - 1 / a), 0)) / 1000,
      distanceKm,
    });
  }
  return motions;
}

/** The shown body's distance from the star, AU (for the light it gets). */
export function starDistanceAu(system: System, shown: string, days: number): number {
  const here = placeSystem(system, days).get(shown);
  return here ? Math.hypot(...here.centre) / AU_KM : 1;
}

/** A trojan swarm in the sky of the body shown. */
export interface SkySwarm {
  /** The planet whose swarm it is. */
  planet: string;
  point: 'L4' | 'L5';
  /** Its bodies larger than 1 km. */
  count: number;
  /** Where its centre is, in radii of the body shown, in its frame. */
  position: Vec3;
}

/** A belt in the sky of the body shown. */
export interface SkyBelt {
  /** Its index in the system's `belts`. */
  index: number;
  /** Its inner and outer edges, circles round the star in its equatorial plane, in radii of the body shown. */
  edges: [Vec3[], Vec3[]];
  /** Whether the body shown (or its planet) orbits within it. */
  inside: boolean;
}

/** Points sampled round a belt's edge. */
const BELT_SAMPLES = 180;

/**
 * The small bodies of the system in the sky of body `shown` after `days`
 * (docs/superpowers/plans/2026-11-06-asteroid-belts.md, task 4): each
 * planet's trojan swarms, at its L4 and L5 points, 60° ahead of it and
 * behind it round the star (they keep pace with it),
 * and each belt as its two edges, circles in the star's equatorial plane.
 * Placed as `systemSky` places the bodies (see `spinOffset` there).
 */
export function smallBodiesSky(
  system: System,
  shown: string,
  days: number,
  spinOffset = 0,
): { swarms: SkySwarm[]; belts: SkyBelt[] } {
  const placements = placeSystem(system, days, { id: shown, spinOffset });
  const self = system.bodies.find((b) => b.id === shown);
  const here = placements.get(shown);
  if (!self || !here) return { swarms: [], belts: [] };
  const [ax, ay, az] = here.axes;
  const unitKm = Math.max(self.physical.radius_km, 1e-6);
  const local = (v: Vec3): Vec3 => [dot(v, ax), dot(v, ay), dot(v, az)];
  const place = (centre: Vec3) => scale(local(sub(centre, here.centre)), 1 / unitKm);
  const swarms: SkySwarm[] = [];
  for (const body of system.bodies) {
    if (body.parent !== null || !body.trojans) continue;
    const there = placements.get(body.id)!;
    for (const [point, shift, count] of [
      ['L4', 60, body.trojans.l4],
      ['L5', -60, body.trojans.l5],
    ] as const) {
      if (count <= 0) continue;
      // The planet's place turned about its orbit's normal: the star, the
      // planet and the point make an equilateral triangle, eccentric orbit
      // or not.
      swarms.push({ planet: body.id, point, count, position: place(rotate(there.centre, there.plane.normal, shift * DEG)) });
    }
  }
  // How far the body shown (or its planet) is from the star.
  const planet = placements.get(self.parent ?? self.id)!;
  const fromStar = Math.hypot(...planet.centre) / AU_KM;
  const circle = (au: number): Vec3[] =>
    Array.from({ length: BELT_SAMPLES }, (_, i) => {
      const angle = (2 * Math.PI * i) / BELT_SAMPLES;
      return place([au * AU_KM * Math.sin(angle), 0, au * AU_KM * Math.cos(angle)]);
    });
  const belts = (system.belts ?? []).map((belt, index) => ({
    index,
    edges: [circle(belt.inner_au), circle(belt.outer_au)] as [Vec3[], Vec3[]],
    inside: fromStar >= belt.inner_au && fromStar <= belt.outer_au,
  }));
  return { swarms, belts };
}
