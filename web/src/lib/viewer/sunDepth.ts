// Optical depth towards the sun through the atmosphere shell, tabulated for
// the sky shader (see atmosphereFragment in shaders.ts).
//
// Each scattering species is an exponential atmosphere over the unit
// sphere, density exp(-(r - 1) / H). Its column along a ray rising at cos mu
// from radius r is the Chapman function, here in its parabolic
// approximation: H rho(r) sqrt(pi x / 2) erfcx(mu sqrt(x / 2)), x = r / H,
// with erfcx(y) ~ 2 / (sqrt(pi) (y + sqrt(y^2 + k(y)))), k blending from
// 4 / pi at the horizon to 2 overhead (within 1 % of erfcx). A ray below the
// horizon that misses the ground crosses the whole atmosphere around its
// tangent point, less the part behind the start. The column beyond the top
// of the shell is subtracted, as the shader's shell ends there. Within 2 %
// of a numerical integration for every sky of sky.ts.
//
// The table stores the column divided by the density at the start, which
// varies slowly, so the shader multiplies it back by the density it has
// computed anyway. Columns: cos of the sun's zenith angle, from the grazing
// ray (tangent to the ground) to the zenith; rows: height from the ground to
// the top of the shell. Both are on a square-root scale, which puts the
// texels where the column changes fastest: near the horizon, and near the
// ground where the grazing angle moves quickly.
import { DataUtils } from 'three';

/** Texels of the table: cos of the sun's zenith angle by height. */
export const SUN_DEPTH_WIDTH = 128;
export const SUN_DEPTH_HEIGHT = 32;

/** Column to infinity divided by the density at r, for a ray rising at cos mu >= 0. */
function chapman(r: number, mu: number, H: number): number {
  const x = r / H;
  const y = mu * Math.sqrt(0.5 * x);
  const k = 4 / Math.PI + ((2 - 4 / Math.PI) * y) / (y + 1);
  return (H * Math.sqrt(2 * x)) / (y + Math.sqrt(y * y + k));
}

/**
 * Column of exp(-(r - 1) / H) from radius `r` along a ray at cos `mu` with
 * the vertical, up to the sphere of radius `top`, divided by the density at
 * `r`. The ray must not hit the ground (r^2 (1 - mu^2) >= 1 when mu < 0).
 */
export function sunColumnFactor(r: number, mu: number, H: number, top: number): number {
  const sin2 = Math.max(1 - mu * mu, 0);
  let column = chapman(r, Math.abs(mu), H);
  if (mu < 0) {
    // Around the tangent point, relative to the density at r.
    const tangent = r * Math.sqrt(sin2);
    column = 2 * Math.sqrt(0.5 * Math.PI * tangent * H) * Math.exp((r - tangent) / H) - column;
  }
  const muTop = Math.sqrt(Math.max(1 - (r * r * sin2) / (top * top), 0));
  const beyond = Math.exp((r - top) / H) * chapman(top, muTop, H);
  return Math.max(column - beyond, 0);
}

/** Cos of the zenith angle of the ray from radius r that grazes the ground. */
export function grazingMu(r: number): number {
  return -Math.sqrt(Math.max(1 - 1 / (r * r), 0));
}

/** The radius and cos of the zenith angle at texture coordinates (u, v) in [0, 1]. */
export function sunDepthAt(u: number, v: number, top: number): { r: number; mu: number } {
  const r = 1 + v * v * (top - 1);
  const muH = grazingMu(r);
  return { r, mu: muH + u * u * (1 - muH) };
}

/**
 * The table for a shell of radius `top` and the scale heights of the gas and
 * the aerosols: RG half floats, row-major, a row per height.
 */
export function sunDepthTable(top: number, heightR: number, heightM: number): Uint16Array {
  const table = new Uint16Array(SUN_DEPTH_WIDTH * SUN_DEPTH_HEIGHT * 2);
  for (let j = 0; j < SUN_DEPTH_HEIGHT; j += 1) {
    for (let i = 0; i < SUN_DEPTH_WIDTH; i += 1) {
      const { r, mu } = sunDepthAt(i / (SUN_DEPTH_WIDTH - 1), j / (SUN_DEPTH_HEIGHT - 1), top);
      const at = (j * SUN_DEPTH_WIDTH + i) * 2;
      table[at] = DataUtils.toHalfFloat(sunColumnFactor(r, mu, heightR, top));
      table[at + 1] = DataUtils.toHalfFloat(sunColumnFactor(r, mu, heightM, top));
    }
  }
  return table;
}
