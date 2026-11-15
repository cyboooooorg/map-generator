// What an amount of water in Earth oceans means for a planet: the share of its
// mass, and the depth it would have spread over the whole surface.

/** Mass of one "ocean", the form's water unit (kg): Earth's hydrosphere, 1.4e21 kg (crates/mapgen/src/planet/regime.rs, `OCEAN_KG`; Charette & Smith 2010). */
export const OCEAN_KG = 1.4e21;
/** Mass of the Earth (kg), IAU 2015 nominal value. */
export const EARTH_MASS_KG = 5.972e24;
/** Mean radius of the Earth (m), IAU 2015 nominal value. */
export const EARTH_RADIUS_M = 6.371e6;
/** Density of liquid water (kg/m³); compression under a deep ocean is ignored. */
export const WATER_DENSITY = 1000;

/** The share (0 to 1) of the planet's mass that `oceans` of water make, for a mass in Earth masses. */
export function waterShare(oceans: number, massEarth: number): number {
  return (oceans * OCEAN_KG) / (massEarth * EARTH_MASS_KG);
}

/** The depth in km of `oceans` of water spread over a whole planet of the given radius in Earth radii (4πR² of surface). */
export function waterDepthKm(oceans: number, radiusEarth: number): number {
  const radius = radiusEarth * EARTH_RADIUS_M;
  const meters = (oceans * OCEAN_KG) / WATER_DENSITY / (4 * Math.PI * radius * radius);
  return meters / 1000;
}
