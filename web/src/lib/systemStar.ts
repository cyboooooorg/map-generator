// A planetary system (docs/system-spec.md) as the star view draws it: the
// star and the orbit of the planet shown (or of the planet of the moon
// shown) as its world's, and every other planet as the view draws a
// world's giants, each a sphere of its preview's colour on its orbit.
import type { Body } from './api/types/Body';
import type { Giant } from './api/types/Giant';
import type { GiantSystem } from './api/types/GiantSystem';
import type { StarInfo } from './api/types/StarInfo';
import type { System } from './api/types/System';
import { withOrbitDefaults } from './viewer/starSystem';

const DEG = Math.PI / 180;
/** The Sun's radius, AU. */
const SUN_RADIUS_AU = 0.004_65;

/** A linear colour as the view's markers take it (sRGB). */
const toSrgb = (c: number) => (c <= 0.003_130_8 ? 12.92 * c : 1.055 * c ** (1 / 2.4) - 0.055);

/**
 * The star view's star and orbit for planet `planet` of a system, its
 * periapsis angle, and the other planets as its giants, with their ids in
 * the same order, each under the name given to it in `names` (by body id),
 * else its designation.
 */
export function systemStarView(
  system: System,
  planet: Body,
  names: Readonly<Record<string, string>> = {},
): { star: StarInfo; omega: number; giants: GiantSystem; ids: string[] } {
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
  });
  const others = system.bodies.filter((b) => b.kind === 'planet' && b.id !== planet.id);
  const giants: Giant[] = others.map((body) => {
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
      // The draw does not give rings yet.
      rings: 'none',
      seed: body.params.seed,
    };
  });
  return {
    star,
    omega: (planet.orbit.node_deg + planet.orbit.periapsis_deg) * DEG,
    giants: { giants, forced_eccentricity: 0, impact_factor: 1 },
    ids: others.map((b) => b.id),
  };
}
