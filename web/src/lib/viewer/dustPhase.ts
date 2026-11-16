import type { Vec3 } from '../geo';

/**
 * Henyey-Greenstein asymmetry of micron-sized ice grains: strongly forward
 * scattering. Saturn's E ring and Enceladus's plume brighten by tens of
 * times from 90° to high phase (Hedman et al. 2012, Icarus 217, "The
 * three-dimensional structure of Saturn's E ring"; Porco et al. 2006).
 */
export const DUST_G = 0.7;

/** Cap on the gain over the 90° brightness, keeping the exact back-lit direction finite. */
export const DUST_GAIN_MAX = 80;

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
 * Cosine of the scattering angle: `sunDir` points from the dust to the sun,
 * `toEye` from the dust to the eye, both unit vectors.
 */
export function cosScatter(sunDir: Vec3, toEye: Vec3): number {
  return -(sunDir[0] * toEye[0] + sunDir[1] * toEye[1] + sunDir[2] * toEye[2]);
}

/** GLSL twin of `dustGain`, with the same constants, for the dust shaders. */
export const dustPhaseGlsl = /* glsl */ `
  float dustGain(float cosScatter) {
    const float g = ${DUST_G.toFixed(2)};
    const float gainMax = ${DUST_GAIN_MAX.toFixed(1)};
    float d = 1.0 + g * g - 2.0 * g * cosScatter;
    float d0 = 1.0 + g * g;
    return min(pow(d0 / d, 1.5), gainMax);
  }
`;
