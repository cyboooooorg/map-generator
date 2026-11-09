// A planetary system (docs/system-spec.md) as the star view draws it: the
// star and the orbit of the planet shown (or of the planet of the moon
// shown) as its world's, and every planet, that one included, as the view
// draws a world's giants, each a sphere of its preview's colour on its
// orbit.
import type { Body } from './api/types/Body';
import type { Preview } from './api/types/Preview';
import type { Rings } from './api/types/Rings';
import type { RingSystem } from './api/types/RingSystem';
import type { GiantSystem } from './api/types/GiantSystem';
import type { StarInfo } from './api/types/StarInfo';
import type { System } from './api/types/System';
import type { Belt } from './api/types/Belt';
import type { Trojans } from './api/types/Trojans';
import { resonanceTurns } from './spinResonance';
import { placeSystem, type Placement } from './systemSky';
import { type SystemGiant, withOrbitDefaults } from './viewer/starSystem';

const DEG = Math.PI / 180;
/** The Sun's radius, AU. */
const SUN_RADIUS_AU = 0.004_65;

/** A linear colour as the view's markers take it (sRGB). */
const toSrgb = (c: number) => (c <= 0.003_130_8 ? 12.92 * c : 1.055 * c ** (1 / 2.4) - 0.055);

/** What the star view's legend of a giant's rings says of `rings`. */
export function ringsKind(rings: RingSystem | null): Rings {
  return rings ? (rings.bright ? 'bright' : 'faint') : 'none';
}

/** A planet of a system as the star view and its cards show it. */
export interface SystemPlanet {
  id: string;
  /** The name given to it, else its designation. */
  name: string;
  /** Its planet type (`terran`, `gas-giant`…) and that type as the interface words it. */
  type: string;
  typeLabel: string;
  radius_km: number;
  mass_earth: number;
  orbit_au: number;
  eccentricity: number;
  year_days: number;
  equilibrium_k: number;
  /** How many moons it has. */
  moons: number;
  /** Its moons, nearest first: id, name (given, else designation) and orbit. */
  moonList: { id: string; name: string; radius_km: number; semi_major_axis_km: number; period_days: number; inclination_deg: number }[];
  /** Its orbit's tilt to the star's equator, degrees. */
  inclination_deg: number;
  /** Mean starlight, relative to Earth's. */
  flux_earth: number;
  /** Surface gravity (cloud tops for a giant), relative to Earth's. */
  gravity: number;
  /** Its day, hours, and whether it is locked to its star (its day its year). */
  day_hours: number;
  locked: boolean;
  /** Whether its maps exist. */
  generated: boolean;
  /** What the viewer draws before its maps exist. */
  preview: Preview;
  /** Its trojans at L4 and L5, where they last; null otherwise (and in systems drawn before them). */
  trojans: Trojans | null;
}

/**
 * The star view's star and orbit for planet `planet` of a system, its
 * periapsis angle, and every planet (`planet` included, at `selected`) as
 * its giants, with their ids and their facts (`planets`) in the same
 * order, each under the name given to it in `names` (by body id), else its
 * designation, its type worded by `typeLabels` (by type), else as is.
 */
