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
  /**
   * Share of the aerosol extinction that scatters, per channel (its tint);
   * over 1 for a thick haze, whose light scatters many times (see hazeAlbedo).
   */
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
/** Optical depth at the zenith per unit of haze density, up to {@link THICK_HAZE}. */
const HAZE_DEPTH = 2.0;
/**
 * Haze density past which a haze thickens towards an opaque one: every
 * built-in look but a methane haze stays under it (Earth 0.1, Venus 0.3,
 * a volcanic world 0.45), so their skies keep the depth of
 * {@link HAZE_DEPTH} per unit of density.
 */
const THICK_HAZE = 0.5;
/**
 * Zenith optical depth of a haze of density 1, reached linearly from
 * {@link THICK_HAZE}. Calibration, target Titan's Cassini true-colour
 * images, where the haze hides the surface in visible light: its vertical
 * optical depth is several in the visible (Tomasko et al. 2005,
 * doi:10.1038/nature04126, Huygens DISR); a methane haze (0.9, the
 * generator's `METHANE_HAZE`) gets about 6.6, its ground seen through
 * about 0.1 % of the light.
 */
const OPAQUE_HAZE_DEPTH = 8.0;

/**
 * Zenith optical depth of the aerosols of a haze of density `haze` (the
 * look's, in [0, 1]): {@link HAZE_DEPTH} per unit up to {@link THICK_HAZE},
 * then rising to {@link OPAQUE_HAZE_DEPTH} at 1.
 */
export function hazeDepth(haze: number): number {
  if (haze <= THICK_HAZE) return HAZE_DEPTH * haze;
  const t = Math.min((haze - THICK_HAZE) / (1 - THICK_HAZE), 1);
  return HAZE_DEPTH * THICK_HAZE + (OPAQUE_HAZE_DEPTH - HAZE_DEPTH * THICK_HAZE) * t;
}

/**
 * Share of the light a thick haze scatters that goes on to scatter again
 * rather than leave, at a haze of density 1 (see {@link hazeAlbedo}).
 * Calibration, target Titan's Cassini true-colour images: a bright orange
 * disc, which single scattering alone leaves a dull brown.
 */
const RESCATTER = 0.55;

/**
 * Per-channel share of the aerosol extinction that the viewer's sky
 * returns, for a haze of density `haze` and colour `colour` (the share
 * that one scattering keeps): the colour itself up to {@link THICK_HAZE};
 * beyond, the light scattered many times, `a / (1 − w a)` for a share `w`
 * that scatters again, rising to {@link RESCATTER} at density 1. The
 * series favours the channels the haze keeps best, so a thick haze is
 * both brighter and more saturated than one scattering makes it.
 */
export function hazeAlbedo(haze: number, colour: [number, number, number]): [number, number, number] {
  if (haze <= THICK_HAZE) return colour;
  const w = RESCATTER * Math.min((haze - THICK_HAZE) / (1 - THICK_HAZE), 1);
  return colour.map((a) => a / (1 - w * a)) as [number, number, number];
}

/**
 * Anisotropy of the aerosols of a haze of density `haze`: 0.76 for a thin
 * haze, lower as it thickens to 0.4 (thick hazes scatter many times, which
 * spreads their light almost evenly), then falling to 0 (isotropic) past
 * {@link THICK_HAZE}. A single-scattering sky cannot otherwise brighten
 * an opaque haze seen from the sunny side: its many scatterings return
 * light about evenly, which an isotropic phase stands in for.
 * Calibration, target Titan's Cassini true-colour images (an evenly lit
 * orange disc).
 */
function hazeAnisotropy(haze: number): number {
  const g = 0.76 - 0.5 * Math.min(haze / 0.4, 1);
  if (haze <= THICK_HAZE) return g;
  return g * (1 - Math.min((haze - THICK_HAZE) / (1 - THICK_HAZE), 1));
}

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
    betaM: hazeDepth(atmosphere.haze) / heightM,
    mieAlbedo: hazeAlbedo(atmosphere.haze, atmosphere.haze_colour),
    heightR,
    heightM,
    // Thick hazes scatter many times, which spreads their light almost
    // evenly: a low anisotropy keeps them visible from the sunny side too.
    g: hazeAnisotropy(atmosphere.haze),
  };
}

/**
 * Zenith haze depth over which a haze hides the ground from afar, and the
 * one the ground is seen through close in (see {@link hazeVeil}).
 * Calibration, no source: about 5 % and 78 % of the light.
 */
const HIDING_HAZE_DEPTH = 3.0;
const CLOSE_HAZE_DEPTH = 0.25;
/**
 * Camera altitudes (globe radii above the ground) between which the haze
 * over the ground thins (see {@link hazeVeil}): whole from a whole-disc
 * view (the default view is 2.2 radii up) to {@link HAZE_THIN_ALTITUDE},
 * thinnest at {@link HAZE_CLEAR_ALTITUDE} and below, on a logarithmic
 * scale. Calibration, no source.
 */
const HAZE_THIN_ALTITUDE = 0.6;
const HAZE_CLEAR_ALTITUDE = 0.05;

/**
 * Factor on the haze extinction over the ground (the surface pass of the
 * sky shader, not the limb) at a camera `altitude` in globe radii, so that
 * the surface of a world whose haze hides it (a zenith depth over
 * {@link HIDING_HAZE_DEPTH}, a methane haze) can be seen close in: 1
 * from afar, where the disc is the haze's, down to the factor that leaves
 * {@link CLOSE_HAZE_DEPTH} close in. 1 at every altitude for any thinner
 * haze (Earth, Mars, Venus). A viewer convention, as Cassini's
 * near-infrared windows (ISS at 938 nm, VIMS at 5 µm) saw Titan's surface
 * through a haze that hides it in visible light (Porco et al. 2005,
 * doi:10.1038/nature03436).
 */
export function hazeVeil(sky: Sky, altitude: number): number {
  const depth = sky.betaM * sky.heightM;
  if (depth <= HIDING_HAZE_DEPTH) return 1;
  const t = Math.log(Math.max(altitude, HAZE_CLEAR_ALTITUDE) / HAZE_CLEAR_ALTITUDE) / Math.log(HAZE_THIN_ALTITUDE / HAZE_CLEAR_ALTITUDE);
  const close = CLOSE_HAZE_DEPTH / depth;
  return close + (1 - close) * Math.min(t, 1);
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
