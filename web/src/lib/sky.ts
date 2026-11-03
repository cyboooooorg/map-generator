import type { WorldMetadata } from './api/types/WorldMetadata';

/**
 * Scattering parameters of a world's atmosphere for the viewer, in units of
 * the globe's radius (1). The shell is far thicker than a real atmosphere
 * (about 0.1 % of Earth's radius) so the sky reads at planet scale; the
 * coefficients are set from optical depths at the zenith, so the look
 * (colour, haze, opacity) follows the planet rather than the exaggeration.
 */
export interface Sky {
  /** Radius of the top of the atmosphere. */
  top: number;
  /** Rayleigh scattering per unit length, red, green, blue. */
  betaR: [number, number, number];
  /** Aerosol (Mie) extinction per unit length. */
  betaM: number;
  /** Share of the aerosol extinction that scatters, per channel (its tint). */
  mieAlbedo: [number, number, number];
  /** Scale heights of the gas and of the aerosols. */
  heightR: number;
  heightM: number;
  /** Anisotropy of aerosol scattering (Henyey-Greenstein `g`), lower for thick hazes. */
  g: number;
}

/** Below this pressure (bar) a world has no sky. */
const AIRLESS_BAR = 1e-4;
/** Optical depth of Earth's air at the zenith, blue channel. */
const EARTH_RAYLEIGH_DEPTH = 0.25;
/** Optical depth of a haze of density 1 at the zenith. */
const HAZE_DEPTH = 2.0;

/** The sky of a world, or null when it has none. */
export function skyFromWorld(meta: WorldMetadata): Sky | null {
  // Worlds made before the atmosphere had Earth's.
  const pressure = meta.pressure_bar ?? 1;
  const atmosphere = meta.preset.atmosphere ?? {
    rayleigh: [0.17, 0.41, 1.0],
    haze: 0.1,
    haze_colour: [1, 1, 1],
    greenhouse: 0,
    clouds: 'water',
    cloud_cover: 0,
  };
  if (pressure < AIRLESS_BAR) {
    return null;
  }
  const thickness = 0.03 + 0.02 * Math.log10(1 + pressure);
  const heightR = 0.25 * thickness;
  const heightM = 0.12 * thickness;
  // Rayleigh depth grows with the column of gas, much softened: in dense
  // atmospheres the aerosols, not the gas, set the colour (Venus is
  // yellow-white, not deep blue).
  const depthR = EARTH_RAYLEIGH_DEPTH * pressure ** 0.35;
  const [r, g, b] = atmosphere.rayleigh;
  const betaR: [number, number, number] = [r, g, b].map((k) => (k * depthR) / heightR) as [number, number, number];
  return {
    top: 1 + thickness,
    betaR,
    betaM: (HAZE_DEPTH * atmosphere.haze) / heightM,
    mieAlbedo: atmosphere.haze_colour,
    heightR,
    heightM,
    // Thick hazes scatter many times, which spreads their light almost
    // evenly: a low anisotropy keeps them visible from the sunny side too.
    g: 0.76 - 0.5 * Math.min(atmosphere.haze / 0.4, 1),
  };
}
