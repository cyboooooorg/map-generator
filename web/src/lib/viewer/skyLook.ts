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
  /** Angular size of the galactic core (its along-band extent), radians; 0.3 at the Sun's place. */
  coreSize: number;
  /** Brightness of the galactic core relative to the Sun's place, `(8.2 / R)²`; 0 for the fixed sky. */
  coreBrightness: number;
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

/** Largest core brightness and size: very close to the centre the law would blow up. */
const CORE_BRIGHTNESS_MAX = 4;
const CORE_SIZE_MAX = 0.6;
/** The core's angular size at the Sun's place, radians. */
const SUN_CORE_SIZE = 0.3;

/** Today's sky: seed 7, the band tilted 60 degrees about x, no bulge. */
export const FIXED_SKY: SkyLook = {
  seed: 7,
  brightness: 1,
  bandWidth: SUN_BAND_WIDTH,
  bulge: 0,
  north: [0, Math.cos(FIXED_TILT), Math.sin(FIXED_TILT)],
  centre: [1, 0, 0],
  coreSize: SUN_CORE_SIZE,
  coreBrightness: 0,
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
  // Distance to the galactic centre, kpc; the core grows and brightens as it shrinks.
  const toCentre = Math.max(0.5, Math.hypot(x, y, z));
  const closeness = SUN_RADIUS / toCentre;
  return {
    seed: place.sky_seed,
    brightness,
    bandWidth,
    bulge,
    north: normalise(place.axes[2]),
    centre: normalise(toSystem(place.axes, normalise([-x, -y, -z]))),
    coreSize: Math.min(CORE_SIZE_MAX, SUN_CORE_SIZE * Math.sqrt(closeness)),
    coreBrightness: Math.min(CORE_BRIGHTNESS_MAX, closeness * closeness),
  };
}

const dot3 = (a: Vec3, b: Vec3) => a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
const cross3 = (a: Vec3, b: Vec3): Vec3 => [
  a[1] * b[2] - a[2] * b[1],
  a[2] * b[0] - a[0] * b[2],
  a[0] * b[1] - a[1] * b[0],
];

/**
 * The band's frame in the system frame: `along` (toward the centre, in the
 * plane), `across` (in the plane, 90 degrees on) and `north`.
 */
export function bandFrame(look: SkyLook): { along: Vec3; across: Vec3; north: Vec3 } {
  const n = look.north;
  const c = look.centre;
  const d = dot3(c, n);
  const flat: Vec3 = [c[0] - n[0] * d, c[1] - n[1] * d, c[2] - n[2] * d];
  const along = Math.hypot(...flat) > 1e-6 ? normalise(flat) : normalise(cross3(n, Math.abs(n[0]) < 0.9 ? [1, 0, 0] : [0, 1, 0]));
  return { along, across: cross3(n, along), north: n };
}