export function systemStarView(
  system: System,
  planet: Body,
  names: Readonly<Record<string, string>> = {},
  typeLabels: Readonly<Record<string, string>> = {},
): {
  star: StarInfo;
  omega: number;
  giants: Omit<GiantSystem, 'giants'> & { giants: SystemGiant[] };
  ids: string[];
  planets: SystemPlanet[];
  selected: number;
  /** The system's belts, nearest first (none in systems drawn before them). */
  belts: Belt[];
} {
  const base = system.star.info;
  const a = planet.orbit.semi_major_axis_au;
  const e = planet.orbit.eccentricity;
  const star = withOrbitDefaults({
    ...base,
    orbit_au: a,
    eccentricity: e,
    periapsis_au: a * (1 - e),
    apoapsis_au: a * (1 + e),
    insolation: planet.surroundings.flux_earth,
    insolation_periapsis: base.luminosity / (a * (1 - e)) ** 2,
    insolation_apoapsis: base.luminosity / (a * (1 + e)) ** 2,
    hz_position: planet.surroundings.hz_position,
    equilibrium_k: planet.surroundings.equilibrium_k,
    year_days: planet.orbit.period_days,
    angular_diameter_deg: (2 * Math.atan((SUN_RADIUS_AU * base.radius) / a)) / DEG,
    tidally_locked: planet.spin.locked,
    spin_orbit_3_2: planet.spin.spin_orbit_3_2 ?? false,
    spin_turns: planet.spin.resonance_turns ?? 0,
  });
  const all = system.bodies.filter((b) => b.kind === 'planet');
  const moonsOf = (id: string) =>
    system.bodies
      .filter((b) => b.parent === id)
      .sort((x, y) => x.orbit.semi_major_axis_km - y.orbit.semi_major_axis_km)
      .map((b) => ({
        id: b.id,
        name: names[b.id] ?? b.designation,
        radius_km: b.physical.radius_km,
        semi_major_axis_km: b.orbit.semi_major_axis_km,
        period_days: b.orbit.period_days,
        inclination_deg: b.orbit.inclination_deg,
        // A moon of the rings: drawn among them (see moonOrbits).
        ring: Boolean(b.ring_role),
      }));
  // Every body placed once per day asked, for all the planets' globes.
  let placed: { days: number; at: Map<string, Placement> } | null = null;
  const placeAt = (days: number) => {
    if (placed?.days !== days) placed = { days, at: placeSystem(system, days) };
    return placed.at;
  };
  const giants: SystemGiant[] = all.map((body) => {
    const colour = (body.preview.colours[0] ?? [0.4, 0.4, 0.4]).map(toSrgb) as [number, number, number];
    return {
      name: names[body.id] ?? body.designation,
      kind: body.type === 'ice-giant' ? 'ice' : 'gas',
      mass_earth: body.physical.mass_earth,
      radius_earth: body.physical.radius_km / 6371,
      orbit_au: body.orbit.semi_major_axis_au,
      eccentricity: body.orbit.eccentricity,
      periapsis_angle: (body.orbit.node_deg + body.orbit.periapsis_deg) * DEG,
      phase: (body.orbit.mean_anomaly_deg * DEG) / (2 * Math.PI),
      year_days: body.orbit.period_days,
      equilibrium_k: body.surroundings.equilibrium_k,
      effective_k: body.surroundings.equilibrium_k,
      class: 0,
      hot: body.type === 'hot-jupiter',
      colour,
      // Its rings as the system drew them; none on a system drawn before.
      rings: ringsKind(body.params.given_rings?.rings ?? null),
      ringSystem: body.params.given_rings?.rings ?? null,
      trojans: body.trojans ?? null,
      seed: body.params.seed,
      inclination: body.orbit.inclination_deg * DEG,
      node: body.orbit.node_deg * DEG,
      moons: moonsOf(body.id).map((m) => ({ ...m, barycentre_km: 0 })),
      axes: (days: number) => placeAt(days).get(body.id)!.axes,
      ...(body.spin.locked || resonanceTurns(body.spin) > 0 ? {} : { spinHours: body.spin.period_hours }),
    };
  });
  return {
    star,
    omega: (planet.orbit.node_deg + planet.orbit.periapsis_deg) * DEG,
    giants: { giants, forced_eccentricity: 0, impact_factor: 1 },
    ids: all.map((b) => b.id),
    planets: all.map((body) => ({
      id: body.id,
      name: names[body.id] ?? body.designation,
      type: body.type,
      typeLabel: typeLabels[body.type] ?? body.type,
      radius_km: body.physical.radius_km,
      mass_earth: body.physical.mass_earth,
      orbit_au: body.orbit.semi_major_axis_au,
      eccentricity: body.orbit.eccentricity,
      year_days: body.orbit.period_days,
      equilibrium_k: body.surroundings.equilibrium_k,
      moons: system.bodies.filter((b) => b.parent === body.id).length,
      moonList: moonsOf(body.id),
      inclination_deg: body.orbit.inclination_deg,
      flux_earth: body.surroundings.flux_earth,
      gravity: body.physical.gravity,
      day_hours: body.spin.period_hours,
      locked: body.spin.locked,
      generated: body.generated,
      preview: body.preview,
      trojans: body.trojans ?? null,
    })),
    selected: all.findIndex((b) => b.id === planet.id),
    belts: system.belts ?? [],
  };
}
