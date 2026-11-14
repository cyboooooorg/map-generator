// How a system's sky looks, derived from its place in the galaxy.
//
// All directions are in the system frame (the star's equatorial frame of
// `lib/systemSky.ts`). The laws are first estimates (tuned by eye later):
// the disc brightens inwards like an exponential disc, the bulge grows as
// the radius shrinks, and away from the plane the band narrows.

import type { GalacticPlace } from '../api/types/GalacticPlace';
import type { Vec3 } from '../skyBodies';

export interface SkyLook {
  /** Seed of the clouds and stars: the block's `sky_seed`, or 7 for the fixed sky. */
  seed: number;
  /** Band surface brightness and star density relative to the Sun's place; 1 for the fixed sky. */
  brightness: number;
  /** Spread of the band around its great circle, radians (standard deviation). */
  bandWidth: number;
  /** Strength of the bulge toward the galactic centre, 0 to 1. */
  bulge: number;
  /** Galactic north pole, unit, system frame. */
  north: Vec3;
  /** Direction to the galactic centre, unit, system frame. */
  centre: Vec3;
}

/** The Sun's galactocentric radius, kpc. */
const SUN_RADIUS = 8.2;
/** The Sun's height above the plane, kpc. */
const SUN_HEIGHT = 0.02;
/** Scale length of the exponential disc, kpc. */
const DISC_SCALE = 2.6;
/** Radial scale of the bulge's strength, kpc. */
const BULGE_SCALE = 3.5;
/** Height above the Sun's, kpc, at which the band is half as wide. */
const NARROW_HEIGHT = 0.6;
/** The band's spread at the Sun's place, radians. */
const SUN_BAND_WIDTH = (7 * Math.PI) / 180;
/** Angle between today's fixed band and the planet's equator, radians. */
const FIXED_TILT = (60 * Math.PI) / 180;

/** Today's sky: seed 7, the band tilted 60 degrees about x, no bulge. */
export const FIXED_SKY: SkyLook = {
  seed: 7,
  brightness: 1,
  bandWidth: SUN_BAND_WIDTH,
  bulge: 0,
  north: [0, Math.cos(FIXED_TILT), Math.sin(FIXED_TILT)],
  centre: [1, 0, 0],
};

function normalise([x, y, z]: Vec3): Vec3 {
  const length = Math.hypot(x, y, z);
  return length > 0 ? [x / length, y / length, z / length] : [1, 0, 0];
}

/** A galactic vector in the system frame. */
function toSystem(axes: GalacticPlace['axes'], [x, y, z]: Vec3): Vec3 {
  return [
    x * axes[0][0] + y * axes[1][0] + z * axes[2][0],
    x * axes[0][1] + y * axes[1][1] + z * axes[2][1],
    x * axes[0][2] + y * axes[1][2] + z * axes[2][2],
  ];
}

/** The sky of a galactic place; the fixed sky when there is none. */
export function skyLook(place: GalacticPlace | null): SkyLook {
  if (!place) return FIXED_SKY;
  const [x, y, z] = place.position_kpc;
  const radius = Math.hypot(x, y);
  const brightness = Math.exp(-(radius - SUN_RADIUS) / DISC_SCALE);
  const bulge = Math.min(1, Math.max(0, Math.exp(-radius / BULGE_SCALE)));
  const above = Math.max(0, Math.abs(z) - SUN_HEIGHT);
  const bandWidth = SUN_BAND_WIDTH / (1 + above / NARROW_HEIGHT);
  return {
    seed: place.sky_seed,
    brightness,
    bandWidth,
    bulge,
    north: normalise(place.axes[2]),
    centre: normalise(toSystem(place.axes, normalise([-x, -y, -z]))),
  };
}
