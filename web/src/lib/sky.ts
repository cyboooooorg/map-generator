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
  const pressure = meta.pressure_bar;
  const atmosphere = meta.look.atmosphere;
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

/**
 * The sky stretched upwards by `lift` (planet radii), for relief displaced
 * above the ground: the shell and its scale heights grow by the same
 * factor and the coefficients shrink by it, so the zenith optical depths,
 * hence the look, stay the same while the mountains stay under the sky.
 */
export function liftSky(sky: Sky, lift: number): Sky {
  const thickness = sky.top - 1;
  const k = (thickness + Math.max(lift, 0)) / thickness;
  return {
    ...sky,
    top: 1 + thickness * k,
    betaR: sky.betaR.map((b) => b / k) as [number, number, number],
    betaM: sky.betaM / k,
    heightR: sky.heightR * k,
    heightM: sky.heightM * k,
  };
}

/** Linear colour of a world's clouds: white water, pale yellow sulfuric acid, ochre dust. */
export function cloudTint(meta: WorldMetadata): [number, number, number] {
  switch (meta.look.atmosphere?.clouds) {
    case 'sulfuric':
      return [0.9, 0.82, 0.55];
    case 'dust':
      return [0.62, 0.38, 0.2];
    default:
      return [0.95, 0.95, 0.95];
  }
}

/**
 * Latitude under a world's star, degrees, at `season` (fraction of the
 * year from the spring equinox) for an axial tilt in degrees, on a circular
 * orbit: `sin δ = sin ε · sin λ`, λ = 2π · season the planet's place on its
 * orbit (Nadeau & McGehee 2021; the generator's `climate::declination`).
 * A world in a system takes the sun's direction from the orbit instead.
 */
export function sunDeclinationDeg(axialTiltDeg: number, season: number): number {
  const tilt = (axialTiltDeg * Math.PI) / 180;
  return (Math.asin(Math.sin(tilt) * Math.sin(2 * Math.PI * season)) * 180) / Math.PI;
}

/**
 * Which of the four named dates (0 spring equinox, 1 summer solstice, 2
 * autumn equinox, 3 winter solstice) the sun's latitude shows, or null when
 * it is between them. The slider position only says the date is near: the
 * name is kept when the sun is truly there, over the tilt at a solstice
 * (within `toleranceDeg`) and over the equator at an equinox, so a solstice
 * label never states 22.8° on a world tilted 23.4°.
 */
export function namedSeason(season: number, declinationDeg: number, axialTiltDeg: number, toleranceDeg = 0.1): number | null {
  const quarter = season * 4;
  const nearest = Math.round(quarter);
  if (Math.abs(quarter - nearest) >= 0.12) return null;
  const expected = nearest % 2 === 0 ? 0 : Math.abs(axialTiltDeg);
  return Math.abs(Math.abs(declinationDeg) - expected) <= toleranceDeg ? ((nearest % 4) + 4) % 4 : null;
}
