import type { Vec3 } from '../geo';

/**
 * Henyey-Greenstein asymmetry of micron-sized ice grains: strongly forward
 * scattering. Saturn's E ring and Enceladus's plume brighten by tens of
 * times from 90° to high phase (Hedman et al. 2012, Icarus 217, "The
 * three-dimensional structure of Saturn's E ring"; Porco et al. 2006).
 */
export const DUST_G = 0.7;

/**
 * Scale height of a dust ring, as a fraction of the radius: its density
 * falls as a Gaussian of the height over the ring plane with this sigma
 * (look-level value; Hedman et al. 2012, Icarus 217, "The three-dimensional
 * structure of Saturn's E ring", find the E ring a few thousand km thick
 * near Enceladus's orbit, thickening outward).
 */
export const DUST_SCALE_HEIGHT = 0.02;

/** Samples along a view ray through a dust ring (sky shader and `dustColumn`). */
export const DUST_MARCH_STEPS = 24;

/** Cap on the gain over the 90° brightness, keeping the exact back-lit direction finite. */
export const DUST_GAIN_MAX = 80;

/**
 * Floor under the dust rings' and halo's phase gain (look calibration): a
 * dust ring is never dimmer than at 90° of scattering, so it stays seen
 * front-lit, where the true phase function would all but hide it.
 */
export const DUST_RING_FLOOR = 1;

/**
 * Cap on the dust rings' and halo's phase gain (look calibration): back-lit
 * they shine at most this many times their 90° light, not the true tens of
 * times (`DUST_GAIN_MAX`), which washed half the frame of a giant's
 * close-up. The plume and the jets keep the full gain.
 */
export const DUST_RING_GAIN_MAX = 4;

/**
 * Look boost of a dust ring's opacity in the giant's close-up (look
 * calibration): seen through face-on, a true E ring (face-on optical depth
 * of a few percent here, far less in nature) is invisible but back-lit;
 * boosted so that it reads as a faint grained ring at 90° of scattering,
 * and with `DUST_RING_GAIN_MAX` a few times brighter back-lit, not washing
 * the frame. Not in a moon's sky, where the long column along the ring
 * plane shows it already (a boost there veils the whole sky).
 */
export const DUST_RING_BOOST = 6;

function hg(cosTheta: number, g: number): number {
  const d = 1 + g * g - 2 * g * cosTheta;
  return (1 - g * g) / (4 * Math.PI * Math.pow(d, 1.5));
}

/**
 * Brightness of dust relative to its brightness at 90° of scattering.
 * `cosScatter` is the cosine of the scattering angle (1 = light passing
 * straight on to the eye, back-lit). Capped at `DUST_GAIN_MAX`.
 */
export function dustGain(cosScatter: number, g = DUST_G): number {
  return Math.min(hg(cosScatter, g) / hg(0, g), DUST_GAIN_MAX);
}

/**
 * The dust rings' and halo's gain: `dustGain` held between
 * `DUST_RING_FLOOR` and `DUST_RING_GAIN_MAX`, the same in the giant's
 * close-up and in its moons' sky.
 */
export function dustRingGain(cosScatter: number): number {
  return Math.min(Math.max(dustGain(cosScatter), DUST_RING_FLOOR), DUST_RING_GAIN_MAX);
}

/**
 * Cosine of the scattering angle: `sunDir` points from the dust to the sun,
 * `toEye` from the dust to the eye, both unit vectors.
 */
export function cosScatter(sunDir: Vec3, toEye: Vec3): number {
  return -(sunDir[0] * toEye[0] + sunDir[1] * toEye[1] + sunDir[2] * toEye[2]);
}

/** GLSL twins of `dustGain` and `dustRingGain`, with the same constants, for the dust shaders. */
export const dustPhaseGlsl = /* glsl */ `
  float dustGain(float cosScatter) {
    const float g = ${DUST_G.toFixed(2)};
    const float gainMax = ${DUST_GAIN_MAX.toFixed(1)};
    float d = 1.0 + g * g - 2.0 * g * cosScatter;
    float d0 = 1.0 + g * g;
    return min(pow(d0 / d, 1.5), gainMax);
  }
  const float DUST_RING_BOOST = ${DUST_RING_BOOST.toFixed(1)};
  float dustRingGain(float cosScatter) {
    return clamp(dustGain(cosScatter), ${DUST_RING_FLOOR.toFixed(1)}, ${DUST_RING_GAIN_MAX.toFixed(1)});
  }
`;
