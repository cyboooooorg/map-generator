// The bodies of the sky as the planet view draws them (see systemSky.ts,
// which places those of a planetary system): their direction, size, map or
// preview, axes, air and display switches, and the shadows they cast on
// the body shown (eclipses).
//
// Directions are in the frame of the body shown, as the viewer's sphere is
// (`[cos lat sin lon, sin lat, cos lat cos lon]`, y along the spin axis);
// positions and radii are in radii of that body.
import type { Preview } from './api/types/Preview';
import type { RingSystem } from './api/types/RingSystem';
import type { WorldMetadata } from './api/types/WorldMetadata';
import { cloudTint, skyFromWorld, type Sky } from './sky';

export type Vec3 = [number, number, number];

/** A body's own x, y (spin axis) and z axes, in another body's frame. */
export type Axes = [Vec3, Vec3, Vec3];

/** The axes of the frame itself. */
export const IDENTITY_AXES: Axes = [
  [1, 0, 0],
  [0, 1, 0],
  [0, 0, 1],
];



/** The seed of a giant's ringlets, from its mass (Earth masses): its own view and its moons' skies draw the same ones. */
export function ringSeed(massEarth: number): number {
  return (massEarth * 7.31) % 13;
}

/** A body drawn in the sky of the body shown. */
export interface SkyBody {
  direction: Vec3;
  /** Apparent diameter from the centre of the body shown, degrees. */
  angularDiameterDeg: number;
  /**
   * Its centre and radius, in radii of the body shown, when it is near
   * enough for the view to place it from the camera rather than at
   * infinity from the centre (where it would slip off its orbit line as
   * the camera turns round the globe).
   */
  position?: Vec3;
  radius?: number;
  /** Colour of its surface, linear RGB, when it has no map. */
  colour: Vec3;
  /** Its surface map (`surface.png`), or null for a plain colour. */
  texture: string | null;
  /**
   * Its own axes (its x, its spin axis, its z: its map's frame) in the
   * frame of the body shown, as it turns on itself; null for the frame of
   * the body shown.
   */
  axes: Axes | null;
  /** Its cloud map (`clouds.png`, grey), or null for none. */
  clouds?: string | null;
  /** Linear colour of its clouds. */
  cloudTint?: Vec3;
  /** Its atmosphere, seen from afar: a halo at the limb (see `halo`). */
  halo?: Halo | null;
  /** Axial tilt, degrees, for its tropics and polar circles. */
  axialTiltDeg?: number;
  /** What is drawn of it: all of it when absent; its grid and tropics only when asked. */
  show?: SkyBodyShow;
  /** How it is drawn while it has no map (a body of a system not generated yet). */
  preview?: Preview;
  /**
   * A giant's rings, in its equatorial plane (edges and gaps in its
   * radii), with the shadow they cast on it; none for a body without.
   */
  rings?: RingSystem | null;
  /** The seed of its rings' ringlets (`ringSeed`), so that they look in the sky as in its own view. */
  ringSeed?: number;
  /**
   * Cosine of its phase angle, the angle at it between the star and the body
   * shown (1 full, -1 new); the sun's direction is used when absent.
   */
  phaseCos?: number;
  /** The star's flux at it over its flux at the body shown (1 when absent). */
  insolation?: number;
}

/** How the planet looks in a moon's sky: its clouds, its air, its marks and their switches. */
export type PlanetLook = Pick<SkyBody, 'clouds' | 'cloudTint' | 'halo' | 'axialTiltDeg' | 'show'>;

/**
 * The planet's look in a moon's sky, from the planet's own `world.json`:
 * its air (the halo of its sky, as its own view scatters it), and its
 * clouds (the layer its view draws, `file` resolved by `url`) in its
 * cloud colour. A world without a sky or a cloud layer (an airless rock)
 * gets none of them, and a giant, whose surface is its clouds, neither.
 */
export function planetLook(meta: WorldMetadata, url: (file: string) => string): PlanetLook {
  const file = meta.layers.clouds?.file;
  return {
    // A giant has no sky to scatter (its pressure is nominal): the halo's
    // limb mix and darkening would wash its bands out.
    halo: meta.giant ? null : halo(skyFromWorld(meta)),
    clouds: file ? url(file) : null,
    cloudTint: cloudTint(meta),
  };
}

/** The display switches of a body in the sky (its own, not the body shown's). */
export interface SkyBodyShow {
  atmosphere: boolean;
  clouds: boolean;
  /** Its clouds drift (they stand still otherwise). */
  drift: boolean;
  grid: boolean;
  tropics: boolean;
  /** Its rings, when it has some. */
  rings: boolean;
  /** Its spin axis, out of its poles. */
  axis: boolean;
  /** Its spin arrow, or its ring when it is locked to what it orbits. */
  spin: boolean;
}

/** An atmosphere seen from afar. */
export interface Halo {
  /** Linear colour of the light it scatters, brightest channel 1. */
  colour: Vec3;
  /** Height of the halo, in radii of the body. */
  thickness: number;
  /** Brightness of the halo at the limb, 0 to 1. */
  strength: number;
}

/**
 * The halo of an atmosphere (see sky.ts), null for an airless body: the
 * colour of the light its gas and aerosols scatter (their columns), and a
 * brightness growing with their optical depth.
 */
export function halo(sky: Sky | null): Halo | null {
  if (!sky) return null;
  const column = sky.betaR.map((b, i) => b * sky.heightR + sky.betaM * sky.heightM * sky.mieAlbedo[i]!) as Vec3;
  const brightest = Math.max(...column, 1e-9);
  const depth = (column[0] + column[1] + column[2]) / 3;
  return {
    colour: column.map((c) => c / brightest) as Vec3,
    thickness: sky.top - 1,
    strength: Math.min(1, 0.9 * (1 - Math.exp(-4 * depth))),
  };
}

/** A body whose shadow can fall on the body shown (an eclipse). */
export interface ShadowCaster {
  /** Its centre, in radii of the body shown. */
  position: Vec3;
  /** Its radius, in radii of the body shown. */
  radius: number;
}

/** Most shadow casters the surface shader takes. */
export const MAX_CASTERS = 4;

/**
 * Whether a body of the sky at `position` (its centre, `radius`, in radii
 * of the body shown, the unit sphere at the origin) is nearer the camera
 * at `camera` than the part of the body shown it covers: then it is drawn
 * in front of it. Along its centre's line of sight when that line meets
 * the body shown, else against the horizon's distance (the farthest any
 * visible point of the body shown can be).
 */
export function inFrontOfGlobe(camera: Vec3, position: Vec3, radius: number): boolean {
  const to: Vec3 = [position[0] - camera[0], position[1] - camera[1], position[2] - camera[2]];
  const distance = Math.hypot(...to);
  if (distance < 1e-12) return true;
  const c2 = camera[0] ** 2 + camera[1] ** 2 + camera[2] ** 2;
  // The camera inside the body shown sees nothing of it to cover.
  if (c2 <= 1) return false;
  const b = (camera[0] * to[0] + camera[1] * to[1] + camera[2] * to[2]) / distance;
  const disc = b * b - (c2 - 1);
  if (disc > 0) {
    const hit = -b - Math.sqrt(disc);
    if (hit > 0) return distance - radius < hit;
  }
  return distance < Math.sqrt(c2 - 1);
}

/**
 * The sky bodies the orbit lines are tested against (see skyOrbitFragment
 * in lib/viewer/shaders.ts), at most `most`: those with a place, drawn at
 * their size (the tangent of their angular radius from the camera at
 * `camera` at least `least`; a smaller one is drawn as a marker, which
 * the lines pass under as before), the largest from the camera first.
 */
export function skyOccluders(
  camera: Vec3,
  bodies: readonly Pick<SkyBody, 'position' | 'radius'>[],
  least: number,
  most: number,
): ShadowCaster[] {
  const found: { position: Vec3; radius: number; half: number }[] = [];
  for (const { position, radius } of bodies) {
    if (!position || radius === undefined) continue;
    const distance = Math.hypot(position[0] - camera[0], position[1] - camera[1], position[2] - camera[2]);
    const half = Math.asin(Math.min(radius / Math.max(distance, 1e-12), 0.999));
    if (Math.tan(half) >= least) found.push({ position, radius, half });
  }
  return found
    .sort((p, q) => q.half - p.half)
    .slice(0, most)
    .map(({ position, radius }) => ({ position, radius }));
}

/**
 * The shadow casters the surface shader gets, at most MAX_CASTERS of them:
 * those whose shadow (its penumbra included) reaches the body shown (the
 * unit sphere) now, the sun in direction `sun` of angular radius
 * `sunRadius` (radians), the largest in its sky first. A small moon close
 * by then wins over a far giant whose shadow cannot reach.
 */
export function pickShadowCasters(casters: readonly ShadowCaster[], sun: Vec3, sunRadius: number): ShadowCaster[] {
  return casters
    .filter(({ position, radius }) => {
      // Along the sunward axis through the body shown, and off it.
      const t = position[0] * sun[0] + position[1] * sun[1] + position[2] * sun[2];
      if (t <= -1) return false;
      const off = Math.hypot(position[0] - t * sun[0], position[1] - t * sun[1], position[2] - t * sun[2]);
      return off < 1 + radius + Math.max(t + 1, 0) * Math.tan(sunRadius);
    })
    .sort((p, q) => q.radius / Math.hypot(...q.position) - p.radius / Math.hypot(...p.position))
    .slice(0, MAX_CASTERS);
}

/**
 * Share of the sunlight a shadow caster hides at a point of the unit
 * sphere (0 to 1), the sun of angular radius `sunRadius` (radians) in
 * direction `sun`: as the surface shader computes it (umbra inside, a
 * penumbra growing with the distance behind the caster).
 */
export function eclipse(point: Vec3, sun: Vec3, caster: ShadowCaster, sunRadius: number): number {
  const c: Vec3 = [caster.position[0] - point[0], caster.position[1] - point[1], caster.position[2] - point[2]];
  const t = c[0] * sun[0] + c[1] * sun[1] + c[2] * sun[2];
  if (t <= 0) return 0;
  const d = Math.hypot(c[0] - t * sun[0], c[1] - t * sun[1], c[2] - t * sun[2]);
  const spread = t * Math.tan(sunRadius);
  const umbra = Math.max(caster.radius - spread, 0);
  const penumbra = caster.radius + spread;
  // Smoothstep from the penumbra's edge to the umbra's (or the centre).
  const x = Math.min(Math.max((penumbra - d) / Math.max(penumbra - umbra, 1e-6), 0), 1);
  const covered = x * x * (3 - 2 * x);
  // An annular eclipse never hides the whole sun.
  const most = caster.radius >= spread ? 1 : (caster.radius / spread) ** 2;
  return covered * most;
}
